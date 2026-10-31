//! Background analysis: what fills the Clips, Dynamics, Volume and Pitch
//! columns while the user is still dragging files in.
//!
//! Analysis is deliberately not part of the processing run. A dropped folder
//! should start reporting what is in it straight away, so that by the time the
//! user reaches Start Processing the table is already the thing they are
//! deciding from. The run itself measures loudness again on the post-chain
//! signal, so nothing here is a prerequisite for processing a file -- it is the
//! report, not a stage.

use std::path::PathBuf;

use rayon::prelude::*;
use tauri::{AppHandle, Manager};

use crate::audio::{analyze, decode};
use crate::error::Result;
use crate::models::{Analysis, FileStatus, QueuedFile};
use crate::state::AppState;

/// Analyses `ids` in the background and returns at once, so `add_paths` can
/// hand the new rows straight back to the table.
pub fn start(app: AppHandle, ids: Vec<String>) {
    if ids.is_empty() {
        return;
    }

    tauri::async_runtime::spawn(async move {
        // Decoding is CPU-bound from end to end, so the batch belongs on the
        // blocking pool. The join is awaited only to turn a panic into one log
        // line rather than a silently abandoned drop.
        if let Err(e) = tokio::task::spawn_blocking(move || batch(&app, &ids)).await {
            tracing::error!("analysis batch panicked: {}", e);
        }
    });
}

fn batch(app: &AppHandle, ids: &[String]) {
    let binding = app.state::<AppState>();
    let state: &AppState = binding.inner();

    // An explicit pool for the same reason the runner builds one: rayon's
    // global pool sizes itself to the machine and would ignore the Processor
    // Load setting entirely.
    //
    // A run started while this batch is in flight builds its own pool, so the
    // two can briefly overlap. That is the deliberate trade: bounding them
    // against each other would mean Start Processing appearing dead until a
    // long drop had finished reading every file.
    let threads = state.settings.read().max_cores.max(1);
    let pool = match rayon::ThreadPoolBuilder::new().num_threads(threads).build() {
        Ok(pool) => pool,
        Err(e) => {
            tracing::error!("could not build analysis pool: {}", e);
            return;
        }
    };

    pool.install(|| {
        ids.par_iter().for_each(|id| {
            // Each failure is already recorded on its own row, so one
            // unreadable file in a dropped folder does not stop the rest of
            // that folder from reporting.
            let _ = run_file(app, state, id);
        });
    });
}

/// Analyses one row and records the outcome on it.
///
/// Shared by the background batch and the `analyze_file` command, so the status
/// transitions and the error text are written in exactly one place. The row's
/// own Status cell is where a failure has to land: the user is looking at the
/// table, so an error that only reached the log would read as a file that
/// silently did nothing.
pub fn run_file(app: &AppHandle, state: &AppState, id: &str) -> Result<Analysis> {
    // Fails fast for a row the user has already removed, which is the common
    // case when a big drop is cleared a second after it lands.
    let file = state.queued_file(id)?;

    if let Some(updated) = claim(state, id) {
        super::emit_file(app, &updated);
    }

    let path = PathBuf::from(&file.path);
    let outcome = decode::decode_to_f32(&path).and_then(|audio| analyze::analyze(&audio));

    match outcome {
        Ok(analysis) => {
            if let Some(updated) = state.update_file(id, |f| {
                // Recorded whatever the row is doing now: the figures describe
                // the source file, and they stay true even if the runner
                // claimed the row while this decode was still going.
                f.analysis = Some(analysis);
                if claimable(f.status) {
                    f.status = FileStatus::Analyzed;
                }
            }) {
                super::emit_file(app, &updated);
            }
            Ok(analysis)
        }
        Err(e) => {
            let message = e.to_string();
            if let Some(updated) = state.update_file(id, |f| {
                if claimable(f.status) {
                    f.status = FileStatus::Error;
                    f.error = Some(message.clone());
                }
            }) {
                super::emit_file(app, &updated);
            }
            Err(e)
        }
    }
}

/// Which statuses analysis is allowed to move a row out of.
///
/// A row the runner has picked up is left alone. The Status cell belongs to
/// whichever job is further along, and an analysis that started earlier must
/// never pull `Processing` back to `Analyzing` or stamp `Analyzed` over a
/// finished `Done`. This is reachable in ordinary use: drop a folder, press
/// Start Processing while its files are still being read.
fn claimable(status: FileStatus) -> bool {
    matches!(
        status,
        FileStatus::Pending
            | FileStatus::Analyzing
            | FileStatus::Analyzed
            | FileStatus::Error
    )
}

/// Marks a row as being analysed, clearing any previous error so a retry does
/// not show the reason it failed last time. `None` when the row is gone or busy.
fn claim(state: &AppState, id: &str) -> Option<QueuedFile> {
    let mut queue = state.queue.lock();
    let file = queue.iter_mut().find(|f| f.id == id)?;
    if !claimable(file.status) {
        return None;
    }
    file.status = FileStatus::Analyzing;
    file.error = None;
    Some(file.clone())
}
