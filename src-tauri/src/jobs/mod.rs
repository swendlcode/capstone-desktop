//! The queue runner: one bounded rayon pool over the queued files, three
//! events out to the frontend, and a cancel flag checked at file boundaries.

pub mod analysis;
pub mod output;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use rayon::prelude::*;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::audio::encode::EncodeSpec;
use crate::error::{CapstoneError, Result};
use crate::models::{FileStatus, ProcessResult, QueuedFile, Settings, Template};
use crate::state::AppState;

pub const EVENT_FILE_UPDATED: &str = "capstone://file-updated";
pub const EVENT_QUEUE_PROGRESS: &str = "capstone://queue-progress";
pub const EVENT_QUEUE_FINISHED: &str = "capstone://queue-finished";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueProgress {
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueFinished {
    pub processed: usize,
    pub failed: usize,
}

/// Every status change goes out through here, so the table never has to poll.
pub fn emit_file(app: &AppHandle, file: &QueuedFile) {
    emit(app, EVENT_FILE_UPDATED, file.clone());
}

/// Starts a run and returns immediately, which is what the command contract
/// promises: `start_processing` must not block the IPC bridge for the length of
/// a batch.
pub fn start(app: AppHandle) -> Result<()> {
    {
        let state = app.state::<AppState>();
        // Already running. A second press is a no-op rather than an error,
        // because two pools over one queue would process some files twice and
        // race on their output paths.
        if state.running.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        state.cancel.store(false, Ordering::SeqCst);
    }

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        // The run is CPU-bound from end to end, so it belongs on the blocking
        // pool rather than on an async worker.
        match tokio::task::spawn_blocking(move || run(&handle)).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => tracing::error!("processing run failed: {}", e),
            Err(e) => tracing::error!("processing run panicked: {}", e),
        }
    });

    Ok(())
}

pub fn cancel(state: &AppState) {
    state.cancel.store(true, Ordering::SeqCst);
}

fn run(app: &AppHandle) -> Result<()> {
    let binding = app.state::<AppState>();
    let state: &AppState = binding.inner();

    let outcome = run_inner(app, state);

    // Released on every path, including a failure to build the pool. Leaving it
    // raised would make Start Processing dead for the rest of the session.
    state.running.store(false, Ordering::SeqCst);
    outcome
}

fn run_inner(app: &AppHandle, state: &AppState) -> Result<()> {
    let settings = state.settings.read().clone();
    let template = state.active_template();

    let ids = pending_ids(state);
    let total = ids.len();
    let done = AtomicUsize::new(0);
    let failed = AtomicUsize::new(0);

    // An explicit pool, never rayon's global one: the global pool sizes itself
    // to the machine and ignores max_cores, which would make the Processor Load
    // setting a decoration.
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(settings.max_cores.max(1))
        .build()
        .map_err(|e| CapstoneError::Other(format!("could not build worker pool: {}", e)))?;

    pool.install(|| {
        ids.par_iter().for_each(|id| {
            // Checked per file rather than mid-file: a cancel should never
            // leave a half-written output on disk for the user to find later.
            if state.is_cancelled() {
                return;
            }
            if !run_one(app, state, id, &settings, &template) {
                failed.fetch_add(1, Ordering::Relaxed);
            }
            let finished = done.fetch_add(1, Ordering::Relaxed) + 1;
            emit(
                app,
                EVENT_QUEUE_PROGRESS,
                QueueProgress {
                    done: finished,
                    total,
                },
            );
        });
    });

    let failed = failed.load(Ordering::Relaxed);
    let processed = done.load(Ordering::Relaxed).saturating_sub(failed);
    emit(
        app,
        EVENT_QUEUE_FINISHED,
        QueueFinished { processed, failed },
    );
    Ok(())
}

/// A file already `Done` is skipped, so pressing Start again after adding more
/// files does not reprocess the batch that just finished. An `Error` row is
/// retried, since the usual reason to press Start again is that the user fixed
/// whatever was wrong.
///
/// `Analyzing` is included, which is why the statuses are listed rather than
/// the two excluded. Analysis starts the moment files are added, so a user who
/// drops a folder and presses Start a second later is looking at a table of
/// rows mid-decode; leaving those out meant the files they had just added were
/// exactly the ones silently skipped. Nothing is lost by taking them: analysis
/// is the report, not a stage, and the run measures loudness itself.
fn pending_ids(state: &AppState) -> Vec<String> {
    state
        .queue
        .lock()
        .iter()
        .filter(|f| {
            matches!(
                f.status,
                FileStatus::Pending
                    | FileStatus::Analyzing
                    | FileStatus::Analyzed
                    | FileStatus::Error
            )
        })
        .map(|f| f.id.clone())
        .collect()
}

/// Processes one row and records the outcome on it. Returns false if it failed.
///
/// The row's own Status cell is where a failure has to land: the user is
/// looking at the table, so an error that only went to the log would read as a
/// file that silently did nothing.
fn run_one(
    app: &AppHandle,
    state: &AppState,
    id: &str,
    settings: &Settings,
    template: &Template,
) -> bool {
    if let Some(file) = state.update_file(id, |f| {
        f.status = FileStatus::Processing;
        f.error = None;
    }) {
        emit_file(app, &file);
    }

    match process_one(state, id, settings, template) {
        Ok(result) => {
            if let Some(file) = state.update_file(id, |f| {
                f.status = FileStatus::Done;
                f.result = Some(result.clone());
            }) {
                emit_file(app, &file);
            }
            true
        }
        Err(e) => {
            let message = e.to_string();
            if let Some(file) = state.update_file(id, |f| {
                f.status = FileStatus::Error;
                f.error = Some(message.clone());
            }) {
                emit_file(app, &file);
            }
            false
        }
    }
}

/// A failed emit is logged and swallowed everywhere in this module: the window
/// may simply have closed, and that is not a reason to abandon a run that is
/// still writing files.
fn emit<P: Serialize + Clone>(app: &AppHandle, event: &str, payload: P) {
    if let Err(e) = app.emit(event, payload) {
        tracing::warn!("could not emit {}: {}", event, e);
    }
}

/// Decode, process, encode. The only stage that knows about the filesystem is
/// this one and `output`, which is what lets the whole DSP chain be tested on a
/// buffer with no disk involved.
fn process_one(
    state: &AppState,
    id: &str,
    settings: &Settings,
    template: &Template,
) -> Result<ProcessResult> {
    let file = state.queued_file(id)?;
    let source = PathBuf::from(&file.path);

    let mut audio = crate::audio::decode::decode_to_f32(&source)?;
    let mut result = crate::audio::process::process(&mut audio, template)?;

    let dest = output::output_path(
        &source,
        file.source_root.as_deref().map(Path::new),
        settings,
    )?;
    let spec = EncodeSpec {
        // No resampling: the file keeps the rate it arrived at. The format
        // labels in the UI name the common case, not a conversion.
        sample_rate: audio.sample_rate,
        channels: audio.channel_count(),
        bit_depth: output::bit_depth_for(settings.format),
        mp3_rate: settings.mp3_rate,
    };

    // Through `write_atomically` rather than straight onto `dest`: a file only
    // appears at the destination once it is complete, so a cancelled run or a
    // failed encode cannot leave a truncated track for the user to find in a
    // set weeks later.
    output::write_atomically(&dest, |temp| {
        crate::audio::encode::encode(&audio, &spec, settings.format, temp)
    })?;

    // `process` never touches the filesystem, so it leaves this empty and the
    // path is filled in here, once there is actually a file at it.
    result.output_path = dest.to_string_lossy().to_string();
    Ok(result)
}
