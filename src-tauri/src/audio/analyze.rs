//! Analysis: what the Clips, Dynamics, Volume and Pitch columns show before
//! anything has been changed.

use ebur128::{EbuR128, Mode};

use crate::audio::{linear_to_db, DecodedAudio};
use crate::error::{CapstoneError, Result};
use crate::models::Analysis;

// Tuning is the one measure here with real algorithmic choices to argue about,
// so the estimator and the spectral helpers under it get their own files.
// `#[path]` because this module is `audio/analyze.rs` while its parts live
// under `audio/analysis/`.
#[path = "analysis/spectrum.rs"]
mod spectrum;
#[path = "analysis/tuning.rs"]
mod tuning;

#[cfg(test)]
#[path = "analysis/tests.rs"]
mod tests;

/// Fraction of full scale a sample has to reach to count toward a clip run.
/// Kept here rather than read from the template because this is measurement:
/// the template's `clip_threshold_pct` decides what gets *repaired*, and the
/// two drifting apart would make the Clips column disagree with the repair
/// count for no reason the user could see.
pub const CLIP_CEILING: f32 = 0.9999;

/// A run has to be this long to be a clip. A single sample at full scale is
/// ordinary; three consecutive ones is a flat top.
pub const MIN_CLIP_RUN: usize = 3;

/// How much a run is allowed to wobble and still count as flat, linear.
///
/// Run length on its own is not a clip detector: the time a waveform naturally
/// spends above the ceiling depends on its frequency, so a full-scale 60 Hz
/// sine dwells there for three or four samples at 44.1 kHz and would be counted
/// as clipped. What identifies clipping is the *flat top*, which is why the
/// spread across the run is tested as well.
///
/// The value has to be well inside the gap between the ceiling and full scale,
/// and that is the whole reason it is derived from the ceiling rather than
/// written down. A delta as large as the gap itself passes everything: a sine
/// that just touches full scale cannot vary by more than the gap while it is
/// above the ceiling, so it reads as flat. At a quarter of the gap a sweep of
/// full-scale sines from 20 Hz to 1 kHz, at sixteen phases each, produces no
/// runs at all, while a clipped master dithered by a 16-bit LSB still reports
/// every one of its 2,000 flat tops.
const FLAT_TOP_DELTA: f32 = (1.0 - CLIP_CEILING) / 4.0;

/// Frames handed to the loudness meter per call.
///
/// The meter wants interleaved samples and the buffer is planar, so something
/// has to be copied. Doing it in blocks keeps that copy at a few hundred
/// kilobytes: interleaving the whole file at once would hold a second copy of
/// it, which is 200 MB for ten minutes of 44.1 kHz stereo, per worker, and the
/// Processor Load setting lets several workers run at once.
const METER_CHUNK_FRAMES: usize = 8192;

/// Everything the table needs from one file.
///
/// Four measures over the same decoded buffer. None of them changes it, so the
/// order is free and the loudness meter is the only one that costs anything
/// worth noticing.
pub fn analyze(audio: &DecodedAudio) -> Result<Analysis> {
    if audio.is_empty() {
        return Err(CapstoneError::Unsupported(
            "file decoded to no audio frames".to_string(),
        ));
    }

    let (lufs, true_peak_db) = loudness(audio)?;
    Ok(Analysis {
        duration_sec: audio.duration_sec(),
        sample_rate: audio.sample_rate,
        channels: audio.channel_count(),
        lufs,
        true_peak_db,
        clip_count: clip_count(audio),
        crest_db: crest_db(audio),
        tuning_cents: tuning_cents(audio),
    })
}

/// Integrated loudness and true peak, EBU R128.
///
/// Construct with `EbuR128::new(channels, rate, Mode::I | Mode::TRUE_PEAK | Mode::LRA)` and
/// feed `add_frames_f32` on interleaved data. The modes have to be asked for at
/// construction: calling `loudness_range()` without `Mode::LRA` returns an
/// error rather than a number.
///
/// `Mode::LRA` is deliberately *not* asked for: `Analysis` has nowhere to put a
/// loudness range, and the mode carries a second 3-second history the meter
/// would otherwise keep for every file in the batch. Add it here the day the
/// table grows a column for it.
///
/// Returns (integrated LUFS, true peak dBTP).
pub fn loudness(audio: &DecodedAudio) -> Result<(f64, f64)> {
    let channels = audio.channel_count().max(1) as u32;
    let mut meter = EbuR128::new(channels, audio.sample_rate, Mode::I | Mode::TRUE_PEAK)
        .map_err(|e| CapstoneError::Other(format!("loudness meter: {e}")))?;

    let frames = audio.frame_count();
    let mut block: Vec<f32> = Vec::with_capacity(METER_CHUNK_FRAMES * channels as usize);
    let mut start = 0;
    while start < frames {
        let end = (start + METER_CHUNK_FRAMES).min(frames);
        block.clear();
        for frame in start..end {
            for channel in &audio.channels {
                block.push(channel[frame]);
            }
        }
        meter
            .add_frames_f32(&block)
            .map_err(|e| CapstoneError::Other(format!("loudness meter: {e}")))?;
        start = end;
    }

    // Gating leaves a file shorter than one 400 ms block with no block to
    // average, and the meter answers -inf. Plain RMS stands in: K-weighting is
    // calibrated so a 1 kHz sine reads its own RMS level, so the two agree
    // within about a dB on music, and a number that close beats a row showing
    // minus infinity.
    let integrated = meter
        .loudness_global()
        .map_err(|e| CapstoneError::Other(format!("loudness meter: {e}")))?;
    let lufs = if integrated.is_finite() {
        integrated
    } else {
        linear_to_db(rms(audio))
    };

    let mut peak = 0.0f64;
    for channel in 0..channels {
        let channel_peak = meter
            .true_peak(channel)
            .map_err(|e| CapstoneError::Other(format!("true peak: {e}")))?;
        peak = peak.max(channel_peak);
    }

    Ok((lufs, linear_to_db(peak)))
}

/// Counts runs of at least `MIN_CLIP_RUN` consecutive samples at or over
/// `CLIP_CEILING`, per channel. A run is one clip however long it is, which is
/// what makes the number mean "how many transients were flattened" rather than
/// "how many samples are loud".
///
/// Worth saying out loud because it will be reported as a bug: an MP3 or AAC
/// source that audibly clips reports **zero** clips. Lossy coding smears the
/// flat top, so there is nothing flat left to find. Those files are still
/// handled, by gain and the limiter, and their true peak over 0 dBFS is the
/// figure that gives them away.
pub fn clip_count(audio: &DecodedAudio) -> u64 {
    audio.channels.iter().map(|c| channel_clip_runs(c)).sum()
}

fn channel_clip_runs(samples: &[f32]) -> u64 {
    let mut runs = 0u64;
    let mut index = 0;
    while index < samples.len() {
        let first = samples[index];
        if first.abs() < CLIP_CEILING {
            index += 1;
            continue;
        }

        // A run is same-sign by definition: a clipped top is one side of the
        // waveform held flat, and two consecutive full-scale samples of
        // opposite sign are a different event entirely.
        let negative = first.is_sign_negative();
        let (mut low, mut high) = (first.abs(), first.abs());
        let mut end = index + 1;
        while end < samples.len()
            && samples[end].abs() >= CLIP_CEILING
            && samples[end].is_sign_negative() == negative
        {
            low = low.min(samples[end].abs());
            high = high.max(samples[end].abs());
            end += 1;
        }

        if end - index >= MIN_CLIP_RUN && high - low <= FLAT_TOP_DELTA {
            runs += 1;
        }
        index = end;
    }
    runs
}

/// Peak-to-RMS in dB, over the whole file. This is the Dynamics column: a
/// heavily limited master lands near 6 dB, something untouched nearer 18.
pub fn crest_db(audio: &DecodedAudio) -> f64 {
    let rms = rms(audio);
    if rms <= 0.0 {
        return 0.0;
    }
    linear_to_db(audio.peak() as f64 / rms)
}

/// Linear RMS over every sample of every channel.
fn rms(audio: &DecodedAudio) -> f64 {
    let mut sum = 0.0f64;
    let mut count = 0usize;
    for channel in &audio.channels {
        for sample in channel {
            sum += (*sample as f64) * (*sample as f64);
            count += 1;
        }
    }
    if count == 0 {
        return 0.0;
    }
    (sum / count as f64).sqrt()
}

/// How far the recording sits from concert pitch, in cents, signed.
///
/// The point of this is vinyl rips and tape, where the whole file is off by a
/// constant amount. So: take the strongest partials across several windows with
/// rustfft, fold each one's frequency onto the nearest equal-tempered semitone,
/// and take the weighted centre of those deviations. Averaging over the file is
/// what separates a mastering drift from a single bent note.
///
/// Returns exactly 0.0 when the material cannot be judged — too short, too
/// percussive, or simply not tuned to anything. See `analysis/tuning.rs` for
/// what each of those means and why the answer is bounded to +/-50 cents.
pub fn tuning_cents(audio: &DecodedAudio) -> f64 {
    tuning::estimate(audio)
}
