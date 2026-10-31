//! Signal generators and measurements the stage tests share.
//!
//! Measurement is deliberately independent of the code under test: the spectra
//! come from `rustfft` and the true peak from `ebur128`, so a bug in the
//! limiter's own sidechain cannot hide behind a test that uses it.

use ebur128::{EbuR128, Mode};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

use crate::audio::DecodedAudio;

pub fn sine(frequency: f64, fs: f64, frames: usize, amplitude: f64) -> Vec<f32> {
    (0..frames)
        .map(|n| {
            (amplitude * (2.0 * std::f64::consts::PI * frequency * n as f64 / fs).sin()) as f32
        })
        .collect()
}

/// A deterministic broadband signal with transients: an xorshift noise floor
/// with a click every 1,000 samples. Deterministic matters, because a limiter
/// safety test that fails one run in fifty is a test nobody trusts.
pub fn transient_noise(frames: usize, amplitude: f64) -> Vec<f32> {
    let mut noise = noise(frames, amplitude);
    for (index, sample) in noise.iter_mut().enumerate() {
        if index % 1_000 >= 3 {
            *sample *= 0.25;
        }
    }
    noise
}

/// Flat broadband noise, same generator without the transients. Its
/// peak-to-loudness ratio is under 2 dB, which is the profile of an
/// already-limited modern master: the material the limiter must leave alone.
pub fn noise(frames: usize, amplitude: f64) -> Vec<f32> {
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    (0..frames)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let value = (state >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0;
            (value * amplitude) as f32
        })
        .collect()
}

/// Raw FFT magnitude. Fed an impulse response this *is* the transfer function,
/// which is what the crossover flatness test needs.
pub fn magnitude(samples: &[f32]) -> Vec<f64> {
    let mut buffer: Vec<Complex<f64>> = samples
        .iter()
        .map(|s| Complex::new(*s as f64, 0.0))
        .collect();
    FftPlanner::<f64>::new()
        .plan_fft_forward(buffer.len())
        .process(&mut buffer);
    buffer[..buffer.len() / 2]
        .iter()
        .map(|c| c.norm())
        .collect()
}

/// Hann-windowed magnitude in dBFS, scaled so a full-scale sine reads 0 dB in
/// its own bin.
pub fn spectrum_dbfs(samples: &[f32]) -> Vec<f64> {
    let len = samples.len();
    let windowed: Vec<f32> = samples
        .iter()
        .enumerate()
        .map(|(n, s)| {
            let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * n as f64 / (len - 1) as f64).cos();
            (*s as f64 * w) as f32
        })
        .collect();
    let scale = len as f64 / 4.0;
    magnitude(&windowed)
        .into_iter()
        .map(|m| 20.0 * (m / scale).max(1e-12).log10())
        .collect()
}

/// Dominant frequency, refined parabolically on the log magnitude - the
/// standard sinusoidal-peak estimate, which with a Hann window is near exact.
pub fn peak_frequency(samples: &[f32], fs: f64) -> f64 {
    let mut len = 1;
    while len * 2 <= samples.len().min(32_768) {
        len *= 2;
    }
    let start = (samples.len() - len) / 2;
    let spectrum = spectrum_dbfs(&samples[start..start + len]);
    let peak = (1..spectrum.len() - 1)
        .max_by(|a, b| spectrum[*a].total_cmp(&spectrum[*b]))
        .unwrap_or(0);
    let (left, centre, right) = (spectrum[peak - 1], spectrum[peak], spectrum[peak + 1]);
    let delta = 0.5 * (left - right) / (left - 2.0 * centre + right);
    (peak as f64 + delta) * fs / len as f64
}

/// Loudest true peak across all channels, in dBTP, from the conformance-tested
/// meter rather than from the limiter's own detector.
pub fn true_peak_dbtp(audio: &DecodedAudio) -> f64 {
    let frames = audio.frame_count();
    let channels = audio.channels.len() as u32;
    let mut meter = EbuR128::new(channels, audio.sample_rate, Mode::TRUE_PEAK).expect("meter");
    let planar: Vec<&[f32]> = audio.channels.iter().map(|c| &c[..frames]).collect();
    meter.add_frames_planar_f32(&planar).expect("frames");
    let peak = (0..channels)
        .map(|c| meter.true_peak(c).unwrap_or(0.0))
        .fold(0.0f64, f64::max);
    crate::audio::linear_to_db(peak)
}
