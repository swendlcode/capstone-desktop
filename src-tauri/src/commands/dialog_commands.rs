use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::audio::decode::AUDIO_EXTENSIONS;
use crate::error::{CapstoneError, Result};
use crate::jobs::output;
use crate::state::AppState;

#[tauri::command]
pub async fn open_output_folder(app: AppHandle, state: State<'_, AppState>) -> Result<()> {
    let settings = state.settings.read().clone();
    let dir = output::resolved_output_dir(&settings);

    // Created first: asking the OS to reveal a folder that does not exist yet
    // fails silently on both platforms, and the button then looks broken on a
    // fresh install where nothing has been processed.
    std::fs::create_dir_all(&dir)?;

    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| CapstoneError::Other(e.to_string()))
}

/// The picker is callback-based, so the result comes back over a oneshot rather
/// than through a blocking call. The blocking variants deadlock when they are
/// invoked from the main thread on macOS, which is exactly where an IPC command
/// can land.
#[tauri::command]
pub async fn pick_files(app: AppHandle) -> Result<Vec<String>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Audio", AUDIO_EXTENSIONS)
        .pick_files(move |picked| {
            let _ = tx.send(picked);
        });

    let picked = rx
        .await
        .map_err(|e| CapstoneError::Other(format!("file picker closed: {}", e)))?;

    // None means the user cancelled, which is not an error.
    Ok(picked
        .map(|paths| paths.into_iter().map(|p| p.to_string()).collect())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn pick_folder(app: AppHandle) -> Result<Option<String>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().pick_folder(move |picked| {
        let _ = tx.send(picked);
    });

    let picked = rx
        .await
        .map_err(|e| CapstoneError::Other(format!("folder picker closed: {}", e)))?;

    Ok(picked.map(|path| path.to_string()))
}
