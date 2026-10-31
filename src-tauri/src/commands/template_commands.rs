use std::io::Read;
use std::path::PathBuf;

use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

use crate::error::{CapstoneError, Result};
use crate::models::{ImportedTemplate, Template};
use crate::state::AppState;
use crate::templates::share;

#[tauri::command]
pub async fn get_templates(state: State<'_, AppState>) -> Result<Vec<Template>> {
    Ok(state.templates.read().clone())
}

/// Stock templates are read-only. The refusal lives here rather than only in
/// the UI, because the UI's gear menu is not the only way this command can be
/// reached and a stock template edited on disk would be silently re-merged away
/// on the next launch anyway.
#[tauri::command]
pub async fn save_template(
    template: Template,
    state: State<'_, AppState>,
) -> Result<Template> {
    if template.is_stock {
        return Err(CapstoneError::Other(
            "stock templates are read-only; duplicate it first".to_string(),
        ));
    }

    // An `is_stock: false` template wearing a stock id is refused rather than
    // stored, because `templates::load` drops any stored copy of a stock id on
    // the next launch. Accepting it would look like a save that worked and then
    // lose the user's settings the next time they opened the app.
    if crate::templates::stock().iter().any(|s| s.id == template.id) {
        return Err(CapstoneError::Other(format!(
            "{} is a stock template id; duplicate it to get your own",
            template.id
        )));
    }

    {
        let mut templates = state.templates.write();
        match templates.iter_mut().find(|t| t.id == template.id) {
            Some(existing) => *existing = template.clone(),
            None => templates.push(template.clone()),
        }
    }
    state.save_templates();
    Ok(template)
}

#[tauri::command]
pub async fn duplicate_template(
    id: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<Template> {
    let source = state.template_by_id(&id)?;
    let copy = Template {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        // The whole point of duplicating: the copy is editable.
        is_stock: false,
        ..source
    };

    state.templates.write().push(copy.clone());
    state.save_templates();
    Ok(copy)
}

#[tauri::command]
pub async fn delete_template(id: String, state: State<'_, AppState>) -> Result<()> {
    let template = state.template_by_id(&id)?;
    if template.is_stock {
        return Err(CapstoneError::Other(
            "stock templates cannot be deleted".to_string(),
        ));
    }

    state.templates.write().retain(|t| t.id != id);

    // Deleting the active template would otherwise leave `active_template_id`
    // pointing at nothing. `AppState::active_template` falls back to Festival
    // regardless, but the setting is corrected here so the picker shows the
    // truth rather than an empty selection.
    {
        let mut settings = state.settings.write();
        if settings.active_template_id == id {
            settings.active_template_id = crate::templates::FESTIVAL_ID.to_string();
        }
    }

    state.save_templates();
    state.save_settings();
    Ok(())
}

/// Writes the template to a file the user picks. `None` when they cancel.
///
/// The picker runs on the Rust side, like the queue's own, and the file is
/// written here too: a template is the user's own settings and there is nothing
/// for the webview to do with the bytes but hand them straight back.
#[tauri::command]
pub async fn export_template(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>> {
    let template = state.template_by_id(&id)?;
    // No description: the per-template line lives in the frontend's own map
    // today, and exporting an empty one is better than exporting a wrong one.
    let json = share::to_json(&template, "")?;

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name(format!("{}.capstone.json", template.name))
        .add_filter("Capstone template", &["json"])
        .save_file(move |picked| {
            let _ = tx.send(picked);
        });

    let picked = rx
        .await
        .map_err(|e| CapstoneError::Other(format!("save dialog closed: {}", e)))?;
    let Some(target) = picked else {
        return Ok(None);
    };

    let path = PathBuf::from(target.to_string());
    std::fs::write(&path, json)?;
    Ok(Some(path.to_string_lossy().to_string()))
}

/// Reads a shared template file into the user's own templates. `None` when the
/// user cancels the picker.
///
/// Everything about trusting the file is [`share::from_json`]'s: a fresh id, a
/// name that cannot overwrite one already there, and every value clamped to the
/// range the Configure tab enforces.
#[tauri::command]
pub async fn import_template(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<ImportedTemplate>> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("Capstone template", &["json"])
        .pick_file(move |picked| {
            let _ = tx.send(picked);
        });

    let picked = rx
        .await
        .map_err(|e| CapstoneError::Other(format!("file picker closed: {}", e)))?;
    let Some(chosen) = picked else {
        return Ok(None);
    };

    let raw = read_capped(&PathBuf::from(chosen.to_string()))?;
    let taken: Vec<String> = state
        .templates
        .read()
        .iter()
        .map(|t| t.name.clone())
        .collect();
    let (template, description) = share::from_json(&raw, &taken)?;

    state.templates.write().push(template.clone());
    state.save_templates();
    Ok(Some(ImportedTemplate {
        template,
        description,
    }))
}

/// Reads at most one byte past the share format's own ceiling, which is what
/// lets `from_json` refuse an oversized file by its size without the file
/// having been loaded whole first. Lossy UTF-8 so a binary the user mispicked
/// comes back as "not a Capstone template file" rather than as an encoding
/// error nobody can act on.
fn read_capped(path: &PathBuf) -> Result<String> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(share::MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
