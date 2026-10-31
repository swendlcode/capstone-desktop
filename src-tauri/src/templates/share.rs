//! The shared-template file: one template written out as a `.json` anybody can
//! post, and the validation that lets Capstone open one it did not write.
//!
//! That file is the whole of "community templates". No server, no account, no
//! gallery: whoever has the file has the template.
//!
//! Which means everything in here arrives from outside, so every value is
//! clamped to the range the Configure tab itself enforces. That is not
//! politeness. `headroom_db` becomes the limiter's ceiling and `makeup_db` is
//! raw gain, so a hand-edited or hostile file could otherwise ask for a ceiling
//! above full scale, or 400 dB of makeup, and the first the user would know is
//! the damage. A band list with a gap in it is worse than wrong: `multiband`
//! reads each `high_hz` as the next band's `low_hz` without checking, so the
//! file would come back with a notch at every edge and nothing in the UI to
//! explain it.
//!
//! The split is deliberate. A file that is merely optimistic gets clamped; one
//! that is not a template at all is refused with a sentence, because silently
//! turning nonsense into Festival is worse than saying no.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::catalog;
use crate::error::{CapstoneError, Result};
use crate::models::{BandSetting, PitchMode, Template, Warmth};

/// Bumped only for a change that an older build could not read. New fields are
/// additive and arrive with a serde default, the same way `Template` itself
/// tolerates them.
pub const SHARE_FORMAT_VERSION: u32 = 1;

/// Nothing legitimate comes close. A template is a few hundred bytes, and the
/// cap is here so a multi-megabyte "template" is refused before it is parsed
/// rather than after.
///
/// Public because the import command reads the chosen file through it: a file
/// that is not a template at all must not be pulled into memory whole just to
/// be told it is too big.
pub const MAX_FILE_BYTES: usize = 64 * 1024;

/// Enough for any split `multiband` can actually use, and a bound on the work a
/// shared file can ask for: one compressor and one crossover chain per band are
/// allocated per channel.
const MAX_BANDS: usize = 8;

/// Long enough for a real name, short enough that a name cannot be used as a
/// payload. Names past this are truncated rather than refused.
const MAX_NAME_CHARS: usize = 60;

const FLOOR_HZ: f32 = 20.0;
const CEILING_HZ: f32 = 20_000.0;
/// The narrowest a band may be, matching the floor the crossover field on the
/// Configure tab enforces.
const MIN_BAND_HZ: f32 = 20.0;

/// The file's shape.
///
/// Field names are camelCase for the same reason the IPC types are, even though
/// this struct crosses no bridge: a user comparing a shared file against their
/// own `templates.json` should see the same spellings in both. Numbers are f64
/// regardless of what `Template` stores, so an out-of-range value arrives to be
/// clamped instead of failing to deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedTemplate {
    pub format_version: u32,
    pub name: String,
    /// One line on when to reach for it. Optional: a template exported before
    /// the picker had anywhere to show it carries none.
    #[serde(default)]
    pub description: String,
    pub target_lufs: f64,
    pub limiter_character: f64,
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

/// Serializes `template` as a shared file, pretty-printed because somebody is
/// going to read it in a text editor before they trust it.
pub fn to_json(template: &Template, description: &str) -> Result<String> {
    let mut json = serde_json::to_string_pretty(&to_shared(template, description))?;
    json.push('\n');
    Ok(json)
}

/// Parses a shared file into a user template: a fresh id, `is_stock: false`,
/// and a name that does not collide with anything in `taken`.
///
/// The author's description comes back beside the template rather than on it,
/// because `Template` has no field to keep it in. The importer quotes it once,
/// in the line confirming what was added.
pub fn from_json(raw: &str, taken: &[String]) -> Result<(Template, String)> {
    if raw.len() > MAX_FILE_BYTES {
        return Err(CapstoneError::Other(format!(
            "that file is {} KB; a template is a few hundred bytes",
            raw.len() / 1024
        )));
    }

    let shared: SharedTemplate = serde_json::from_str(raw)
        .map_err(|e| CapstoneError::Other(format!("not a Capstone template file: {}", e)))?;

    if shared.format_version != SHARE_FORMAT_VERSION {
        return Err(CapstoneError::Other(format!(
            "that file is template format {}, and this version of Capstone reads {}",
            shared.format_version, SHARE_FORMAT_VERSION
        )));
    }

    let template = bind(
        &shared,
        Uuid::new_v4().to_string(),
        unique_name(&shared.name, taken),
    )?;
    Ok((template, shared.description))
}

/// The same bounds applied to a template coming off disk, so a hand-edited
/// `templates.json` is as safe as an imported file. A template that cannot be
/// bounded at all keeps its name and takes Festival's values, logged: the
/// alternative is dropping something the user made without telling them.
pub fn clamped(template: Template) -> Template {
    let id = template.id.clone();
    let name = template.name.clone();
    bind(&to_shared(&template, ""), id.clone(), name.clone()).unwrap_or_else(|e| {
        tracing::warn!("template {:?} holds values Capstone cannot use ({}); falling back to the shipped settings", name, e);
        Template {
            id,
            name,
            is_stock: false,
            ..catalog::festival()
        }
    })
}

/// A name already in use gets a numeric suffix rather than overwriting. Two
/// people's "Warehouse" are two templates, and an import that quietly replaced
/// the one the user had spent an evening tuning would be the worst kind of data
/// loss: the invisible kind.
pub fn unique_name(desired: &str, taken: &[String]) -> String {
    let trimmed = desired.trim();
    let base: String = if trimmed.is_empty() {
        "Imported".to_string()
    } else {
        trimmed.chars().take(MAX_NAME_CHARS).collect()
    };

    if !taken.iter().any(|n| n == &base) {
        return base;
    }
    (2..u32::MAX)
        .map(|n| format!("{} {}", base, n))
        .find(|candidate| !taken.iter().any(|n| n == candidate))
        .unwrap_or(base)
}

fn to_shared(template: &Template, description: &str) -> SharedTemplate {
    SharedTemplate {
        format_version: SHARE_FORMAT_VERSION,
        name: template.name.clone(),
        description: description.to_string(),
        target_lufs: template.target_lufs,
        limiter_character: template.limiter_character as f64,
        repair_clips: template.repair_clips,
        repair_above_lufs: template.repair_above_lufs,
        clip_threshold_pct: template.clip_threshold_pct,
        headroom_db: template.headroom_db,
        warmth: template.warmth,
        correct_pitch: template.correct_pitch,
        pitch_tolerance_cents: template.pitch_tolerance_cents,
        pitch_mode: template.pitch_mode,
        bands: template.bands.clone(),
    }
}

/// Clamps every field into range and hands back the template, or refuses.
/// The ranges are the Configure tab's own: a value outside them is one no
/// control in the app can produce, so clamping to the edge lands the user where
/// a slider would have stopped them anyway.
fn bind(shared: &SharedTemplate, id: String, name: String) -> Result<Template> {
    Ok(Template {
        id,
        name,
        is_stock: false,
        target_lufs: bounded(shared.target_lufs, -24.0, -6.0, "targetLufs")?,
        limiter_character: bounded(shared.limiter_character, 0.0, 10.0, "limiterCharacter")?.round()
            as u8,
        repair_clips: shared.repair_clips,
        repair_above_lufs: bounded(shared.repair_above_lufs, -30.0, 0.0, "repairAboveLufs")?,
        clip_threshold_pct: bounded(shared.clip_threshold_pct, 90.0, 100.0, "clipThresholdPct")?,
        headroom_db: bounded(shared.headroom_db, 0.0, 6.0, "headroomDb")?,
        warmth: shared.warmth,
        correct_pitch: shared.correct_pitch,
        pitch_tolerance_cents: bounded(
            shared.pitch_tolerance_cents,
            1.0,
            50.0,
            "pitchToleranceCents",
        )?,
        pitch_mode: shared.pitch_mode,
        bands: bind_bands(&shared.bands)?,
    })
}

fn bind_bands(submitted: &[BandSetting]) -> Result<Vec<BandSetting>> {
    if submitted.is_empty() || submitted.len() > MAX_BANDS {
        return Err(CapstoneError::Other(format!(
            "a template needs 1 to {} bands; that file has {}",
            MAX_BANDS,
            submitted.len()
        )));
    }

    let mut bands: Vec<BandSetting> = Vec::with_capacity(submitted.len());
    for band in submitted {
        bands.push(BandSetting {
            // Overwritten by `retile`, which owns where the edges land.
            low_hz: FLOOR_HZ,
            high_hz: CEILING_HZ,
            threshold_db: bounded32(band.threshold_db, -60.0, 0.0, "thresholdDb")?,
            ratio: bounded32(band.ratio, 1.0, 20.0, "ratio")?,
            attack_ms: bounded32(band.attack_ms, 0.1, 200.0, "attackMs")?,
            release_ms: bounded32(band.release_ms, 10.0, 2000.0, "releaseMs")?,
            makeup_db: bounded32(band.makeup_db, -12.0, 12.0, "makeupDb")?,
        });
    }

    retile(&mut bands, submitted)?;
    Ok(bands)
}

/// Lays the bands end to end from 20 Hz to 20 kHz using the submitted
/// crossovers, sorted. A file with a gap, an overlap or its bands out of order
/// becomes a tiling rather than an error, because the shape is recoverable and
/// the alternative is refusing a template over an edge the user cannot see.
fn retile(bands: &mut [BandSetting], submitted: &[BandSetting]) -> Result<()> {
    let count = bands.len();
    let mut crossovers: Vec<f32> = Vec::with_capacity(count.saturating_sub(1));
    for band in &submitted[..count - 1] {
        crossovers.push(bounded32(band.high_hz, FLOOR_HZ, CEILING_HZ, "highHz")?);
    }
    crossovers.sort_by(|a, b| a.total_cmp(b));

    let mut low = FLOOR_HZ;
    for (index, band) in bands.iter_mut().enumerate() {
        band.low_hz = low;
        band.high_hz = match crossovers.get(index) {
            Some(&hz) => {
                // Every band left above this one still needs its own minimum
                // width, or the last crossovers would pile up against 20 kHz.
                let reserved = (count - index - 1) as f32 * MIN_BAND_HZ;
                let floor = low + MIN_BAND_HZ;
                hz.clamp(floor, (CEILING_HZ - reserved).max(floor))
            }
            None => CEILING_HZ,
        };
        low = band.high_hz;
    }
    Ok(())
}

/// A non-finite value is refused rather than clamped: NaN came from a broken
/// writer and not from an optimistic user, and `f64::clamp` panics on it.
fn bounded(value: f64, min: f64, max: f64, field: &str) -> Result<f64> {
    if !value.is_finite() {
        return Err(CapstoneError::Other(format!("{} is not a number", field)));
    }
    Ok(value.clamp(min, max))
}

fn bounded32(value: f32, min: f32, max: f32, field: &str) -> Result<f32> {
    if !value.is_finite() {
        return Err(CapstoneError::Other(format!("{} is not a number", field)));
    }
    Ok(value.clamp(min, max))
}

#[cfg(test)]
#[path = "share_tests.rs"]
mod tests;
