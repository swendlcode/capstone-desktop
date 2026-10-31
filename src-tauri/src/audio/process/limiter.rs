//! The output limiter: the last stage, and the one that decides whether a file
//! that hit its loudness target still sounds like music.
//!
//! Gain is never the last step. The order here is gain, then limit, and the
//! limiter is the only guarantee that nothing leaves above the ceiling.
//!
//! With a -12.5 LUFS target and a -1.5 dBTP ceiling the limiter only engages
//! once the source's peak-to-loudness ratio passes 11 dB. Modern dance masters
//! sit at 6 to 9 dB: they get turned down and the limiter never touches them,
//! which is most of a DJ library and has to come out clean. Vinyl rips, 80s and
//! 90s CDs and classical sit at 13 to 18 dB: they get turned up by 5 to 10 dB
//! and need 2 to 7 dB of peak limiting, which is where the character control
//! becomes audible.

use super::character;
use super::truepeak::{TruePeak, FLUSH, GROUP_DELAY};
use crate::audio::{db_to_linear, linear_to_db, DecodedAudio};

/// How far a sample may sit above the ceiling before the backstop counts it as
/// a real overshoot rather than a float rounding artefact. The clamp still
/// happens either way.
const FLOAT_SLACK: f32 = 1e-6;

/// How close to the target counts as arrived.
///
/// A twentieth of a decibel is an order of magnitude under anything audible
/// and an order of magnitude under the half-unit tolerance the product claims,
/// so stopping here cannot be the reason a file misses its target. It also has
/// to be comfortably above the loudness meter's own repeatability, or the
/// refinement below would keep nudging a file that had already arrived.
const TARGET_TOLERANCE_DB: f64 = 0.05;

/// Corrective passes after the first.
///
/// Each pass gives back about three quarters of what the one before it was
/// short by, because the gain it adds is itself partly limited away. Three is
/// therefore well past enough for any material: the worst case measured here,
/// a transient-heavy source 0.84 dB short after the first pass, is inside
/// 0.05 dB after two.
const MAX_REFINEMENTS: usize = 3;

/// Brings the buffer to `target_lufs` and holds it under `ceiling_db`.
///
/// `ceiling_db` is a ceiling in dBFS, so the caller passes the negative of the
/// template's headroom.
///
/// `character` is the Limiter Character control, 0-10, default 4. It is a
/// single dial over release behaviour and knee: at 0 the limiter is as
/// transparent and slow as it can be, at 10 it is fast and obvious. One control
/// rather than four is deliberate, since this is a batch tool for a DJ library
/// and not a mastering suite.
///
/// Gain and limiting are not one shot. Peak limiting removes loudness as well
/// as peaks, so a single gain move followed by a limiter lands *under* the
/// target by however much the limiter took - 0.84 LU on transient-heavy
/// material, which is past the half-unit the product promises, and the error
/// grows with the source's peak-to-loudness ratio, so it is exactly the vinyl
/// rips and older masters that miss by the most. The loop below measures what
/// is left and makes it up, which converges because the make-up gain is itself
/// subject to the same limiting.
///
/// What it must not do is make a file that needed no work louder than the
/// target: a buffer already on target and already under the ceiling comes out
/// bit identical, because the first pass applies no gain, the limiter is
/// transparent below its ceiling, and the second measurement finds nothing to
/// correct.
///
/// Returns the total gain applied, in dB, which is what the Volume column
/// shows after a run.
pub fn apply(audio: &mut DecodedAudio, target_lufs: f64, ceiling_db: f64, character: u8) -> f64 {
    if audio.is_empty() {
        return 0.0;
    }
    // Measured here and not taken from the caller's earlier reading: multiband
    // and warmth both move loudness by one to three decibels, so a gain
    // computed from the file as it arrived would miss the target by that much.
    //
    // `None` is silence, or something so short the gating has nothing to say.
    // Moving its level is not a decision this stage can make honestly, but the
    // ceiling still has to be held, so the limiter runs and no gain is given.
    let Some(measured) = super::integrated_lufs(audio) else {
        limit(audio, ceiling_db, character);
        return 0.0;
    };

    let mut applied_db = 0.0;
    let mut shortfall_db = target_lufs - measured;
    for pass in 0..=MAX_REFINEMENTS {
        // The first pass always runs, even with nothing to correct: a file
        // already on target can still have peaks over the ceiling, and that is
        // the one thing this stage guarantees.
        if pass > 0 && shortfall_db.abs() <= TARGET_TOLERANCE_DB {
            break;
        }
        if shortfall_db.abs() > 1e-9 {
            audio.apply_gain(db_to_linear(shortfall_db) as f32);
            applied_db += shortfall_db;
        }
        limit(audio, ceiling_db, character);
        // Re-limiting an already-limited buffer is free of charge: the
        // detector asks for no reduction below the ceiling, so only the gain
        // this pass added can produce any, which is what makes the sequence
        // contract rather than grind the file down.
        match super::integrated_lufs(audio) {
            Some(after) => shortfall_db = target_lufs - after,
            // It was measurable a moment ago, so this means the stages above
            // have taken it under the standard's absolute gate. Nothing
            // further can be said about its level.
            None => break,
        }
    }
    applied_db
}

/// The brickwall itself, without the gain move, and the number of samples the
/// backstop had to clamp.
///
/// Separate from `apply` because this is the part that has to be provably safe,
/// and a function that measures loudness cannot be swept over ten thousand
/// gain and ceiling combinations in a unit test.
pub(super) fn limit(audio: &mut DecodedAudio, ceiling_db: f64, character: u8) -> u64 {
    let frames = audio.frame_count();
    if frames == 0 {
        return 0;
    }
    let settings = character::lookup(character);
    let fs = audio.sample_rate.max(1) as f64;
    let ceiling = db_to_linear(ceiling_db);
    let required = required_gain(audio, frames, ceiling, settings.knee_db);
    let lookahead = ((settings.lookahead_ms * fs / 1000.0).round() as usize).clamp(1, frames);
    let mut envelope = envelope::attack(&required, lookahead);
    envelope::release(&mut envelope, &settings, fs);
    // The gain is linked across channels. Limiting each channel on its own
    // sidechain pumps the stereo image.
    let linear: Vec<f32> = envelope.iter().map(|log| log.exp() as f32).collect();
    for channel in audio.channels.iter_mut() {
        for (sample, gain) in channel.iter_mut().zip(&linear) {
            *sample *= gain;
        }
    }
    backstop(audio, frames, ceiling)
}

/// The gain each sample needs if the true peak around it is to land on the
/// ceiling, before any smoothing.
fn required_gain(audio: &DecodedAudio, frames: usize, ceiling: f64, knee_db: f64) -> Vec<f64> {
    let mut detectors: Vec<TruePeak> = audio
        .channels
        .iter()
        .map(|_| TruePeak::normalised())
        .collect();
    let mut interpolated = vec![0.0f64; frames + FLUSH];
    let mut sampled = vec![0.0f64; frames];
    for (index, slot) in interpolated.iter_mut().enumerate() {
        let mut peak = 0.0f64;
        for (channel, detector) in audio.channels.iter().zip(detectors.iter_mut()) {
            let sample = if index < frames {
                channel[index] as f64
            } else {
                0.0
            };
            if index < frames {
                sampled[index] = sampled[index].max(sample.abs());
            }
            peak = peak.max(detector.push(sample));
        }
        *slot = peak;
    }
    (0..frames)
        .map(|frame| {
            // The four phases describe the waveform between n-6 and n-5, so the
            // reading that covers sample n arrives at n+5 and n+6. Reading the
            // detector at n instead would apply the gain six samples late, and
            // those six samples would already have been multiplied by a larger
            // one: an overshoot path no amount of look-ahead closes.
            //
            // The sample's own magnitude joins the max because an interpolator
            // can read very slightly below it on an isolated spike, and a true
            // peak below the sample peak is not physically possible.
            let detected = sampled[frame]
                .max(interpolated[frame + GROUP_DELAY - 1])
                .max(interpolated[frame + GROUP_DELAY]);
            knee_gain(detected, ceiling, knee_db)
        })
        .collect()
}

/// Quadratic soft knee at infinite ratio, in the log domain.
///
/// The internal ceiling is deliberately *not* biased by half the knee width.
/// Taking `f(e) = e - (e + W/2)^2 / 2W`, `f'(e) = 0` at `e = W/2` where
/// `f = 0`: the output never exceeds the ceiling and touches it exactly at the
/// upper corner of the knee. Biasing the ceiling to compensate is the classic
/// reflex here and it just makes every file quieter than the target.
fn knee_gain(detected: f64, ceiling: f64, knee_db: f64) -> f64 {
    if detected <= 0.0 {
        return 1.0;
    }
    let excess = linear_to_db(detected) - linear_to_db(ceiling);
    let half = knee_db / 2.0;
    let reduction = if excess >= half {
        excess
    } else if excess <= -half {
        0.0
    } else {
        (excess + half).powi(2) / (2.0 * knee_db)
    };
    db_to_linear(-reduction)
}

/// Final hard clip at the ceiling, counting the samples it had to move. A
/// non-zero count is a bug in the envelope, not a feature, which is why it is
/// counted rather than quietly applied.
fn backstop(audio: &mut DecodedAudio, frames: usize, ceiling: f64) -> u64 {
    let limit = ceiling as f32;
    let mut touched = 0;
    for channel in audio.channels.iter_mut() {
        for sample in channel[..frames].iter_mut() {
            if sample.abs() > limit {
                if sample.abs() > limit * (1.0 + FLOAT_SLACK) {
                    touched += 1;
                }
                *sample = sample.signum() * limit;
            }
        }
    }
    touched
}

// The envelope is where the brickwall guarantee actually lives, so its four
// functions and their proofs sit together in one file of their own.
#[path = "limiter_envelope.rs"]
mod envelope;

#[cfg(test)]
#[path = "limiter_tests.rs"]
mod tests;
