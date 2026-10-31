//! The templates that ship with the app.
//!
//! Stock templates are code rather than data (see the parent module), so this
//! file is the whole shipped set. Five situations, and the numbers are derived
//! from each one rather than nudged off Festival: the whole point of a template
//! is that picking the right one means not touching a slider.
//!
//! The one number that ties them together is where the limiter starts working.
//! A target of `T` LUFS under a `headroom_db` of `H` puts the ceiling at `-H`
//! dBTP, so limiting begins once a source's peak-to-loudness ratio passes
//! `|T| - H` dB. Modern dance masters sit at 6 to 9 dB, vinyl rips and older
//! CDs at 13 to 18 (see "Why the limiter has a true-peak sidechain of its own"
//! in CLAUDE.md). Each template below picks its pair so that the material it is
//! for lands on target without the limiter doing the work.

use crate::models::{BandSetting, PitchMode, Template, Warmth};

pub const FESTIVAL_ID: &str = "stock-festival";
pub const CLUB_ID: &str = "stock-club";
pub const STREAM_ID: &str = "stock-stream";
pub const VINYL_ID: &str = "stock-vinyl";
pub const ARCHIVE_ID: &str = "stock-archive";

impl Default for BandSetting {
    fn default() -> Self {
        Self {
            low_hz: 20.0,
            high_hz: 20_000.0,
            threshold_db: -18.0,
            ratio: 2.0,
            attack_ms: 10.0,
            release_ms: 120.0,
            makeup_db: 0.0,
        }
    }
}

/// Festival, the shipped default. `Default` rather than a free function because
/// `Template` is deserialized with a container-level serde default: a field
/// added in a later release fills in from here instead of failing the whole
/// file.
impl Default for Template {
    fn default() -> Self {
        festival()
    }
}

/// One row per band: the band's upper edge, then threshold, ratio, attack and
/// release. Lower edges chain upward from 20 Hz, so the crossovers meet exactly
/// by construction — `multiband` reads each `high_hz` as the next band's
/// `low_hz` without checking, and a typo in a literal would be a notch at that
/// edge rather than a parse error.
type BandRow = (f32, f32, f32, f32, f32);

fn tile(rows: [BandRow; 4]) -> Vec<BandSetting> {
    let mut low = 20.0;
    rows.into_iter()
        .map(|(high_hz, threshold_db, ratio, attack_ms, release_ms)| {
            let band = BandSetting {
                low_hz: low,
                high_hz,
                threshold_db,
                ratio,
                attack_ms,
                release_ms,
                makeup_db: 0.0,
            };
            low = high_hz;
            band
        })
        .collect()
}

/// A four-way split: sub/low, body, presence, air.
pub fn default_bands() -> Vec<BandSetting> {
    tile([
        (120.0, -16.0, 2.5, 20.0, 200.0),
        (800.0, -18.0, 2.0, 12.0, 150.0),
        (5_000.0, -20.0, 1.8, 6.0, 100.0),
        (20_000.0, -22.0, 1.6, 3.0, 80.0),
    ])
}

/// Big outdoor rig. Loud, controlled, and the shipped default.
pub fn festival() -> Template {
    Template {
        id: FESTIVAL_ID.to_string(),
        name: "Festival".to_string(),
        is_stock: true,
        target_lufs: -12.5,
        limiter_character: 4,
        repair_clips: true,
        repair_above_lufs: -14.0,
        clip_threshold_pct: 99.7,
        headroom_db: 1.5,
        warmth: Warmth::Gentle,
        correct_pitch: false,
        pitch_tolerance_cents: 10.0,
        pitch_mode: PitchMode::KeepTempo,
        bands: default_bands(),
    }
}

/// A club room, which is smaller and far more sub-heavy than a field: the low
/// band is compressed harder and slower so one over-long 50 Hz tail cannot eat
/// the amplifier's headroom, and the character is up at 6 for the glue a dense
/// room wants. Hotter than Festival by 1.5 dB with half a decibel less ceiling,
/// so limiting starts at 10 dB of peak-to-loudness instead of 11.
pub fn club() -> Template {
    Template {
        id: CLUB_ID.to_string(),
        name: "Club".to_string(),
        target_lufs: -11.0,
        limiter_character: 6,
        clip_threshold_pct: 99.5,
        headroom_db: 1.0,
        bands: tile([
            (120.0, -14.0, 3.0, 25.0, 220.0),
            (800.0, -16.0, 2.2, 12.0, 150.0),
            (5_000.0, -18.0, 1.8, 6.0, 100.0),
            (20_000.0, -20.0, 1.6, 3.0, 80.0),
        ]),
        ..festival()
    }
}

/// A submission, not a set. Every streaming platform normalises to about
/// -14 LUFS and asks for a -1 dBTP ceiling, so anything louder is turned back
/// down at the far end and the only thing the extra level bought was limiting.
/// Warmth is off because colour is not something to add to a file somebody else
/// is going to master around, and the bands barely move: at 13 dB of
/// peak-to-loudness before the limiter engages, most masters pass clean.
pub fn stream() -> Template {
    Template {
        id: STREAM_ID.to_string(),
        name: "Stream".to_string(),
        target_lufs: -14.0,
        limiter_character: 2,
        repair_above_lufs: -16.0,
        headroom_db: 1.0,
        warmth: Warmth::None,
        bands: tile([
            (120.0, -20.0, 1.6, 25.0, 250.0),
            (800.0, -22.0, 1.4, 15.0, 180.0),
            (5_000.0, -24.0, 1.3, 8.0, 120.0),
            (20_000.0, -26.0, 1.2, 4.0, 90.0),
        ]),
        ..festival()
    }
}

/// A record played back on a deck that was not quite on speed. This is the one
/// template with pitch correction **on**, and the one that uses `change_tempo`:
/// a slow platter stretches pitch and time together, so resampling undoes both
/// and shifting pitch alone would leave the timing wrong. The low band tames
/// turntable rumble; everything above it is near enough inert, because a rip
/// has 13 to 18 dB of peak-to-loudness and the job is to keep it.
pub fn vinyl_rip() -> Template {
    Template {
        id: VINYL_ID.to_string(),
        name: "Vinyl Rip".to_string(),
        target_lufs: -16.0,
        limiter_character: 2,
        clip_threshold_pct: 99.5,
        warmth: Warmth::None,
        correct_pitch: true,
        pitch_tolerance_cents: 5.0,
        pitch_mode: PitchMode::ChangeTempo,
        bands: tile([
            (120.0, -18.0, 2.0, 30.0, 250.0),
            (800.0, -24.0, 1.3, 15.0, 180.0),
            (5_000.0, -26.0, 1.2, 8.0, 120.0),
            (20_000.0, -28.0, 1.2, 4.0, 90.0),
        ]),
        ..festival()
    }
}

/// Repair and nothing else, for a master going into an archive or on to someone
/// else's desk. Every band sits at 1.0:1, which is how the multiband is turned
/// off without a switch for it — the crossovers still sum flat, and no band can
/// apply gain reduction. Character 0 is the fastest and most transparent
/// setting, and the 2 dB ceiling is deliberately the widest of the five: clip
/// repair reaches above full scale on purpose and `headroom_db` is what pays
/// for that overshoot, so the one stage this template does run gets the most
/// room to work in.
pub fn archive() -> Template {
    Template {
        id: ARCHIVE_ID.to_string(),
        name: "Archive".to_string(),
        target_lufs: -16.0,
        limiter_character: 0,
        repair_above_lufs: -24.0,
        clip_threshold_pct: 99.5,
        headroom_db: 2.0,
        warmth: Warmth::None,
        bands: tile([
            (120.0, 0.0, 1.0, 20.0, 200.0),
            (800.0, 0.0, 1.0, 12.0, 150.0),
            (5_000.0, 0.0, 1.0, 6.0, 100.0),
            (20_000.0, 0.0, 1.0, 3.0, 80.0),
        ]),
        ..festival()
    }
}

/// Every template that ships with the app. Festival stays first: it is the
/// fallback `AppState::active_template` reaches for, and the picker shows the
/// list in this order.
pub fn stock() -> Vec<Template> {
    vec![festival(), club(), stream(), vinyl_rip(), archive()]
}
