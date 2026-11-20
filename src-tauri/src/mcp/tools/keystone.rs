//! Handing finished masters to Keystone.
//!
//! The two apps are deliberately separate — Capstone masters files and knows
//! nothing about libraries, Keystone keeps a library and does not process
//! audio — but the user's workflow crosses between them on every batch: the
//! masters that just came out of here are the files that should be in there,
//! pointing at the rating, cues and analysis the originals already had.
//!
//! What this knows about Keystone is deliberately the smallest possible
//! thing: where its data directory is, and that a running Keystone leaves a
//! loopback handshake in it. Everything else goes through Keystone's own
//! agent tools — `import_tracks` and `relink_track` — so the behaviour is
//! Keystone's and stays correct when Keystone changes it.
//!
//! And when Keystone is not running, this does **not** start it. A tool that
//! launched another application as a side effect of a handoff would be doing
//! something nobody asked for; it answers with the paths and says what to
//! call instead, which is a sentence an agent can act on.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use serde_json::{json, Value};

use super::{opt_str, strings_arg};
use crate::error::{CapstoneError, Result};
use crate::mcp::McpContext;
use crate::models::FileStatus;
use crate::state::AppState;
use tauri::Manager;

/// Keystone's bundle identifier, which is also its data directory's name.
const KEYSTONE_IDENTIFIER: &str = "app.keystone.desktop";
const HANDSHAKE_FILE: &str = "mcp_bridge.json";
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);
/// Generous: an import of fifty files walks them and starts analysis before
/// it answers.
const CALL_TIMEOUT: Duration = Duration::from_secs(60);

/// Send masters to Keystone: import them, or point existing tracks at them.
///
/// With no arguments it sends everything this run wrote, which is the whole
/// point — "master these and put them in the library" should be two calls,
/// not fifty.
pub fn send(ctx: &McpContext, args: &Value) -> Result<Value> {
    let paths = match strings_arg(args, "paths") {
        empty if empty.is_empty() => written_outputs(ctx)?,
        named => named,
    };
    if paths.is_empty() {
        return Err(CapstoneError::NotFound(
            "nothing to send: no masters have been written in this session, and no paths \
were named"
                .into(),
        ));
    }

    let relink_to = opt_str(args, "relinkTrackId");
    if relink_to.is_some() && paths.len() != 1 {
        return Err(CapstoneError::Other(format!(
            "relinkTrackId points one track at one file, but {} paths were given",
            paths.len()
        )));
    }

    let Some(handshake) = handshake()? else {
        return Ok(unreachable_answer(
            paths,
            "Keystone is not running, so nothing was sent",
        ));
    };

    let (tool, call_args) = match &relink_to {
        Some(track_id) => (
            "relink_track",
            json!({ "trackId": track_id, "newPath": paths[0] }),
        ),
        // `recursive` is irrelevant for files and harmless if a folder is
        // named; Keystone decides what a folder means.
        None => ("import_tracks", json!({ "paths": paths })),
    };

    match call(&handshake, tool, &call_args) {
        Ok(result) => Ok(json!({
            "sent": paths.len(),
            "paths": paths,
            "tool": tool,
            "keystone": result,
        })),
        Err(reason) => Ok(unreachable_answer(
            paths,
            &format!("Keystone refused the call: {reason}"),
        )),
    }
}

/// The paths of the masters written in this session.
///
/// Read from the queue's own rows rather than the output folder: a folder
/// listing cannot tell this run's files from last week's, and `skipped` rows
/// have no result at all, which is exactly what should not be sent.
fn written_outputs(ctx: &McpContext) -> Result<Vec<String>> {
    let live = ctx.require_live("collecting this run's masters")?;
    let state = live.app.state::<AppState>();
    let queue = state.queue.lock().clone();
    Ok(queue
        .iter()
        .filter(|f| f.status == FileStatus::Done)
        .filter_map(|f| f.result.as_ref())
        .map(|r| r.output_path.clone())
        .filter(|p| !p.is_empty() && Path::new(p).is_file())
        .collect())
}

/// What to say when the handoff could not happen: the paths, and the call
/// that would do it. The work is not lost, and the next step is one sentence.
fn unreachable_answer(paths: Vec<String>, why: &str) -> Value {
    json!({
        "sent": 0,
        "paths": paths,
        "note": format!(
            "{why}. The masters are written and listed here — open Keystone (its own \
launch_app tool will do it) and call Keystone's import_tracks with these paths."
        ),
    })
}

/// Keystone's handshake, or `None` when it is not running.
///
/// The file is a hint and not a promise — Keystone can die without removing
/// it — so the port is probed before it is trusted, the same way Capstone's
/// own MCP binary treats Capstone's.
fn handshake() -> Result<Option<Handshake>> {
    let dir = crate::mcp::data_dir_of(KEYSTONE_IDENTIFIER)?;
    let file = dir.join(HANDSHAKE_FILE);
    let Ok(text) = std::fs::read_to_string(&file) else {
        return Ok(None);
    };
    let Ok(handshake) = serde_json::from_str::<Handshake>(&text) else {
        return Ok(None);
    };
    let addr = SocketAddr::from(([127, 0, 0, 1], handshake.port));
    if TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).is_err() {
        return Ok(None);
    }
    Ok(Some(handshake))
}

#[derive(serde::Deserialize)]
struct Handshake {
    port: u16,
    token: String,
}

/// One POST to Keystone's bridge.
///
/// Hand-rolled against `TcpStream` for the same reasons `bin/mcp.rs` is: it
/// is loopback, the peer is a known server, and nothing about TLS, redirects
/// or connection reuse applies. `Connection: close` makes end-of-stream the
/// end of the body, so no length parsing is needed.
fn call(handshake: &Handshake, tool: &str, args: &Value) -> std::result::Result<Value, String> {
    let body = serde_json::to_vec(&json!({ "tool": tool, "args": args }))
        .map_err(|e| e.to_string())?;
    let addr = SocketAddr::from(([127, 0, 0, 1], handshake.port));
    let mut stream =
        TcpStream::connect_timeout(&addr, CONNECT_TIMEOUT).map_err(|e| e.to_string())?;
    stream
        .set_read_timeout(Some(CALL_TIMEOUT))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(CALL_TIMEOUT))
        .map_err(|e| e.to_string())?;

    let request = format!(
        "POST /mcp/call HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Authorization: Bearer {token}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {len}\r\n\
         Connection: close\r\n\
         \r\n",
        port = handshake.port,
        token = handshake.token,
        len = body.len(),
    );
    stream
        .write_all(request.as_bytes())
        .and_then(|()| stream.write_all(&body))
        .and_then(|()| stream.flush())
        .map_err(|e| e.to_string())?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("Keystone's bridge sent no headers")?;
    let payload: Value =
        serde_json::from_slice(&raw[split + 4..]).map_err(|e| e.to_string())?;
    if let Some(message) = payload.get("error").and_then(Value::as_str) {
        return Err(message.to_owned());
    }
    payload
        .get("result")
        .cloned()
        .ok_or_else(|| "Keystone's bridge answered without a result".to_string())
}

/// Is Keystone reachable? Cheap, and the honest thing to call before
/// promising a handoff.
pub fn status(_ctx: &McpContext, _args: &Value) -> Result<Value> {
    let dir = crate::mcp::data_dir_of(KEYSTONE_IDENTIFIER)?;
    let running = handshake()?.is_some();
    Ok(json!({
        "running": running,
        "dataDirectory": dir.to_string_lossy(),
        "note": if running {
            "Keystone is running and will take a handoff"
        } else {
            "Keystone is not running; open it, or call its own launch_app tool"
        },
    }))
}
