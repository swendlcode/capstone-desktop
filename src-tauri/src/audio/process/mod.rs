//! The processing chain, in the order the stages have to run.

mod biquad;
mod character;
pub mod clip_repair;
pub mod limiter;
pub mod multiband;
mod oversample;
pub mod pitch;
#[cfg(test)]
mod testkit;
mod truepeak;
pub mod warmth;

use crate::audio::DecodedAudio;
use crate::error::Result;
use crate::models::{ProcessResult, Template};

/// What a file with nothing measurable in it is reported as, and the point
/// below which this chain declines to move a file's level at all.
///
/// It is the absolute gate of the loudness standard rather than an invented
/// number. Without the floor, a file 80 dB down would ask for 67 dB of gain and
/// get its own noise floor normalised to dance-floor level.
const SILENCE_LUFS: f64 = -70.0;

/// Runs the whole chain in place and reports what changed.
///
/// Order is not a style choice. Clipped peaks are repaired first, while the
/// damage is still where the encoder left it; pitch correction comes before
/// anything that reshapes level, because resampling and time-stretching both
/// change peak behaviour; multiband and warmth shape the sound; the gain move
/// and the limiter come last, so the file lands on its target loudness after
/// every stage that could have moved it.
///
/// `output_path` in the returned `ProcessResult` is left empty: this function
/// never touches the filesystem, and `jobs` fills the path in once it has
/// encoded the buffer. Keeping the write out of here is what lets the chain be
/// tested on a buffer with no disk involved.
pub fn process(audio: &mut DecodedAudio, template: &Template) -> Result<ProcessResult> {
    let lufs_before = integrated_lufs(audio).unwrap_or(SILENCE_LUFS);

    // Clipped peaks first. Every stage after this one filters or scales the
    // signal, and each of them destroys the flat tops that are the only
    // evidence the file was ever clipped.
    let clips_repaired = if template.repair_clips && lufs_before > template.repair_above_lufs {
        clip_repair::repair(audio, template)
    } else {
        0
    };

    // Pitch before anything that measures level: `ChangeTempo` alters the
    // duration, and the gating blocks the loudness figure is built from are
    // counted off the duration.
    let pitch_corrected_cents = if template.correct_pitch {
        correct_tuning(audio, template)?
    } else {
        0.0
    };

    multiband::apply(audio, &template.bands);
    warmth::apply(audio, template.warmth);

    // The gain move and the limiter last, and the limiter measures loudness for
    // itself because the two stages above just moved it by one to three
    // decibels. `headroom_db` is a distance below full scale; the limiter wants
    // a ceiling, so it is passed negated.
    let gain_applied_db = limiter::apply(
        audio,
        template.target_lufs,
        -template.headroom_db,
        template.limiter_character,
    );

    Ok(ProcessResult {
        output_path: String::new(),
        lufs_before,
        lufs_after: integrated_lufs(audio).unwrap_or(SILENCE_LUFS),
        clips_repaired,
        gain_applied_db,
        pitch_corrected_cents,
    })
}

/// Measures the file's deviation from concert pitch and corrects it if the
/// template's tolerance says it is worth doing.
///
/// Measured here rather than reused from the row's own analysis, because clip
/// repair has already run by this point and because `ChangeTempo` would
/// invalidate an earlier reading anyway.
fn correct_tuning(audio: &mut DecodedAudio, template: &Template) -> Result<f64> {
    let deviation = crate::audio::analyze::tuning_cents(audio);
    if !deviation.is_finite() || deviation.abs() < template.pitch_tolerance_cents {
        return Ok(0.0);
    }
    pitch::correct(audio, deviation, template.pitch_mode)
}

/// Integrated gated loudness, through `analyze::loudness` rather than a meter
/// of this module's own, so the figure the chain aims at and the figure the
/// table shows can never disagree about the same file.
///
/// `None` means there is nothing worth moving: the meter failed, or the file
/// sits below the standard's absolute gate, where normalising would do nothing
/// but bring the file's own noise floor up to dance-floor level.
///
/// The chain asks before the stages, inside the limiter for the gain move and
/// once per corrective pass after it, and once more at the end, because each
/// reading has to describe the signal as it is at that point. A reading is
/// under a second on a five-minute track, against seconds for the decode and
/// seconds more for the warmth oversampler, and a stale figure would miss the
/// target by whatever the stages in between did. A file that lands on target
/// at the first attempt - most of a DJ library, since the limiter only engages
/// past 11 dB of peak-to-loudness - pays for one extra reading and no extra
/// passes.
fn integrated_lufs(audio: &DecodedAudio) -> Option<f64> {
    if audio.is_empty() {
        return None;
    }
    crate::audio::analyze::loudness(audio)
        .ok()
        .map(|(lufs, _true_peak)| lufs)
        .filter(|value| value.is_finite() && *value > SILENCE_LUFS)
}

#[cfg(test)]
#[path = "chain_tests.rs"]
mod tests;
