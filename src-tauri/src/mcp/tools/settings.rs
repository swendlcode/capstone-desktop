//! Output settings: where masters are written, in what format, and which
//! template a run applies.

use serde_json::{json, Value};

use super::object_arg;
use crate::error::{CapstoneError, Result};
use crate::mcp::McpContext;
use crate::models::Settings;

pub fn get(ctx: &McpContext) -> Result<Value> {
    Ok(serde_json::to_value(ctx.settings.read().clone())?)
}

/// A patch, for the same reason `update_template` takes one.
///
/// `activeTemplateId` is checked against the templates that exist. Pointing it
/// at nothing would leave the next run with no settings to apply, and the
/// failure would surface as a refused run rather than as this call.
pub fn update(ctx: &McpContext, args: &Value) -> Result<Value> {
    let patch = object_arg(args, "fields")?;
    let existing = ctx.settings.read().clone();

    let mut merged = serde_json::to_value(&existing)?;
    let object = merged
        .as_object_mut()
        .expect("settings serialise to an object");
    for (key, value) in patch {
        if !object.contains_key(key) {
            return Err(CapstoneError::Other(format!(
                "`{key}` is not a settings field"
            )));
        }
        object.insert(key.clone(), value.clone());
    }

    let updated: Settings = serde_json::from_value(merged)
        .map_err(|e| CapstoneError::Other(format!("those fields do not make settings: {e}")))?;

    if !ctx
        .templates
        .read()
        .iter()
        .any(|t| t.id == updated.active_template_id)
    {
        return Err(CapstoneError::NotFound(format!(
            "no template with id {}",
            updated.active_template_id
        )));
    }

    *ctx.settings.write() = updated.clone();
    ctx.save_settings();
    Ok(json!(serde_json::to_value(updated)?))
}
