//! Repairing clipped peaks.
//!
//! What was clipped is gone: the job is to put back a plausible curve where the
//! encoder left a flat top, not to recover the original. Interpolating across
//! the run from the slope on either side is what makes a repaired transient
//! sound like a transient again rather than a click.
//!
//! Two things this is not. It does not remove the harmonic distortion already
//! baked into the file; it prevents *new* distortion downstream. And on a lossy
//! source it will correctly find nothing, because MP3 and AAC coding smears the
//! flat top away - such a file still clips, and is handled by gain and limiting
//! alone.

use crate::audio::DecodedAudio;
use crate::models::Template;

/// A run shorter than this is ordinary programme material touching full scale.
const MIN_RUN: usize = 3;

/// Flatness tolerance, as a fraction of full scale: two least-significant bits
/// of 16-bit, which is what a float decode of a clipped integer master lands
/// within.
const FLATNESS: f32 = 1e-4;

/// Runs longer than this are left alone. 20 ms of flat signal is almost always
/// a square-wave synth or a sub-bass drop, and a cubic across a gap that long
/// is fiction.
const MAX_RUN_SEC: f64 = 0.02;

/// The reconstruction is allowed to reach +6 dB. Past that the cubic is
/// extrapolating nonsense; the limiter handles whatever is left.
const CEILING: f64 = 2.0;

/// Repairs every clipped run and returns how many it fixed.
///
/// `template.clip_threshold_pct` is a percentage of full scale, 99.7 by
/// default. The reconstruction deliberately reaches *above* full scale - a run
/// that was 2 dB into the clip comes back about 2 dB over - and that is what
/// `template.headroom_db` makes room for later. Capping the repair at the
/// headroom instead would defeat the point of doing it: the chain order
/// repair, measure, gain, limit is what makes the overshoot safe.
///
/// The caller decides whether to run this at all: the template only enables
/// repair when the file's own average volume is louder than
/// `repair_above_lufs`, because a quiet file with flat tops was mastered that
/// way and is not damage to undo.
pub fn repair(audio: &mut DecodedAudio, template: &Template) -> u64 {
    let threshold = (template.clip_threshold_pct / 100.0).clamp(0.5, 1.0) as f32;
    let max_run = (MAX_RUN_SEC * audio.sample_rate.max(1) as f64).round() as usize;
    audio
        .channels
        .iter_mut()
        .map(|channel| repair_channel(channel, threshold, max_run.max(MIN_RUN)))
        .sum()
}

/// How much of the over-threshold region the flat top has to account for.
///
/// Flatness alone is not quite enough once the signal is sampled. A clean
/// 60 Hz sine touching full scale has three or four samples within `FLATNESS`
/// of its peak, because its curvature there is smaller than two bits of
/// 16-bit - it looks like a very short flat top. What separates the two cases
/// is how much of the over-threshold region that flat part occupies: a clipped
/// top fills nearly all of it (0.97 measured on a sine 0.5 dB over, 0.79 at
/// 0.04 dB over), while a smooth peak fills about 0.18 of it, because the
/// waveform spends far longer between the threshold and its peak than it does
/// at the peak.
const PLATEAU_SHARE_NUMERATOR: usize = 2;

/// Per channel independently. A run on one channel only is still a run; being
/// clever about stereo correlation buys nothing here.
fn repair_channel(samples: &mut [f32], threshold: f32, max_run: usize) -> u64 {
    let len = samples.len();
    let mut repaired = 0;
    let mut cursor = 0;
    while cursor < len {
        if samples[cursor].abs() < threshold {
            cursor += 1;
            continue;
        }
        let positive = samples[cursor] > 0.0;
        let mut region_end = cursor;
        while region_end < len
            && samples[region_end].abs() >= threshold
            && (samples[region_end] > 0.0) == positive
        {
            region_end += 1;
        }
        if let Some((offset, extent)) = plateau(&samples[cursor..region_end]) {
            let start = cursor + offset;
            let end = cursor + extent;
            // Anchors come from two samples either side, so a run touching
            // either edge of the buffer has nothing to interpolate from and is
            // left as it is rather than guessed at.
            //
            // These four conditions are the whole decision. What separates a
            // clipped top from a smooth peak that merely touched full scale is
            // the plateau share: a clipped top fills nearly all of its
            // over-threshold region, a smooth one about a fifth of it. The
            // reconstruction that follows is then unconditional, because by
            // this point the run *is* clipped and the only question left is
            // what curve to put back.
            if (MIN_RUN..=max_run).contains(&(end - start))
                && (end - start) * PLATEAU_SHARE_NUMERATOR >= region_end - cursor
                && start >= 2
                && end + 1 < len
            {
                reconstruct(samples, start, end, threshold);
                repaired += 1;
            }
        }
        cursor = region_end;
    }
    repaired
}

/// The longest contiguous stretch within `region` whose magnitude sits inside
/// `FLATNESS` of the region's own maximum - the flat top itself, with the
/// samples that merely crossed the threshold on the way up trimmed off.
///
/// Trimming matters more than it looks. At 110 Hz and 48 kHz a sine 0.5 dB into
/// the clip has about half a sample between the threshold and full scale, so
/// roughly half of all real clipped runs carry one transitional sample. Judging
/// flatness over the whole over-threshold region would throw those away.
fn plateau(region: &[f32]) -> Option<(usize, usize)> {
    let floor = region.iter().fold(0.0f32, |a, s| a.max(s.abs())) - FLATNESS;
    let mut best: Option<(usize, usize)> = None;
    let mut open = None;
    for (index, sample) in region.iter().enumerate() {
        if sample.abs() >= floor {
            open.get_or_insert(index);
        } else if let Some(start) = open.take() {
            best = longest(best, (start, index));
        }
    }
    if let Some(start) = open {
        best = longest(best, (start, region.len()));
    }
    best
}

fn longest(best: Option<(usize, usize)>, candidate: (usize, usize)) -> Option<(usize, usize)> {
    match best {
        Some(held) if held.1 - held.0 >= candidate.1 - candidate.0 => Some(held),
        _ => Some(candidate),
    }
}

/// Cubic Hermite across the gap with one-sided tangents scaled by the gap
/// width - the Catmull-Rom family. Measured against clipped sines it recovers
/// the original amplitude to within 0.05 dB up to 2 dB of overdrive and
/// degrades gracefully past that, for a fraction of the cost of a natural
/// spline through four anchors.
///
/// Short runs of three to eight samples are genuinely underdetermined: there is
/// almost no information about how high the original went and the answer
/// carries about half a decibel of spread. The methods that do materially
/// better solve a sparsity-constrained optimisation at a hundred to a thousand
/// times the cost. This is a floor, not a ceiling, and is not restoration.
fn reconstruct(samples: &mut [f32], start: usize, end: usize, threshold: f32) {
    let length = end - start;
    let p0 = samples[start - 2] as f64;
    let p1 = samples[start - 1] as f64;
    let p2 = samples[end] as f64;
    let p3 = samples[end + 1] as f64;
    let gap = (length + 1) as f64;
    let m1 = (p1 - p0) * gap;
    let m2 = (p3 - p2) * gap;
    let sign = if samples[start] > 0.0 { 1.0 } else { -1.0 };
    let rail = samples[start].abs() as f64;

    let mut fitted: Vec<f64> = (1..=length)
        .map(|step| hermite(step as f64 / gap, p1, m1, p2, m2))
        .collect();

    // Every sample inside the run had a true value at or above the rail, so a
    // fit that comes back under the clip threshold is wrong about this run by
    // construction - whether it merely sagged or went all the way across zero.
    if fitted.iter().any(|value| value * sign < threshold as f64) {
        fitted = arc(length, rail, edge_slope(p0, p1, p2, p3), sign);
    }

    for (offset, value) in fitted.iter().enumerate() {
        samples[start + offset] = value.clamp(-CEILING, CEILING) as f32;
    }
}

fn hermite(t: f64, p1: f64, m1: f64, p2: f64, m2: f64) -> f64 {
    let t2 = t * t;
    let t3 = t2 * t;
    (2.0 * t3 - 3.0 * t2 + 1.0) * p1
        + (t3 - 2.0 * t2 + t) * m1
        + (-2.0 * t3 + 3.0 * t2) * p2
        + (t3 - t2) * m2
}

/// Mean slope either side of the run, per sample. This is the rate at which the
/// waveform crossed the rail, which is what sets how far above it it went.
fn edge_slope(p0: f64, p1: f64, p2: f64, p3: f64) -> f64 {
    ((p1 - p0).abs() + (p2 - p3).abs()) / 2.0
}

/// The arc must not come back flat, or the repair would hand back something a
/// clip detector reads as a flat top all over again. Four thousandths of full
/// scale is 0.03 dB of curvature: inaudible, and two orders of magnitude over
/// the flatness tolerance either detector uses.
const MIN_ARC_HEIGHT: f64 = 4e-3;

/// A parabolic arc pinned to the rail at the run's edges.
///
/// This is the fallback when the Hermite fit comes back under the threshold,
/// which is the common case on a steep transient rather than a rare one: the
/// samples either side of a clipped peak sit far below the rail when the
/// waveform is moving quickly, and a Catmull-Rom curve drawn through them sags
/// under it. Measured on programme material clipped 14 dB into the rail, 521 of
/// 1,090 flat tops took this path, and the previous behaviour - declining to
/// repair them - left 48% of the file's clipping in place while reporting the
/// other 52% as fixed.
///
/// Near its peak a waveform is a parabola. If it crosses the rail `R` with
/// slope `s` and spends `W` samples above it, then `A - R = |s| * W / 4`. For a
/// 110 Hz sine at 48 kHz clipped 1.5 dB that gives 0.183 against a true 0.189,
/// inside 0.06 dB; at 0.5 and 2.0 dB of overdrive it is inside 0.05 dB as well,
/// which is the same accuracy the cubic reaches on the cases the cubic can
/// handle.
///
/// The curve is strictly above the rail everywhere inside the run and strictly
/// convex, so it can neither invent a polarity flip nor leave a plateau behind.
fn arc(length: usize, rail: f64, slope: f64, sign: f64) -> Vec<f64> {
    let width = (length + 1) as f64;
    let centre = width / 2.0;
    // `CEILING` has to bound the arc *before* it is built, not after. Clamping
    // a parabola that overshot replaces the flat top at the rail with a new
    // flat top at +6 dB, which is exactly what happened to 46 of 1,090
    // repaired runs on clipped programme material: every one came back at
    // 2.000000 across its whole length.
    //
    // The overshoot is not a slip in the arithmetic either. `A - R = |s|*W/4`
    // assumes the peak is resolved by the sampling, and these runs are three
    // to nine samples of near-Nyquist content at +14 dB, where consecutive
    // samples differ by more than full scale and the slope estimate means
    // nothing. There the honest position is the one the module already takes
    // for any over-long run: do not extrapolate, put back a plausible curve
    // and let the limiter deal with the rest.
    let head_room = (CEILING - rail).max(MIN_ARC_HEIGHT);
    let height = (slope.abs() * width / 4.0).clamp(MIN_ARC_HEIGHT, head_room);
    (1..=length)
        .map(|step| {
            let offset = (step as f64 - centre) / centre;
            sign * (rail + height * (1.0 - offset * offset))
        })
        .collect()
}

#[cfg(test)]
#[path = "clip_repair_tests.rs"]
mod tests;
