//! Deviation from concert pitch, in cents.
//!
//! The question is not "what note is this" but "is this whole record sitting
//! off A440", which is the common state of a vinyl rip, a tape transfer or
//! anything recorded before a studio owned a tuner. So every strong partial in
//! the file votes: each one is folded onto the nearest equal-tempered semitone,
//! its distance from that semitone goes into a one-cent histogram weighted by
//! how loud it was, and the centre of that histogram is the answer. One bent
//! note cannot move it; a tape running fast moves all of it.
//!
//! Folding to the nearest semitone carries an ambiguity that no amount of
//! cleverness removes: a record 60 cents sharp is indistinguishable from one 40
//! cents flat. The answer is therefore bounded to +/-50 cents and always points
//! the shorter way, which is also the musically right call, since 60 cents is a
//! different key rather than a mistuning.

use std::f64::consts::TAU;

use rustfft::{num_complex::Complex, FftPlanner};

use super::spectrum;
use crate::audio::DecodedAudio;

/// Frames quieter than this hold no pitch worth reading.
const SILENCE_DBFS: f64 = -50.0;

/// Spectral flatness above which a frame is noise or a drum hit. A kick carries
/// no tuning information; letting it vote only flattens the histogram.
const MAX_FLATNESS: f64 = 0.4;

/// How far below the frame's loudest bin a peak may sit and still vote.
const PEAK_FLOOR_DB: f64 = -30.0;

/// A peak also has to stand this far above the local noise floor, measured as
/// the median of its own stretch of spectrum.
const MEDIAN_MULTIPLE: f64 = 3.0;

/// Fewer usable frames than this and the file does not get an answer. A reading
/// from a handful of frames is a guess, and the Pitch column shows nothing
/// rather than a number nobody should act on.
const MIN_FRAMES: usize = 50;

/// How concentrated the histogram has to be, 0 (no tuning at all) to 1
/// (perfectly in tune with itself).
const MIN_CONFIDENCE: f64 = 0.15;

/// One histogram bin per cent, covering the fold interval [-50, +50).
const BINS: usize = 100;

/// Signed cents off concert pitch, or exactly 0.0 when the file cannot be
/// judged: too short, too percussive, or not tuned to anything.
pub fn estimate(audio: &DecodedAudio) -> f64 {
    let rate = audio.sample_rate as f64;
    if rate <= 0.0 {
        return 0.0;
    }

    let mono = spectrum::mono_sum(audio);
    let window = spectrum::window_len(rate);
    if mono.len() < window {
        return 0.0;
    }

    let (histogram, usable) = gather(&mono, window, rate);
    if usable < MIN_FRAMES {
        return 0.0;
    }

    let (deviation, confidence) = circular_centre(&histogram);
    if confidence < MIN_CONFIDENCE {
        return 0.0;
    }
    deviation
}

/// Walks the file, frame by frame, and returns the filled histogram together
/// with the number of frames that had something to say.
fn gather(mono: &[f64], window: usize, rate: f64) -> ([f64; BINS], usize) {
    // Zero-padding to twice the window adds no resolution, but it puts more
    // points on the same spectral peak, which is what the parabolic refinement
    // reads to place that peak between bins.
    let fft_len = window * 2;
    let hop = spectrum::hop_len(mono.len(), window);
    let taper = spectrum::hann(window);

    let fft = FftPlanner::<f64>::new().plan_fft_forward(fft_len);
    let mut scratch = vec![Complex::default(); fft.get_inplace_scratch_len()];
    let mut buffer = vec![Complex::default(); fft_len];
    let mut magnitudes = vec![0.0f64; fft_len / 2 + 1];

    let band = spectrum::band_bins(rate, fft_len);
    let mut histogram = [0.0f64; BINS];
    let mut usable = 0usize;

    let mut start = 0;
    while start + window <= mono.len() {
        let frame = &mono[start..start + window];
        start += hop;

        if spectrum::frame_dbfs(frame) < SILENCE_DBFS {
            continue;
        }

        for (slot, (sample, weight)) in buffer.iter_mut().zip(frame.iter().zip(&taper)) {
            *slot = Complex::new(sample * weight, 0.0);
        }
        buffer[window..].fill(Complex::default());
        fft.process_with_scratch(&mut buffer, &mut scratch);
        for (bin, slot) in magnitudes.iter_mut().enumerate() {
            *slot = buffer[bin].norm();
        }

        if spectrum::flatness(&magnitudes[band.0..=band.1]) > MAX_FLATNESS {
            continue;
        }

        usable += 1;
        vote(&magnitudes, band, rate, fft_len, &mut histogram);
    }

    (histogram, usable)
}

/// Every strong partial in one frame, folded into the histogram and weighted by
/// its own magnitude, so a loud fundamental outvotes a faint harmonic.
fn vote(
    magnitudes: &[f64],
    band: (usize, usize),
    rate: f64,
    fft_len: usize,
    histogram: &mut [f64; BINS],
) {
    let (low_bin, high_bin) = band;
    let in_band = &magnitudes[low_bin..=high_bin];
    let loudest = in_band.iter().copied().fold(0.0f64, f64::max);
    if loudest <= 0.0 {
        return;
    }
    let floor = loudest * 10f64.powf(PEAK_FLOOR_DB / 20.0);
    let medians = spectrum::segment_medians(in_band);

    for bin in (low_bin + 1)..high_bin {
        let magnitude = magnitudes[bin];
        if magnitude < floor || magnitude <= magnitudes[bin - 1] || magnitude < magnitudes[bin + 1]
        {
            continue;
        }
        let segment = ((bin - low_bin) / spectrum::SEGMENT_BINS).min(medians.len() - 1);
        if magnitude < MEDIAN_MULTIPLE * medians[segment] {
            continue;
        }

        let offset = spectrum::refine(magnitudes[bin - 1], magnitude, magnitudes[bin + 1]);
        let frequency = (bin as f64 + offset) * rate / fft_len as f64;
        if frequency > 0.0 {
            histogram[fold(frequency)] += magnitude;
        }
    }
}

/// Histogram bin for one frequency's distance from the nearest semitone.
fn fold(frequency: f64) -> usize {
    let midi = 12.0 * (frequency / 440.0).log2() + 69.0;
    let cents = (midi - midi.round()) * 100.0;
    ((cents.round() as i64) + 50).rem_euclid(BINS as i64) as usize
}

/// Centre of the histogram, treated as the circle it is.
///
/// -50 and +50 cents are the same place, so a linear mean, or an argmax, splits
/// any peak that straddles the wrap and answers 0 for a record that is 49 cents
/// out. The resultant vector has no such seam, and its length doubles as the
/// confidence: all the weight in one place gives 1, weight spread evenly round
/// the circle gives 0.
///
/// The published method smooths the histogram with a 3-cent Gaussian first.
/// That step is left out here because it cannot change this answer: a circular
/// convolution with a symmetric kernel scales the resultant by a positive real
/// number (0.982 at sigma 3) and cannot rotate it. It matters for an argmax,
/// which is what the method uses it with.
///
/// Returns (deviation in cents, confidence 0..1).
fn circular_centre(histogram: &[f64; BINS]) -> (f64, f64) {
    let total: f64 = histogram.iter().sum();
    if total <= 0.0 {
        return (0.0, 0.0);
    }

    let (mut real, mut imaginary) = (0.0f64, 0.0f64);
    for (bin, weight) in histogram.iter().enumerate() {
        let theta = TAU * (bin as f64 - 50.0) / BINS as f64;
        real += weight * theta.cos();
        imaginary += weight * theta.sin();
    }

    let deviation = BINS as f64 * imaginary.atan2(real) / TAU;
    let confidence = (real * real + imaginary * imaginary).sqrt() / total;
    (deviation, confidence)
}
