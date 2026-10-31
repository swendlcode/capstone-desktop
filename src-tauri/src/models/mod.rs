//! The IPC contract.
//!
//! Every type in this file crosses the Tauri bridge, so every struct carries
//! `rename_all = "camelCase"` and every enum the casing its JSON form uses.
//! Nothing on either side of the bridge checks this at compile time: a missed
//! rename surfaces as an `undefined` column in the UI, which is why it is
//! spelled out on each type rather than assumed.
//!
//! Field names here are frozen. The frontend is built against exactly these.

use serde::{Deserialize, Serialize};

/// What analysis found in a file, before anything is changed.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub duration_sec: f64,
    pub sample_rate: u32,
    pub channels: u16,
    /// Integrated loudness, EBU R128.
    pub lufs: f64,
    pub true_peak_db: f64,
    /// Consecutive-sample runs at or over the ceiling, not individual samples:
    /// one flattened transient counts once however many samples it squashed.
    pub clip_count: u64,
    /// Peak-to-RMS. This is what the Dynamics column shows.
    pub crest_db: f64,
    /// Deviation from concert pitch, signed; negative is flat.
    pub tuning_cents: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Pending,
    Analyzing,
    Analyzed,
    Processing,
    Done,
    Error,
}

/// One row of the Process Audio Files table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedFile {
    pub id: String,
    pub path: String,
    pub filename: String,
    pub analysis: Option<Analysis>,
    pub status: FileStatus,
    pub result: Option<ProcessResult>,
    pub error: Option<String>,
    /// The folder this file was dropped in, when it arrived as part of one.
    ///
    /// `None` for a file picked individually. Mirroring needs it: without a
    /// record of what the user actually dropped there is no way to know how
    /// much of the path below it to reproduce, and reproducing only the
    /// immediate parent quietly flattens a nested crate.
    pub source_root: Option<String>,
}

/// What processing actually did, which is not the same as what was asked for:
/// the table's middle columns switch from the analysis figures to these once a
/// file is done.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessResult {
    pub output_path: String,
    pub lufs_before: f64,
    pub lufs_after: f64,
    pub clips_repaired: u64,
    pub gain_applied_db: f64,
    pub pitch_corrected_cents: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Warmth {
    None,
    Gentle,
    Tube,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PitchMode {
    /// Shift pitch and leave the running time alone.
    KeepTempo,
    /// Resample, the way nudging a turntable does: pitch and tempo move together.
    ChangeTempo,
}

/// One band of the multiband compressor.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandSetting {
    pub low_hz: f32,
    pub high_hz: f32,
    pub threshold_db: f32,
    pub ratio: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub makeup_db: f32,
}

/// A named set of processing settings. Stock templates are read-only; the UI
/// makes one editable by duplicating it.
///
/// The container-level serde default is what lets a templates.json written by
/// an older build deserialize after a release adds a field, rather than the
/// whole file failing to parse and the user losing every template they made.
/// The values themselves live in `crate::templates`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Template {
    pub id: String,
    pub name: String,
    pub is_stock: bool,
    pub target_lufs: f64,
    pub limiter_character: u8,
    pub repair_clips: bool,
    pub repair_above_lufs: f64,
    pub clip_threshold_pct: f64,
    pub headroom_db: f64,
    pub warmth: Warmth,
    pub correct_pitch: bool,
    pub pitch_tolerance_cents: f64,
    pub pitch_mode: PitchMode,
    pub bands: Vec<BandSetting>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Aiff,
    Alac,
    Flac,
    Mp3,
    Wav,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mp3Rate {
    Cbr320,
    Vbr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputLocation {
    Specific,
    SameAsSource,
}

/// Everything on the Personalize tab. Defaults live in `crate::settings`, for
/// the same forward-compatibility reason as `Template`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub output_location: OutputLocation,
    pub output_dir: Option<String>,
    pub mirror_input_structure: bool,
    pub format: OutputFormat,
    pub mp3_rate: Mp3Rate,
    pub max_cores: usize,
    pub active_template_id: String,
}
