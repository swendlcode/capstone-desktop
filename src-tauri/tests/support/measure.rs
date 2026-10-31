//! Measurement, deliberately independent of the code under test.
//!
//! Every number an assertion reads comes from here, and nothing here calls
//! into `capstone_lib`'s own analysis or sidechains: loudness and true peak
//! come from `ebur128` directly, spectra from `rustfft`, and the clip detector
//! is written out again rather than shared. A bug in the limiter's own
//! detector, or in `analyze::clip_count`, therefore cannot hide behind a test
//! that measured with it.

use capstone_lib::audio::DecodedAudio;
use ebur128::{EbuR128, Mode};
use rustfft::num_complex::Complex;
use rustfft::FftPlanner;

/// Integrated loudness, EBU R128, from the conformance-tested meter.
pub fn integrated_lufs(audio: &DecodedAudio) -> f64 {
    let frames = audio.frame_count();
    let channels = audio.channels.len() as u32;
    let mut meter = EbuR128::new(channels, audio.sample_rate, Mode::I).expect("meter");
    let planar: Vec<&[f32]> = audio.channels.iter().map(|c| &c[..frames]).collect();
    meter.add_frames_planar_f32(&planar).expect("frames");
    meter.loudness_global().expect("integrated loudness")
}

/// Loudest true peak across all channels, in dBTP. This is the inter-sample
/// figure: the ceiling test needs it, because an overshoot between two samples
/// does not exist in the samples at all.
pub fn true_peak_dbtp(audio: &DecodedAudio) -> f64 {
    let frames = audio.frame_count();
    let channels = audio.channels.len() as u32;
    let mut meter = EbuR128::new(channels, audio.sample_rate, Mode::TRUE_PEAK).expect("meter");
    let planar: Vec<&[f32]> = audio.channels.iter().map(|c| &c[..frames]).collect();
    meter.add_frames_planar_f32(&planar).expect("frames");
    let peak = (0..channels)
        .map(|c| meter.true_peak(c).unwrap_or(0.0))
        .fold(0.0f64, f64::max);
    linear_to_db(peak)
}

/// Largest absolute sample, in dBFS. The honest sample peak, which is what
/// "no output sample exceeds the ceiling" is about.
pub fn sample_peak_dbfs(audio: &DecodedAudio) -> f64 {
    let peak = audio
        .channels
        .iter()
        .flat_map(|c| c.iter())
        .fold(0.0f32, |acc, s| acc.max(s.abs()));
    linear_to_db(peak as f64)
}

/// Plain RMS over every sample of every channel, in dBFS.
///
/// Not a loudness figure and not meant to be one. It exists for the cases a
/// gated meter cannot describe: below the loudness standard's -70 LUFS
/// absolute gate every block is excluded, so `integrated_lufs` answers -inf
/// and two different files are indistinguishable by it. RMS also has the
/// property the comparison needs, which is that an allpass filter leaves it
/// alone - the multiband bank sums to allpass, so it moves the sample peak
/// while moving no energy.
pub fn rms_dbfs(audio: &DecodedAudio) -> f64 {
    let frames = audio.frame_count();
    let mut sum = 0.0f64;
    let mut count = 0usize;
    for channel in &audio.channels {
        for sample in &channel[..frames] {
            sum += (*sample as f64) * (*sample as f64);
            count += 1;
        }
    }
    if count == 0 {
        return -200.0;
    }
    linear_to_db((sum / count as f64).sqrt())
}

pub fn linear_to_db(linear: f64) -> f64 {
    if linear <= 0.0 {
        return -200.0;
    }
    20.0 * linear.log10()
}

pub fn db_to_linear(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

/// Fraction of full scale a sample has to reach to count toward a clip run,
/// and how long and how flat the run has to be.
///
/// Written out again rather than imported so this detector stays independent
/// of the one in `analyze`. The numbers are the ones the product contract
/// implies: a run of three or more consecutive same-sign samples pinned at the
/// rail, flat to within a quarter of the gap between the ceiling and full
/// scale.
const CEILING: f32 = 0.9999;
const MIN_RUN: usize = 3;
const FLAT_DELTA: f32 = (1.0 - CEILING) / 4.0;

/// One flat-topped run: which channel, where it starts, where it ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipRun {
    pub channel: usize,
    pub start: usize,
    pub end: usize,
}

/// Every flat-topped run in the buffer, in order.
///
/// The ranges are kept, not just the count, because "the repaired region is no
/// longer flat-topped" is a claim about those exact samples and nothing else:
/// a repair that merely turned the level down would pass a count-only test.
pub fn clip_runs(audio: &DecodedAudio) -> Vec<ClipRun> {
    let mut out = Vec::new();
    for (channel, samples) in audio.channels.iter().enumerate() {
        let mut index = 0;
        while index < samples.len() {
            if samples[index].abs() < CEILING {
                index += 1;
                continue;
            }
            let negative = samples[index].is_sign_negative();
            let (mut low, mut high) = (samples[index].abs(), samples[index].abs());
            let mut end = index + 1;
            while end < samples.len()
                && samples[end].abs() >= CEILING
                && samples[end].is_sign_negative() == negative
            {
                low = low.min(samples[end].abs());
                high = high.max(samples[end].abs());
                end += 1;
            }
            if end - index >= MIN_RUN && high - low <= FLAT_DELTA {
                out.push(ClipRun {
                    channel,
                    start: index,
                    end,
                });
            }
            index = end;
        }
    }
    out
}

/// Spread of `samples[range]` on one channel, linear. Zero means the stretch is
/// perfectly flat; anything above `FLAT_DELTA` means it is not a flat top any
/// more.
pub fn spread(audio: &DecodedAudio, channel: usize, start: usize, end: usize) -> f32 {
    let slice = &audio.channels[channel][start..end.min(audio.channels[channel].len())];
    let low = slice.iter().fold(f32::INFINITY, |a, s| a.min(s.abs()));
    let high = slice.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    if slice.is_empty() {
        0.0
    } else {
        high - low
    }
}

/// How flat a stretch has to still be to count as a flat top.
pub const FLATNESS_LIMIT: f32 = FLAT_DELTA;

/// Raw, unwindowed magnitude spectrum in dB. Fed an impulse response this *is*
/// the transfer function, which is what the band-sum flatness claim is about:
/// a window would roll the response off at both ends and hide a notch under
/// its own shape.
pub fn transfer_db(samples: &[f32]) -> Vec<f64> {
    let mut buffer: Vec<Complex<f64>> = samples
        .iter()
        .map(|s| Complex::new(*s as f64, 0.0))
        .collect();
    FftPlanner::<f64>::new()
        .plan_fft_forward(buffer.len())
        .process(&mut buffer);
    let half = buffer.len() / 2;
    buffer[..half]
        .iter()
        .map(|c| 20.0 * c.norm().max(1e-12).log10())
        .collect()
}

/// Hann-windowed magnitude spectrum in dBFS, scaled so a full-scale sine reads
/// 0 dB in its own bin.
pub fn spectrum_dbfs(samples: &[f32]) -> Vec<f64> {
    let len = samples.len();
    let windowed: Vec<f64> = samples
        .iter()
        .enumerate()
        .map(|(n, s)| {
            let w = 0.5 - 0.5 * (std::f64::consts::TAU * n as f64 / (len - 1) as f64).cos();
            *s as f64 * w
        })
        .collect();
    let mut buffer: Vec<Complex<f64>> = windowed.iter().map(|s| Complex::new(*s, 0.0)).collect();
    FftPlanner::<f64>::new()
        .plan_fft_forward(len)
        .process(&mut buffer);
    let scale = len as f64 / 4.0;
    buffer[..len / 2]
        .iter()
        .map(|c| 20.0 * (c.norm() / scale).max(1e-12).log10())
        .collect()
}

/// The largest power of two that fits, taken from the middle of the buffer so
/// a fade or an envelope at either end cannot smear the peak.
pub fn centre_window(samples: &[f32], cap: usize) -> &[f32] {
    let mut len = 1;
    while len * 2 <= samples.len().min(cap) {
        len *= 2;
    }
    let start = (samples.len() - len) / 2;
    &samples[start..start + len]
}

/// Dominant frequency, refined parabolically on the log magnitude. With a Hann
/// window this is accurate to a small fraction of a cent on a steady tone,
/// which is what the pitch assertions need.
pub fn peak_frequency(samples: &[f32], fs: f64) -> f64 {
    let window = centre_window(samples, 32_768);
    let spectrum = spectrum_dbfs(window);
    let peak = (1..spectrum.len() - 1)
        .max_by(|a, b| spectrum[*a].total_cmp(&spectrum[*b]))
        .unwrap_or(1);
    let (left, centre, right) = (spectrum[peak - 1], spectrum[peak], spectrum[peak + 1]);
    let delta = 0.5 * (left - right) / (left - 2.0 * centre + right);
    (peak as f64 + delta) * fs / window.len() as f64
}

/// Frequency of the strongest bin within `+/- span_hz` of `near_hz`, refined
/// the same way. Used to follow one known partial rather than whichever
/// happens to be loudest.
pub fn partial_frequency(samples: &[f32], fs: f64, near_hz: f64, span_hz: f64) -> f64 {
    let window = centre_window(samples, 32_768);
    let spectrum = spectrum_dbfs(window);
    let bin_hz = fs / window.len() as f64;
    let low = ((near_hz - span_hz) / bin_hz).floor().max(1.0) as usize;
    let high = (((near_hz + span_hz) / bin_hz).ceil() as usize).min(spectrum.len() - 2);
    let peak = (low..=high)
        .max_by(|a, b| spectrum[*a].total_cmp(&spectrum[*b]))
        .unwrap_or(low);
    let (left, centre, right) = (spectrum[peak - 1], spectrum[peak], spectrum[peak + 1]);
    let delta = 0.5 * (left - right) / (left - 2.0 * centre + right);
    (peak as f64 + delta) * bin_hz
}

/// Cents between two frequencies, signed.
pub fn cents_between(measured: f64, reference: f64) -> f64 {
    1200.0 * (measured / reference).log2()
}

/// Total energy in the harmonics of `fundamental`, relative to the fundamental
/// itself, in dB. This is the warmth measure: a memoryless shaper's whole
/// output is the fundamental plus its harmonics, so more harmonic energy is
/// more saturation and nothing else.
pub fn harmonic_distortion_db(samples: &[f32], fs: f64, fundamental: f64) -> f64 {
    let window = centre_window(samples, 32_768);
    let spectrum = spectrum_dbfs(window);
    let bin = |hz: f64| (hz * window.len() as f64 / fs).round() as usize;
    // Three bins either side of each harmonic: a Hann window spreads a tone
    // over about that much, so summing a single bin would read the window
    // shape rather than the harmonic.
    let energy_at = |hz: f64| -> f64 {
        let centre = bin(hz);
        if centre + 3 >= spectrum.len() {
            return 0.0;
        }
        (centre - 3..=centre + 3)
            .map(|k| db_to_linear(spectrum[k]).powi(2))
            .sum()
    };
    let fundamental_energy = energy_at(fundamental);
    if fundamental_energy <= 0.0 {
        return -200.0;
    }
    let harmonics: f64 = (2..=8)
        .map(|n| fundamental * n as f64)
        .filter(|hz| *hz < fs / 2.0 - 500.0)
        .map(energy_at)
        .sum();
    10.0 * (harmonics / fundamental_energy).max(1e-20).log10()
}
