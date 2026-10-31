use tauri::State;

use crate::error::{CapstoneError, Result};
use crate::models::Template;
use crate::state::AppState;

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
