//! One function per tool, and the table that reaches them.

use serde_json::{json, Value};

use super::McpContext;
use crate::error::{CapstoneError, Result};

mod queue;
mod settings;
mod templates;

pub fn dispatch(ctx: &McpContext, tool: &str, args: &Value) -> Result<Value> {
    match tool {
        // Templates — the mastering settings. On disk, so these work with
        // the app closed.
        "list_templates" => templates::list(ctx),
        "get_template" => templates::get(ctx, args),
        "update_template" => templates::update(ctx, args),
        "duplicate_template" => templates::duplicate(ctx, args),
        "delete_template" => templates::delete(ctx, args),
        "export_template" => templates::export(ctx, args),
        "import_template" => templates::import(ctx, args),

        // Output settings. Also on disk.
        "get_settings" => settings::get(ctx),
        "update_settings" => settings::update(ctx, args),

        // The queue and the run. In the app only.
        "get_queue" => queue::get(ctx),
        "add_files" => queue::add(ctx, args),
        "remove_from_queue" => queue::remove(ctx, args),
        "clear_queue" => queue::clear(ctx),
        "analyze_file" => queue::analyze(ctx, args),
        "start_processing" => queue::start(ctx),
        "cancel_processing" => queue::cancel(ctx),

        other => Err(CapstoneError::NotFound(format!("no tool named {other}"))),
    }
}

// ---------------------------------------------------------------------------
// Argument reading. Same shape as Keystone's, and the same reason: a missed
// argument has to come back as a sentence naming it, because that message is
// all the model has to correct itself with.
// ---------------------------------------------------------------------------

pub fn missing(key: &str) -> CapstoneError {
    CapstoneError::Other(format!("`{key}` is required"))
}

pub fn str_arg(args: &Value, key: &str) -> Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| missing(key))
}

pub fn strings_arg(args: &Value, key: &str) -> Vec<String> {
    match args.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Some(Value::String(one)) => vec![one.clone()],
        _ => Vec::new(),
    }
}

/// The patch object a partial update takes.
pub fn object_arg<'a>(args: &'a Value, key: &str) -> Result<&'a serde_json::Map<String, Value>> {
    args.get(key)
        .and_then(Value::as_object)
        .filter(|o| !o.is_empty())
        .ok_or_else(|| CapstoneError::Other(format!("`{key}` is an object of fields to change")))
}

pub fn ok() -> Result<Value> {
    Ok(json!({ "ok": true }))
}
