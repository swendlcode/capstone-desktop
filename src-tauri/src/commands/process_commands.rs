use tauri::{AppHandle, State};

use crate::error::Result;
use crate::jobs;
use crate::state::AppState;

/// Spawns the run and returns at once. The frontend learns what happened from
/// `capstone://queue-progress` and `capstone://queue-finished`, never from this
/// call's return value.
#[tauri::command]
pub async fn start_processing(app: AppHandle) -> Result<()> {
    jobs::start(app)
}

/// Raises the flag and returns. The run stops at the next file boundary, so
/// this is not a promise that nothing is being written right now.
#[tauri::command]
pub async fn cancel_processing(state: State<'_, AppState>) -> Result<()> {
    jobs::cancel(state.inner());
    Ok(())
}
