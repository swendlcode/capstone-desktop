//! The queue and the run.
//!
//! Every tool here needs the app. The queue is held in memory and the
//! processing pool and its progress events live in the window, so with
//! Capstone closed there is nothing to read and nothing to run — which these
//! say, rather than returning an empty queue that would read as "you have no
//! files".

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::{ok, str_arg, strings_arg};
use crate::commands::queue_commands::enqueue;
use crate::error::{CapstoneError, Result};
use crate::jobs;
use crate::mcp::McpContext;
use crate::models::FileStatus;
use crate::state::AppState;
use tauri::Manager;

/// The queue, as counts by default and as rows when asked.
///
/// Fifty files of full rows is a wall of JSON — every analysis reading, every
/// result, every path — and the question being asked is almost always "how far
/// along is it". So the default answer is the counts, and `files` is opt-in.
/// `status` narrows the rows to one state, which is how an agent asks the
/// other common question: which ones failed, and why.
pub fn get(ctx: &McpContext, args: &Value) -> Result<Value> {
    let live = ctx.require_live("reading the queue")?;
    let state = live.app.state::<AppState>();
    let queue = state.queue.lock().clone();

    let wanted = super::opt_str(args, "status");
    if let Some(name) = &wanted {
        if parse_status(name).is_none() {
            return Err(CapstoneError::Other(format!(
                "`{name}` is not a status; use pending, analyzing, analyzed, processing, done or error"
            )));
        }
    }
    let wanted = wanted.and_then(|n| parse_status(&n));

    let mut counts = BTreeMap::new();
    for file in &queue {
        *counts.entry(status_name(file.status)).or_insert(0usize) += 1;
    }

    let rows: Vec<&crate::models::QueuedFile> = queue
        .iter()
        .filter(|f| wanted.is_none_or(|w| f.status == w))
        .collect();

    let mut out = json!({
        "total": queue.len(),
        "counts": counts,
        "matching": rows.len(),
    });

    // Errors are carried whatever the caller asked for: a run that has
    // finished with three failures must not look finished-and-fine just
    // because the summary was the cheap call.
    let failures: Vec<Value> = queue
        .iter()
        .filter(|f| f.status == FileStatus::Error)
        .map(|f| json!({ "fileId": f.id, "filename": f.filename, "error": f.error }))
        .collect();
    if !failures.is_empty() {
        out["failures"] = json!(failures);
    }

    if super::bool_arg(args, "files", false) || wanted.is_some() {
        out["files"] = serde_json::to_value(rows)?;
    } else if !queue.is_empty() {
        out["note"] = json!("counts only; pass files true, or a status, for the rows");
    }
    Ok(out)
}

fn status_name(status: FileStatus) -> &'static str {
    match status {
        FileStatus::Pending => "pending",
        FileStatus::Analyzing => "analyzing",
        FileStatus::Analyzed => "analyzed",
        FileStatus::Processing => "processing",
        FileStatus::Done => "done",
        FileStatus::Error => "error",
    }
}

fn parse_status(name: &str) -> Option<FileStatus> {
    match name.trim().to_ascii_lowercase().as_str() {
        "pending" => Some(FileStatus::Pending),
        "analyzing" => Some(FileStatus::Analyzing),
        "analyzed" => Some(FileStatus::Analyzed),
        "processing" => Some(FileStatus::Processing),
        "done" => Some(FileStatus::Done),
        "error" => Some(FileStatus::Error),
        _ => None,
    }
}

/// Queue files or folders, and start reading them.
///
/// Folders are walked, non-audio is ignored and a path already queued is
/// skipped, all exactly as a drop onto the window does.
pub fn add(ctx: &McpContext, args: &Value) -> Result<Value> {
    let paths = strings_arg(args, "paths");
    if paths.is_empty() {
        return Err(CapstoneError::Other(
            "`paths` is the files or folders to queue".into(),
        ));
    }
    let live = ctx.require_live("queueing files")?;
    let state = live.app.state::<AppState>();

    let added = enqueue(&state.queue, &paths);
    // Analysis fills the Clips, Dynamics, Volume and Pitch columns. Started
    // here for the same reason the command starts it: a queued row with no
    // reading is a row nothing can decide anything about.
    jobs::analysis::start(
        live.app.clone(),
        added.iter().map(|f| f.id.clone()).collect(),
    );

    Ok(json!({
        "added": added.len(),
        "files": serde_json::to_value(added)?,
        "note": "analysis has started; the readings arrive over the next few seconds",
    }))
}

pub fn remove(ctx: &McpContext, args: &Value) -> Result<Value> {
    let id = str_arg(args, "fileId")?;
    let live = ctx.require_live("changing the queue")?;
    let state = live.app.state::<AppState>();
    let before = state.queue.lock().len();
    state.queue.lock().retain(|f| f.id != id);
    let after = state.queue.lock().len();
    if before == after {
        return Err(CapstoneError::NotFound(format!("no queued file with id {id}")));
    }
    ok()
}

pub fn clear(ctx: &McpContext) -> Result<Value> {
    let live = ctx.require_live("clearing the queue")?;
    let state = live.app.state::<AppState>();
    state.queue.lock().clear();
    ok()
}

/// Re-read one file. The retry path: a row that errored, or one whose file
/// was replaced on disk.
pub fn analyze(ctx: &McpContext, args: &Value) -> Result<Value> {
    let id = str_arg(args, "fileId")?;
    let live = ctx.require_live("analysing a file")?;
    let state = live.app.state::<AppState>();
    let analysis = jobs::analysis::run_file(&live.app, state.inner(), &id)?;
    Ok(serde_json::to_value(analysis)?)
}

/// Start the run and return at once.
///
/// Returning immediately is the command's own contract: progress arrives on
/// `capstone://queue-progress` and the finish on `capstone://queue-finished`,
/// neither of which an MCP call can wait on. So this answers with what it
/// started, and `get_queue` is how the caller finds out how it went.
pub fn start(ctx: &McpContext) -> Result<Value> {
    let live = ctx.require_live("processing")?;
    let state = live.app.state::<AppState>();

    let queued = state.queue.lock().len();
    if queued == 0 {
        return Err(CapstoneError::Other(
            "the queue is empty; add files first".into(),
        ));
    }
    let template = state.active_template();
    let output_dir = jobs::output::resolved_output_dir(&state.settings.read());

    jobs::start(live.app.clone())?;

    Ok(json!({
        "started": true,
        "files": queued,
        "template": template.name,
        "outputFolder": output_dir.to_string_lossy(),
        "note": "running in the background; call get_queue to see how each file ended up",
    }))
}

/// Raise the cancel flag. The run stops at the next file boundary, so this is
/// not a promise that nothing is being written right now.
pub fn cancel(ctx: &McpContext) -> Result<Value> {
    let live = ctx.require_live("cancelling")?;
    let state = live.app.state::<AppState>();
    jobs::cancel(state.inner());
    Ok(json!({ "ok": true, "note": "the run stops at the next file boundary" }))
}
