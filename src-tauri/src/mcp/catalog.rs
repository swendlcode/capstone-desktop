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
    "launch_app",
];

/// Said on every tool that cannot work without the window.
///
/// It names `launch_app`, because "needs the app" is only half an instruction:
/// the other half is what to do about it.
const NEEDS_APP: &str = " Needs Capstone to be running — call launch_app first if it is not.";

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

fn boolean(description: &str) -> Value {
    json!({ "type": "boolean", "description": description })
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
                "fields": json!({ "type": "object", "description": "Field name to new value: outputLocation, outputDir, mirrorInputStructure, format, mp3Rate, maxCores, activeTemplateId, collisionPolicy (overwrite, skip or rename — what to do when a file is already at the destination; two files in one run that would collide are always renamed apart regardless)." })
            }),
            &["fields"],
        ),
        tool(
            "get_queue",
            &format!("How a run is going. Returns counts by status (pending, analyzing, analyzed, processing, done, error) plus any failures with their reasons. Pass `files` true, or a `status`, for the full rows — fifty files of rows is a great deal of detail for a progress check.{NEEDS_APP}"),
            json!({
                "files": boolean("Include the full row for each file: its readings, its result, its paths. Default false."),
                "status": string("Only files in this state: pending, analyzing, analyzed, processing, done or error. Implies `files`.")
            }),
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
        tool(
            "launch_app",
            "Start Capstone and wait until it answers, so the tools that need the running app will work. Returns immediately if it is already running.",
            json!({ "wait": boolean("Wait for it to come up, default true. False returns as soon as it has been started.") }),
            &[],
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three lists — `TOOL_NAMES`, the catalogue, and the match arms in
    /// `tools::dispatch` — have to agree, and nothing checked it. A tool
    /// advertised without an arm answers "no tool named"; an arm with no
    /// catalogue entry is a tool the model never finds out about.
    #[test]
    fn the_catalogue_and_the_name_list_agree() {
        let advertised: Vec<String> = tool_list()
            .as_array()
            .expect("an array of tools")
            .iter()
            .map(|t| t["name"].as_str().expect("a name").to_string())
            .collect();
        let listed: Vec<String> = TOOL_NAMES.iter().map(|s| s.to_string()).collect();
        assert_eq!(advertised, listed, "tool_list and TOOL_NAMES disagree");
    }

    #[test]
    fn every_tool_has_a_description_and_a_coherent_schema() {
        for entry in tool_list().as_array().expect("an array") {
            let name = entry["name"].as_str().expect("a name");
            assert!(
                !entry["description"]
                    .as_str()
                    .unwrap_or_default()
                    .trim()
                    .is_empty(),
                "{name} has no description"
            );
            assert_eq!(entry["inputSchema"]["type"], "object", "{name}");
            for required in entry["inputSchema"]["required"]
                .as_array()
                .expect("a required list")
            {
                let key = required.as_str().expect("a string");
                assert!(
                    entry["inputSchema"]["properties"].get(key).is_some(),
                    "{name} requires `{key}` but does not declare it"
                );
            }
        }
    }

    /// The tools that cannot work with the app closed must say so, and say
    /// what to do about it. Every one of them goes through `require_live`.
    #[test]
    fn the_live_only_tools_name_the_way_out() {
        let live_only = [
            "get_queue",
            "add_files",
            "remove_from_queue",
            "clear_queue",
            "analyze_file",
            "start_processing",
            "cancel_processing",
        ];
        for entry in tool_list().as_array().expect("an array") {
            let name = entry["name"].as_str().expect("a name");
            if !live_only.contains(&name) {
                continue;
            }
            let description = entry["description"].as_str().unwrap_or_default();
            assert!(
                description.contains("launch_app"),
                "{name} needs the app but does not tell the model how to get it"
            );
        }
    }
}
