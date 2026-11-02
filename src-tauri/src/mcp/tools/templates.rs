//! Templates: the mastering settings an agent is most likely to be asked to
//! adjust. On disk, so every tool here works with the app closed.

use serde_json::{json, Value};

use super::{object_arg, ok, str_arg};
use crate::error::{CapstoneError, Result};
use crate::mcp::McpContext;
use crate::models::Template;
use crate::templates::{catalog, share};

pub fn list(ctx: &McpContext) -> Result<Value> {
    let templates = ctx.templates.read().clone();
    Ok(json!({ "templates": serde_json::to_value(templates)? }))
}

pub fn get(ctx: &McpContext, args: &Value) -> Result<Value> {
    Ok(serde_json::to_value(by_id(ctx, &str_arg(args, "templateId")?)?)?)
}

/// Change named fields on a template.
///
/// A patch rather than the whole object, which is what the window's own
/// `save_template` takes. An agent asked to raise a target by a decibel
/// should not have to round-trip fourteen fields and six bands to do it, and
/// every field it echoes back is a field it can corrupt.
///
/// The merge goes through `Template`'s own deserialisation and then
/// `share::clamped`, so a patch cannot put a value outside the range the
/// Configure tab enforces — the same guard an imported file gets.
pub fn update(ctx: &McpContext, args: &Value) -> Result<Value> {
    let id = str_arg(args, "templateId")?;
    let patch = object_arg(args, "fields")?;
    let existing = by_id(ctx, &id)?;

    if existing.is_stock {
        return Err(CapstoneError::Other(format!(
            "`{}` is a stock template and is read-only; duplicate it first",
            existing.name
        )));
    }

    let mut merged = serde_json::to_value(&existing)?;
    let object = merged
        .as_object_mut()
        .expect("a template serialises to an object");
    for (key, value) in patch {
        if !object.contains_key(key) {
            return Err(CapstoneError::Other(format!(
                "`{key}` is not a template field"
            )));
        }
        object.insert(key.clone(), value.clone());
    }

    // The id and the stock flag are identity, not settings: a patch that
    // moved either would either orphan the template or forge a stock one.
    object.insert("id".into(), json!(existing.id));
    object.insert("isStock".into(), json!(false));

    let updated: Template = serde_json::from_value(merged)
        .map_err(|e| CapstoneError::Other(format!("those fields do not make a template: {e}")))?;
    let updated = share::clamped(updated);

    {
        let mut templates = ctx.templates.write();
        match templates.iter_mut().find(|t| t.id == updated.id) {
            Some(slot) => *slot = updated.clone(),
            None => templates.push(updated.clone()),
        }
    }
    ctx.save_templates();
    Ok(serde_json::to_value(updated)?)
}

pub fn duplicate(ctx: &McpContext, args: &Value) -> Result<Value> {
    let source = by_id(ctx, &str_arg(args, "templateId")?)?;
    let taken: Vec<String> = ctx.templates.read().iter().map(|t| t.name.clone()).collect();
    let desired = super::str_arg(args, "name").unwrap_or_else(|_| format!("{} copy", source.name));

    let copy = Template {
        id: uuid::Uuid::new_v4().to_string(),
        name: share::unique_name(&desired, &taken),
        // The whole point of duplicating: the copy is editable.
        is_stock: false,
        ..source
    };
    ctx.templates.write().push(copy.clone());
    ctx.save_templates();
    Ok(serde_json::to_value(copy)?)
}

pub fn delete(ctx: &McpContext, args: &Value) -> Result<Value> {
    let id = str_arg(args, "templateId")?;
    let template = by_id(ctx, &id)?;
    if template.is_stock {
        return Err(CapstoneError::Other(
            "stock templates cannot be deleted".into(),
        ));
    }
    ctx.templates.write().retain(|t| t.id != id);
    ctx.save_templates();

    // The active template pointing at something deleted would leave a run
    // with nothing to apply, so it falls back the way the app's own delete
    // does.
    let mut settings = ctx.settings.write();
    if settings.active_template_id == id {
        settings.active_template_id = catalog::stock()
            .first()
            .map(|t| t.id.clone())
            .unwrap_or_default();
    }
    drop(settings);
    ctx.save_settings();
    ok()
}

/// Write a template to a path. No picker: the agent names the file.
pub fn export(ctx: &McpContext, args: &Value) -> Result<Value> {
    let template = by_id(ctx, &str_arg(args, "templateId")?)?;
    let dest = str_arg(args, "destinationPath")?;
    std::fs::write(&dest, share::to_json(&template, "")?)?;
    Ok(json!({ "path": dest, "name": template.name }))
}

/// Read a shared template file in.
///
/// Through `share::from_json`, which is where trusting the file is handled: a
/// fresh id, a name that cannot overwrite one already there, and every value
/// clamped to the range the Configure tab enforces.
pub fn import(ctx: &McpContext, args: &Value) -> Result<Value> {
    let source = str_arg(args, "sourcePath")?;
    let raw = std::fs::read_to_string(&source)?;
    let taken: Vec<String> = ctx.templates.read().iter().map(|t| t.name.clone()).collect();
    let (template, description) = share::from_json(&raw, &taken)?;

    ctx.templates.write().push(template.clone());
    ctx.save_templates();
    Ok(json!({
        "template": serde_json::to_value(template)?,
        "description": description,
    }))
}

fn by_id(ctx: &McpContext, id: &str) -> Result<Template> {
    ctx.templates
        .read()
        .iter()
        .find(|t| t.id == id)
        .cloned()
        .ok_or_else(|| CapstoneError::NotFound(format!("no template with id {id}")))
}
