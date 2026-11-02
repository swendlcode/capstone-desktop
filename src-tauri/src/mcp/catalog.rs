//! What `tools/list` answers.
//!
//! The descriptions say which tools need the app open, because that is the
//! one thing a model cannot work out for itself here and the one thing that
//! changes what it should do next.

use serde_json::{json, Value};

pub const TOOL_NAMES: &[&str] = &[
    "list_templates",
    "get_template",
    "update_template",
    "duplicate_template",
    "delete_template",
    "export_template",
    "import_template",
    "get_settings",
    "update_settings",
    "get_queue",
    "add_files",
    "remove_from_queue",
    "clear_queue",
    "analyze_file",
    "start_processing",
    "cancel_processing",
];

/// Said on every tool that cannot work without the window.
const NEEDS_APP: &str = " Needs Capstone to be running.";

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": props,
            "required": required,
        },
    })
}

fn string(description: &str) -> Value {
    json!({ "type": "string", "description": description })
}

pub fn tool_list() -> Value {
    json!([
        tool(
            "list_templates",
            "Every mastering template: target loudness, limiter character, clip repair, warmth, pitch correction and the EQ bands. Stock templates are marked and read-only.",
            json!({}),
            &[],
        ),
        tool(
            "get_template",
            "One template in full.",
            json!({ "templateId": string("The template's id.") }),
            &["templateId"],
        ),
        tool(
            "update_template",
            "Change named fields on a template. Send only what changes, e.g. {\"targetLufs\": -9}. Values are clamped to the ranges the Configure tab enforces. Stock templates are read-only — duplicate one first. The id and stock flag cannot be patched.",
            json!({
                "templateId": string("The template's id."),
                "fields": json!({ "type": "object", "description": "Field name to new value: targetLufs, limiterCharacter, repairClips, repairAboveLufs, clipThresholdPct, headroomDb, warmth, correctPitch, pitchToleranceCents, pitchMode, bands, name." })
            }),
            &["templateId", "fields"],
        ),
        tool(
            "duplicate_template",
            "Copy a template into an editable one. This is how you get a changeable version of a stock template.",
            json!({
                "templateId": string("The template to copy."),
                "name": string("What to call the copy. Defaults to '<name> copy'. A clashing name is made unique.")
            }),
            &["templateId"],
        ),
        tool(
            "delete_template",
            "Delete a template. Stock templates cannot be deleted. If it was the active one, the active template falls back to a stock one.",
            json!({ "templateId": string("The template's id.") }),
            &["templateId"],
        ),
        tool(
            "export_template",
            "Write a template to a shareable .json file at a path you name.",
            json!({
                "templateId": string("The template's id."),
                "destinationPath": string("Where to write the file.")
            }),
            &["templateId", "destinationPath"],
        ),
        tool(
            "import_template",
            "Read a shared template file in. It arrives with a fresh id, a name that cannot overwrite an existing one, and every value clamped.",
            json!({ "sourcePath": string("The .json file to read.") }),
            &["sourcePath"],
        ),
        tool(
            "get_settings",
            "Output settings: where masters are written, whether the input folder structure is mirrored, the format and MP3 rate, how many cores a run uses, and which template is active.",
            json!({}),
            &[],
        ),
        tool(
            "update_settings",
            "Change named output settings. Send only what changes, e.g. {\"format\": \"wav\"}. Setting activeTemplateId is checked against the templates that exist.",
            json!({
                "fields": json!({ "type": "object", "description": "Field name to new value: outputLocation, outputDir, mirrorInputStructure, format, mp3Rate, maxCores, activeTemplateId." })
            }),
            &["fields"],
        ),
        tool(
            "get_queue",
            &format!("The files waiting, being processed, or finished — with each one's readings and its result or error. This is how you find out how a run went.{NEEDS_APP}"),
            json!({}),
            &[],
        ),
        tool(
            "add_files",
            &format!("Queue files or folders for mastering. Folders are walked, non-audio is skipped, and a path already queued is not added twice. Reading them starts immediately.{NEEDS_APP}"),
            json!({ "paths": json!({ "type": "array", "items": { "type": "string" }, "description": "Files or folders." }) }),
            &["paths"],
        ),
        tool(
            "remove_from_queue",
            &format!("Take one file out of the queue.{NEEDS_APP}"),
            json!({ "fileId": string("The queued file's id, from get_queue.") }),
            &["fileId"],
        ),
        tool(
            "clear_queue",
            &format!("Empty the queue.{NEEDS_APP}"),
            json!({}),
            &[],
        ),
        tool(
            "analyze_file",
            &format!("Re-read one queued file. Use it to retry a file that errored, or after replacing it on disk.{NEEDS_APP}"),
            json!({ "fileId": string("The queued file's id.") }),
            &["fileId"],
        ),
        tool(
            "start_processing",
            &format!("WRITES MASTERED FILES. Start a run over the queue with the active template. Returns as soon as it starts, naming the template and output folder — call get_queue afterwards to see how each file ended up.{NEEDS_APP}"),
            json!({}),
            &[],
        ),
        tool(
            "cancel_processing",
            &format!("Stop a run. It stops at the next file boundary, so this is not a promise that nothing is being written right now.{NEEDS_APP}"),
            json!({}),
            &[],
        ),
    ])
}
