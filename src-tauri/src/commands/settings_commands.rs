use tauri::State;

use crate::error::Result;
use crate::models::Settings;
use crate::state::AppState;

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings> {
    Ok(state.settings.read().clone())
}

/// Returns the settings as stored rather than as sent, because they are not
/// always the same: a zero core count is corrected here, and the frontend needs
/// to see the corrected value or its slider and the next run would disagree.
#[tauri::command]
pub async fn update_settings(
    settings: Settings,
    state: State<'_, AppState>,
) -> Result<Settings> {
    let mut settings = settings;
    if settings.max_cores == 0 {
        settings.max_cores = crate::settings::default_max_cores();
    }

    *state.settings.write() = settings.clone();
    state.save_settings();
    Ok(settings)
}
