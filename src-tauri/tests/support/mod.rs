//! Shared scaffolding for the end-to-end audio tests.
//!
//! `pipeline` below is the same sequence `jobs::process_one` runs - decode the
//! file, run the chain, resolve a destination, write it atomically, encode -
//! assembled out of the same public functions. The one thing it cannot reuse is
//! the queue runner itself, which needs a live `AppHandle`: everything from the
//! decode inward is the production path, and the IPC layer above it is a thin
//! wrapper over exactly these calls.

#![allow(dead_code)]

pub mod files;
pub mod fixtures;
pub mod measure;

use std::path::{Path, PathBuf};

use capstone_lib::audio::{decode, encode, process, DecodedAudio};
use capstone_lib::jobs::output;
use capstone_lib::models::{
    Mp3Rate, OutputFormat, OutputLocation, ProcessResult, Settings, Template,
};
use capstone_lib::templates;

/// What one trip through the pipeline produced.
pub struct Run {
    /// The report the row would show.
    pub result: ProcessResult,
    /// The buffer as the chain left it, before any encoder touched it.
    pub processed: DecodedAudio,
    /// The written file, decoded back through the same reader the app uses.
    pub decoded: DecodedAudio,
    pub output: PathBuf,
}

/// Settings that write `format` into `dir`, with everything else at the value
/// the Personalize tab ships with.
pub fn settings(format: OutputFormat, dir: &Path) -> Settings {
    Settings {
        output_location: OutputLocation::Specific,
        output_dir: Some(dir.to_string_lossy().to_string()),
        mirror_input_structure: false,
        format,
        mp3_rate: Mp3Rate::Cbr320,
        max_cores: 1,
        active_template_id: templates::FESTIVAL_ID.to_string(),
    }
}

/// Decode, process, encode, decode back.
///
/// The destination comes from `output::output_path` and the write goes through
/// `output::write_atomically`, so a file only ever appears at the destination
/// once it is complete - the same guarantee the app makes.
pub fn pipeline(source: &Path, template: &Template, settings: &Settings) -> Run {
    let (mut audio, tags) = decode::decode_with_tags(source)
        .unwrap_or_else(|e| panic!("decoding {} failed: {e}", source.display()));

    let mut result = process::process(&mut audio, template)
        .unwrap_or_else(|e| panic!("processing {} failed: {e}", source.display()));

    let dest = output::output_path(source, settings).expect("output path");
    let spec = encode::EncodeSpec {
        // No resampling: the file keeps the rate it arrived at, which is what
        // `jobs::process_one` asks for.
        sample_rate: audio.sample_rate,
        channels: audio.channel_count(),
        bit_depth: output::bit_depth_for(settings.format),
        mp3_rate: settings.mp3_rate,
    };
    output::write_atomically(&dest, |temp| {
        encode::encode_with_tags(&audio, &spec, settings.format, temp, &tags)
    })
    .unwrap_or_else(|e| panic!("writing {} failed: {e}", dest.display()));
    result.output_path = dest.to_string_lossy().to_string();

    let decoded = decode::decode_to_f32(&dest)
        .unwrap_or_else(|e| panic!("reading back {} failed: {e}", dest.display()));

    Run {
        result,
        processed: audio,
        decoded,
        output: dest,
    }
}

/// The same trip, but reporting the failure instead of panicking on it. The
/// edge-case tests need to distinguish "declined cleanly" from "panicked".
pub fn try_pipeline(
    source: &Path,
    template: &Template,
    settings: &Settings,
) -> Result<Run, String> {
    let (mut audio, tags) = decode::decode_with_tags(source).map_err(|e| e.to_string())?;
    let mut result = process::process(&mut audio, template).map_err(|e| e.to_string())?;
    let dest = output::output_path(source, settings).map_err(|e| e.to_string())?;
    let spec = encode::EncodeSpec {
        sample_rate: audio.sample_rate,
        channels: audio.channel_count(),
        bit_depth: output::bit_depth_for(settings.format),
        mp3_rate: settings.mp3_rate,
    };
    output::write_atomically(&dest, |temp| {
        encode::encode_with_tags(&audio, &spec, settings.format, temp, &tags)
    })
    .map_err(|e| e.to_string())?;
    result.output_path = dest.to_string_lossy().to_string();
    let decoded = decode::decode_to_f32(&dest).map_err(|e| e.to_string())?;
    Ok(Run {
        result,
        processed: audio,
        decoded,
        output: dest,
    })
}

/// The ceiling the template's headroom implies, in dBFS.
pub fn ceiling_db(template: &Template) -> f64 {
    -template.headroom_db
}
