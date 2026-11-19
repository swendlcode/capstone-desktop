//! Starting Capstone, for the tools that cannot work without it.
//!
//! The queue is held in memory and the processing pool and its progress events
//! live in the window, so `get_queue`, `add_files`, `analyze_file` and
//! `start_processing` all need the app. Their refusal names this tool.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::error::{CapstoneError, Result};
use crate::mcp::McpContext;

/// A cold launch loads the templates, the settings and the window. 30s is
/// generous and still bounded.
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(30);
const LAUNCH_POLL: Duration = Duration::from_millis(250);
const PROBE_TIMEOUT: Duration = Duration::from_millis(400);

pub fn launch(ctx: &McpContext, args: &Value) -> Result<Value> {
    if ctx.live.is_some() {
        return Ok(json!({
            "running": true,
            "note": "Capstone is already running; this call went through it",
        }));
    }

    let target = app_executable()?;
    spawn(&target)?;

    if !super::bool_arg(args, "wait", true) {
        return Ok(json!({
            "running": false,
            "launched": target.to_string_lossy(),
            "note": "started without waiting; the next call may still find it coming up",
        }));
    }

    let clock = Instant::now();
    while clock.elapsed() < LAUNCH_TIMEOUT {
        if bridge_answers(&ctx.data_dir) {
            return Ok(json!({
                "running": true,
                "launched": target.to_string_lossy(),
                "readyInMs": clock.elapsed().as_millis(),
            }));
        }
        std::thread::sleep(LAUNCH_POLL);
    }

    Err(CapstoneError::Other(format!(
        "started {} but it did not answer within {}s",
        target.display(),
        LAUNCH_TIMEOUT.as_secs()
    )))
}

/// The handshake file is a hint — the app can die without removing it — so the
/// port it names is probed rather than trusted.
fn bridge_answers(data_dir: &std::path::Path) -> bool {
    let Some(handshake) = crate::bridge::read_handshake(data_dir) else {
        return false;
    };
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], handshake.port));
    std::net::TcpStream::connect_timeout(&addr, PROBE_TIMEOUT).is_ok()
}

/// macOS: the `.app` two levels above this binary, since `capstone-mcp` ships
/// inside the bundle. Elsewhere, and for a dev build, the main binary beside
/// it. Found from this executable rather than assumed to be in /Applications.
fn app_executable() -> Result<PathBuf> {
    let exe = std::env::current_exe()
        .map_err(|e| CapstoneError::Other(format!("cannot locate this executable: {e}")))?;
    let dir = exe
        .parent()
        .ok_or_else(|| CapstoneError::Other("this executable has no folder".into()))?;

    #[cfg(target_os = "macos")]
    if let Some(bundle) = dir.parent().and_then(|c| c.parent()) {
        if bundle.extension().and_then(|e| e.to_str()) == Some("app") {
            return Ok(bundle.to_path_buf());
        }
    }

    let name = if cfg!(windows) { "capstone.exe" } else { "capstone" };
    let sibling = dir.join(name);
    if sibling.is_file() {
        return Ok(sibling);
    }
    Err(CapstoneError::NotFound(format!(
        "could not find Capstone next to {}; open the app yourself",
        exe.display()
    )))
}

fn spawn(target: &std::path::Path) -> Result<()> {
    let mut command = if target.extension().and_then(|e| e.to_str()) == Some("app") {
        // `open` hands it to the window server, which is what gives a GUI app
        // a session; spawning the inner binary directly does not.
        let mut c = std::process::Command::new("/usr/bin/open");
        c.arg("-a").arg(target);
        c
    } else {
        std::process::Command::new(target)
    };

    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| CapstoneError::Other(format!("could not start {}: {e}", target.display())))?;
    Ok(())
}
