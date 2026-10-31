//! Framing and spectral measurement, shared by the tuning estimator.
//!
//! Nothing here decides anything about a file; it prepares frames and measures
//! single spectra. The judgements live in `tuning.rs`.

use std::f64::consts::TAU;

use crate::audio::DecodedAudio;

/// Analysis window, in seconds. 93 ms is 4096 samples at 44.1 kHz: long enough
/// that a semitone is many bins wide, short enough to sit inside one note.
const WINDOW_SEC: f64 = 0.093;

/// Analysis band. Below 80 Hz the bin grid is too coarse against a semitone;
/// above 5 kHz cymbals and high harmonics outnumber anything fundamental.
const MIN_HZ: f64 = 80.0;
const MAX_HZ: f64 = 5000.0;

/// Bins per median segment. One median per segment rather than per bin: a sort
/// of 64 values is cheap, and spectral tilt across 64 bins is small enough that
/// the two answers agree where it matters.
pub const SEGMENT_BINS: usize = 64;

/// Frames actually transformed, however long the file is.
///
/// Accuracy comes from averaging over the whole file, not from transforming
/// every hop of it: at 75% overlap a ten-minute track is 25,000 frames, and the
/// deviation it reports is the same as the one 600 frames spread across those
/// same ten minutes report. So the hop is stretched to cover the file in at most
/// this many frames, which holds a long file to a fraction of a second while a
/// short one is still analysed hop by hop.
const MAX_FRAMES: usize = 600;

/// Mono sum. Tuning is a property of the record, not of one side of it, and a
/// partial that is strong in only one channel still votes with its own weight.
pub fn mono_sum(audio: &DecodedAudio) -> Vec<f64> {
    let frames = audio.frame_count();
    let channels = audio.channels.len().max(1) as f64;
    (0..frames)
        .map(|frame| {
            audio
                .channels
                .iter()
                .map(|channel| channel[frame] as f64)
                .sum::<f64>()
                / channels
        })
        .collect()
}

pub fn window_len(rate: f64) -> usize {
    let target = (WINDOW_SEC * rate).round().max(1.0) as usize;
    target.next_power_of_two().clamp(2048, 16384)
}

/// 75% overlap, stretched so that a long file is still covered in at most
/// `MAX_FRAMES` frames.
pub fn hop_len(samples: usize, window: usize) -> usize {
    let base = (window / 4).max(1);
    let available = samples.saturating_sub(window) / base + 1;
    base * available.div_ceil(MAX_FRAMES).max(1)
}

pub fn hann(len: usize) -> Vec<f64> {
    (0..len)
        .map(|n| 0.5 - 0.5 * (TAU * n as f64 / len as f64).cos())
        .collect()
}

pub fn frame_dbfs(frame: &[f64]) -> f64 {
    if frame.is_empty() {
        return f64::NEG_INFINITY;
    }
    let mean_square = frame.iter().map(|s| s * s).sum::<f64>() / frame.len() as f64;
    if mean_square <= 0.0 {
        return f64::NEG_INFINITY;
    }
    10.0 * mean_square.log10()
}

/// First and last bin of the analysis band, inclusive, both inside the
/// half-spectrum so a caller can always look one bin either side.
pub fn band_bins(rate: f64, fft_len: usize) -> (usize, usize) {
    let per_bin = rate / fft_len as f64;
    let last = (fft_len / 2).saturating_sub(1);
    let low = ((MIN_HZ / per_bin).floor() as usize).max(1);
    let high = ((MAX_HZ / per_bin).ceil() as usize).min(last);
    (low.min(high), high.max(low))
}

/// Geometric mean over arithmetic mean of the power spectrum: near 0 for a pure
/// tone, near 1 for white noise. This is what tells a drum hit from a note.
pub fn flatness(band: &[f64]) -> f64 {
    const FLOOR: f64 = 1e-20;
    if band.is_empty() {
        return 1.0;
    }
    let mut log_sum = 0.0f64;
    let mut sum = 0.0f64;
    for magnitude in band {
        let power = magnitude * magnitude + FLOOR;
        log_sum += power.ln();
        sum += power;
    }
    let arithmetic = sum / band.len() as f64;
    if arithmetic <= 0.0 {
        return 1.0;
    }
    (log_sum / band.len() as f64).exp() / arithmetic
}

/// Median magnitude of each `SEGMENT_BINS`-wide stretch of the band, which a
/// peak has to stand clear of to count as a partial rather than noise.
pub fn segment_medians(band: &[f64]) -> Vec<f64> {
    let mut medians = Vec::with_capacity(band.len() / SEGMENT_BINS + 1);
    for segment in band.chunks(SEGMENT_BINS) {
        let mut sorted = segment.to_vec();
        sorted.sort_by(|a, b| a.total_cmp(b));
        medians.push(sorted[sorted.len() / 2]);
    }
    if medians.is_empty() {
        medians.push(0.0);
    }
    medians
}

/// Sub-bin position of a peak, from the three bins around it.
///
/// The fit is on log magnitude, not linear: with a Hann window the quadratic is
/// very nearly exact there, while on linear magnitude it is biased. This is the
/// standard sinusoidal-peak estimator, and it is what buys sub-cent frequency
/// resolution out of a 5 Hz bin grid.
pub fn refine(left: f64, centre: f64, right: f64) -> f64 {
    const FLOOR: f64 = 1e-20;
    let a = (left + FLOOR).ln();
    let b = (centre + FLOOR).ln();
    let c = (right + FLOOR).ln();
    let denominator = a - 2.0 * b + c;
    if denominator.abs() < 1e-12 {
        return 0.0;
    }
    (0.5 * (a - c) / denominator).clamp(-0.5, 0.5)
}
