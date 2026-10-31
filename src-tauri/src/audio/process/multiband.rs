//! The multiband compressor behind "EDIT MULTIBAND DYNAMICS".
//!
//! A serial bank with upward allpass compensation, which is derived rather than
//! guessed. For crossovers at 120, 800 and 5000 Hz:
//!
//! ```text
//! band0 = LP120                      band1 = LP800 * HP120
//! band2 = LP5k * HP800 * HP120       band3 = HP5k * HP800 * HP120
//!
//! band2 + band3            = AP5k * HP800 * HP120
//! band1' + band2 + band3   = AP5k * AP800 * HP120     (AP5k on band1)
//! total                    = AP5k * AP800 * AP120     (AP800*AP5k on band0)
//! ```
//!
//! So band *k* needs the allpass of every crossover above it, and the sum of
//! all bands is exactly allpass. The bands are not time-aligned with each
//! other, since each carries a different number of phase rotations, and that is
//! fine for dynamics: it is the sum that has to be flat.
//!
//! A tree topology with hand-derived compensation is the classic source of
//! crossover notches, which is why this one is written out literally and the
//! flatness is asserted in a test before anything gets tuned by ear.

use super::biquad::{Allpass, Lr4};
use crate::audio::{db_to_linear, DecodedAudio};
use crate::models::BandSetting;

/// Soft-knee width. `BandSetting` carries no knee field, so it is fixed at the
/// value the shipped band defaults were chosen around: 6 dB with ratios at or
/// under 1.6:1 gives one to three decibels of gain reduction on already
/// mastered material, which is consistency rather than mastering.
const KNEE_DB: f64 = 6.0;

/// Detector averaging. The lowest band gets a longer window because its attack
/// has to stay above about one and a half periods of its own content or it
/// modulates the waveform instead of the envelope.
const DETECTOR_MS: f64 = 10.0;
const DETECTOR_MS_LOW: f64 = 30.0;
const LOW_BAND_HZ: f64 = 200.0;

/// Splits the buffer into the template's bands, compresses each on its own
/// terms, and sums them back.
///
/// The bands arrive sorted with each `high_hz` equal to the next band's
/// `low_hz`, so the split is a chain of crossovers with no gap and no overlap
/// to reason about. A crossover pair has to sum flat, or the file comes back
/// with a notch at every band edge that no single band's settings explain.
pub fn apply(audio: &mut DecodedAudio, bands: &[BandSetting]) {
    let frames = audio.frame_count();
    let channel_count = audio.channels.len();
    if frames == 0 || channel_count == 0 || bands.is_empty() {
        return;
    }
    let fs = audio.sample_rate.max(1) as f64;
    let corners = corners(bands, fs);
    let band_count = corners.len() + 1;
    let mut splits: Vec<Split> = (0..channel_count)
        .map(|_| Split::new(&corners, fs))
        .collect();
    let mut compressors: Vec<Compressor> = bands[..band_count]
        .iter()
        .map(|band| Compressor::new(band, fs))
        .collect();
    let mut split_out = vec![0.0f64; band_count * channel_count];
    let mut mixed = vec![0.0f64; channel_count];

    for frame in 0..frames {
        for (channel, split) in splits.iter_mut().enumerate() {
            let base = channel * band_count;
            let sample = audio.channels[channel][frame] as f64;
            split.process(sample, &mut split_out[base..base + band_count]);
        }
        mix(&split_out, &mut compressors, channel_count, &mut mixed);
        for (channel, value) in mixed.iter().enumerate() {
            audio.channels[channel][frame] = *value as f32;
        }
    }
}

/// Sums the bands back with each band's gain applied, and that gain is shared
/// across channels: an unlinked band compressor shifts the stereo image as soon
/// as it starts working.
fn mix(split_out: &[f64], compressors: &mut [Compressor], channel_count: usize, mixed: &mut [f64]) {
    let band_count = compressors.len();
    mixed.fill(0.0);
    for (band, compressor) in compressors.iter_mut().enumerate() {
        let mut energy = 0.0;
        for channel in 0..channel_count {
            let value = split_out[channel * band_count + band];
            energy += value * value;
        }
        let gain = compressor.gain(energy / channel_count as f64);
        for (channel, slot) in mixed.iter_mut().enumerate() {
            *slot += split_out[channel * band_count + band] * gain;
        }
    }
}

/// The crossover frequencies, which are the `high_hz` of every band but the
/// last.
///
/// Truncates at the first corner that is out of order or too near Nyquist,
/// which merges the bands above it into one. That is what keeps a template
/// built for 44.1 kHz from producing nonsense on an 8 kHz file, where a 5 kHz
/// crossover does not exist.
fn corners(bands: &[BandSetting], fs: f64) -> Vec<f64> {
    let limit = fs * 0.45;
    let mut out: Vec<f64> = Vec::new();
    for band in &bands[..bands.len() - 1] {
        let corner = band.high_hz as f64;
        if corner >= limit || corner <= out.last().copied().unwrap_or(0.0) {
            break;
        }
        out.push(corner);
    }
    out
}

/// One channel's filter bank.
struct Split {
    stages: Vec<(Lr4, Lr4)>,
    /// Per band, the allpasses of every crossover above it.
    compensation: Vec<Vec<Allpass>>,
}

impl Split {
    fn new(corners: &[f64], fs: f64) -> Self {
        let stages = corners
            .iter()
            .map(|corner| (Lr4::low_pass(*corner, fs), Lr4::high_pass(*corner, fs)))
            .collect();
        let compensation = (0..corners.len() + 1)
            .map(|band| {
                corners
                    .iter()
                    .skip(band + 1)
                    .map(|corner| Allpass::new(*corner, fs))
                    .collect()
            })
            .collect();
        Self {
            stages,
            compensation,
        }
    }

    fn process(&mut self, x: f64, out: &mut [f64]) {
        let mut residual = x;
        for (index, (low, high)) in self.stages.iter_mut().enumerate() {
            out[index] = low.process(residual);
            residual = high.process(residual);
        }
        out[self.stages.len()] = residual;
        for (band, chain) in self.compensation.iter_mut().enumerate() {
            for allpass in chain.iter_mut() {
                out[band] = allpass.process(out[band]);
            }
        }
    }
}

struct Compressor {
    threshold_db: f64,
    /// `1 - 1/ratio`: how much of the excess over the threshold is given back.
    slope: f64,
    makeup: f64,
    detector: f64,
    attack: f64,
    release: f64,
    energy: f64,
    reduction_db: f64,
}

impl Compressor {
    fn new(band: &BandSetting, fs: f64) -> Self {
        let ratio = (band.ratio as f64).max(1.0);
        let window = if (band.low_hz as f64) < LOW_BAND_HZ {
            DETECTOR_MS_LOW
        } else {
            DETECTOR_MS
        };
        Self {
            threshold_db: band.threshold_db as f64,
            slope: 1.0 - 1.0 / ratio,
            makeup: db_to_linear(band.makeup_db as f64),
            detector: pole(window, fs),
            attack: pole(band.attack_ms as f64, fs),
            release: pole(band.release_ms as f64, fs),
            energy: 0.0,
            reduction_db: 0.0,
        }
    }

    /// `mean_square` is the band's power averaged across channels.
    fn gain(&mut self, mean_square: f64) -> f64 {
        self.energy += (mean_square - self.energy) * self.detector;
        let level_db = 10.0 * self.energy.max(1e-20).log10();
        let target = knee(level_db - self.threshold_db, KNEE_DB) * self.slope;
        let coefficient = if target > self.reduction_db {
            self.attack
        } else {
            self.release
        };
        self.reduction_db += (target - self.reduction_db) * coefficient;
        db_to_linear(-self.reduction_db) * self.makeup
    }
}

/// Quadratic soft knee, in dB. Returns the excess to be acted on, not the
/// reduction: the band's ratio scales it.
fn knee(over: f64, width: f64) -> f64 {
    if width <= 0.0 {
        return over.max(0.0);
    }
    let half = width / 2.0;
    if over <= -half {
        0.0
    } else if over >= half {
        over
    } else {
        (over + half).powi(2) / (2.0 * width)
    }
}

fn pole(ms: f64, fs: f64) -> f64 {
    let tau = (ms / 1000.0).max(1e-6);
    1.0 - (-1.0 / (tau * fs)).exp()
}

#[cfg(test)]
#[path = "multiband_tests.rs"]
mod tests;
