//! Waiting for a run to finish.
//!
//! Separate from the queue tools because it is the one call here that does
//! not answer immediately, and because the queue module was at the file-size
//! cap. What it waits on, and the two cases it refuses rather than waits
//! through, are in the doc comment below.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::queue::status_name;
use crate::error::{CapstoneError, Result};
use crate::mcp::McpContext;
use crate::models::FileStatus;
use crate::state::AppState;
use tauri::Manager;

/// How long `wait_for_queue` waits when the caller does not say. A batch of
/// fifty tracks on a laptop is a couple of minutes; this is generous for that
/// and still short enough that a stuck run is noticed.
const DEFAULT_WAIT_SECS: u64 = 300;
/// The ceiling. A wait longer than this is a poll with extra steps, and the
/// bridge thread it occupies is better given back.
const MAX_WAIT_SECS: u64 = 1_800;
/// How often the queue is checked. Fast enough that the answer lands within a
/// second of the run ending, cheap enough to be invisible.
const POLL: Duration = Duration::from_millis(250);

/// Block until the run finishes, then answer with how it went.
///
/// The reason this exists: `start_processing` returns immediately, because the
/// command it sits over must not hold the IPC bridge for the length of a
/// batch. That leaves an agent polling `get_queue` every few seconds — a call
/// per poll, each one a full JSON round trip, to find out something the app
/// already knows the moment it happens.
///
/// It waits on the **queue's own state** rather than on the run flag. A file
/// still `Pending` after the pool has stopped — which is what a cancel leaves
/// behind — must not read as a batch still going, and the flag alone cannot
/// tell those apart.
///
/// Two refusals rather than a silent wait: nothing to wait for, and a queue
/// with work in it that nobody started. Waiting out a five-minute timeout to
/// report "still pending" is the least useful thing this could do.
pub fn wait(ctx: &McpContext, args: &Value) -> Result<Value> {
    let live = ctx.require_live("waiting for the queue")?;
    let state = live.app.state::<AppState>();
    let timeout = Duration::from_secs(
        super::opt_u64(args, "timeoutSeconds")
            .unwrap_or(DEFAULT_WAIT_SECS)
            .clamp(1, MAX_WAIT_SECS),
    );

    if unfinished(&state) == 0 {
        return Ok(finished_answer(&state, json!("the queue had already finished")));
    }
    if !state.running.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(CapstoneError::Other(
            "there are files waiting but no run in progress: call start_processing first".into(),
        ));
    }

    let clock = Instant::now();
    while clock.elapsed() < timeout {
        if unfinished(&state) == 0 {
            return Ok(finished_answer(
                &state,
                json!(format!("finished after {:.1}s", clock.elapsed().as_secs_f64())),
            ));
        }
        std::thread::sleep(POLL);
    }

    let mut out = finished_answer(&state, json!("the wait timed out; the run is still going"));
    out["timedOut"] = json!(true);
    out["stillToGo"] = json!(unfinished(&state));
    Ok(out)
}

/// Files that are neither done nor failed.
fn unfinished(state: &AppState) -> usize {
    state
        .queue
        .lock()
        .iter()
        .filter(|f| {
            !matches!(f.status, FileStatus::Done | FileStatus::Error)
        })
        .count()
}

/// What the run produced: the counts, every failure, and where the masters
/// landed. The same shape `get_queue` answers with, plus the outputs, because
/// the next thing anybody does with a finished batch is pick the files up.
fn finished_answer(state: &AppState, note: Value) -> Value {
    let queue = state.queue.lock().clone();
    let mut counts = BTreeMap::new();
    for file in &queue {
        *counts.entry(status_name(file.status)).or_insert(0usize) += 1;
    }
    let outputs: Vec<&str> = queue
        .iter()
        .filter_map(|f| f.result.as_ref())
        .map(|r| r.output_path.as_str())
        .filter(|p| !p.is_empty())
        .collect();
    let failures: Vec<Value> = queue
        .iter()
        .filter(|f| f.status == FileStatus::Error)
        .map(|f| json!({ "fileId": f.id, "filename": f.filename, "error": f.error }))
        .collect();
    let skipped: Vec<&str> = queue
        .iter()
        .filter(|f| f.status == FileStatus::Done && f.result.is_none())
        .map(|f| f.filename.as_str())
        .collect();

    let mut out = json!({
        "total": queue.len(),
        "counts": counts,
        "written": outputs.len(),
        "outputs": outputs,
        "note": note,
    });
    if !failures.is_empty() {
        out["failed"] = json!(failures.len());
        out["failures"] = json!(failures);
    }
    if !skipped.is_empty() {
        out["skipped"] = json!(skipped.len());
        out["skippedFiles"] = json!(skipped);
    }
    out
}
