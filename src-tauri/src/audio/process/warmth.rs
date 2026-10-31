//! Analogue-style warmth: a saturation stage, optional, three settings.
//!
//! Both active settings are memoryless waveshapers, and the only thing that
//! decides whether they sound like warmth or like a broken codec is
//! oversampling - see `super::oversample`.
//!
//! The level normalisation is the other half of it. A fixed shaper's
//! distortion depends on how hard it is driven, so without normalisation the
//! same setting does visibly different things to a -20 LUFS vinyl rip and a
//! -6 LUFS master. For a tool whose whole purpose is batch consistency that is
//! a defect, so the signal is scaled so its running 400 ms level sits at
//! -18 dBFS going in, and the scaling is undone coming out.

use super::oversample::Oversampler;
use crate::audio::{db_to_linear, DecodedAudio};
use crate::models::Warmth;

/// Level the shaper is driven at, whatever the file arrives at.
const DRIVE_TARGET_DB: f64 = -18.0;

/// Averaging window for the drive level. Long enough that it tracks the
/// arrangement rather than the waveform, so it cannot modulate the signal.
const DRIVE_WINDOW_SEC: f64 = 0.4;

/// The most the drive normalisation will move a file either way. Without a
/// clamp a near-silent passage asks for an unbounded boost.
const MAX_TRIM_DB: f64 = 24.0;

/// Shorter than this and the oversampler's own impulse response is a larger
/// part of the output than the file is, so the stage is skipped. 512 samples is
/// under 12 ms: not audio anybody is warming up.
const MIN_FRAMES: usize = 512;

const GENTLE_DRIVE: f64 = 1.5;
const TUBE_DRIVE: f64 = 2.2;
const TUBE_BIAS: f64 = 0.25;

/// `tanh(TUBE_BIAS)` and `TUBE_DRIVE * sech^2(TUBE_BIAS)`, written out rather
/// than recomputed per sample.
const TUBE_BIAS_TANH: f64 = 0.244_918_662_403_709_13;
const TUBE_NORM: f64 = 2.068_032_667_374_031_7;

/// Gentle is a soft odd-order saturation, barely measurable and meant to be
/// barely audible. Tube is the same shape driven much harder, with the
/// even-order asymmetry that makes a valve stage sound like one.
///
/// `Warmth::None` is a no-op and must stay one: the user who picked "just
/// regular sound" asked for their file back, not for a stage set to zero. The
/// oversampler is not even constructed on that path, so it is bit-transparent.
pub fn apply(audio: &mut DecodedAudio, warmth: Warmth) {
    let (factor, shaper): (usize, fn(f64) -> f64) = match warmth {
        Warmth::None => return,
        // Even harmonics fold worse than odd ones and the tube drive is higher,
        // so it needs the extra factor of two.
        Warmth::Gentle => (4, gentle),
        Warmth::Tube => (8, tube),
    };
    let frames = audio.frame_count();
    if frames < MIN_FRAMES || audio.channels.is_empty() {
        return;
    }
    let fs = audio.sample_rate.max(1) as f64;
    let scale = drive_scale(audio, frames, fs);
    let block_dc = matches!(warmth, Warmth::Tube);

    for index in 0..audio.channels.len() {
        let mut oversampler = Oversampler::new(factor);
        let delay = oversampler.delay();
        let mut dc = DcBlocker::new(fs);
        let mut shaped = vec![0.0f32; frames];
        // Runs `delay` samples past the end so the last real sample makes it
        // back out, and reads the output `delay` late so the buffer stays
        // sample-aligned: clip positions and loudness gating blocks downstream
        // both assume it.
        for step in 0..frames + delay {
            let driven = if step < frames {
                audio.channels[index][step] as f64 * scale[step]
            } else {
                0.0
            };
            let value = oversampler.shape(driven, shaper);
            if step >= delay {
                let frame = step - delay;
                let undriven = value / scale[frame];
                shaped[frame] = if block_dc {
                    dc.process(undriven) as f32
                } else {
                    undriven as f32
                };
            }
        }
        audio.channels[index][..frames].copy_from_slice(&shaped);
    }
}

/// `tanh(d*x)/d` rather than `tanh(d*x)/tanh(d)` with a separate trim.
///
/// The two shapes differ only by a constant, and dividing by `d` makes the
/// small-signal gain exactly one by construction. A hardcoded trim would have
/// to be re-derived every time the drive moved, and if it were ever wrong the
/// warmth setting would quietly leak into the Volume column.
///
/// tanh is odd, so this produces only third, fifth and seventh harmonics - no
/// even content, which is what "just regular sound, slightly rounded" means.
fn gentle(x: f64) -> f64 {
    (GENTLE_DRIVE * x).tanh() / GENTLE_DRIVE
}

/// The bias is what creates the even harmonics, and the even harmonics are the
/// valve signature: roughly the second at -32 dB and the third at -38 dB at
/// -10 dBFS, so about 3% distortion dominated by the second.
///
/// Subtracting `tanh(b)` removes the static offset the bias introduces and
/// dividing by `d * sech^2(b)` holds the small-signal gain at one.
fn tube(x: f64) -> f64 {
    ((TUBE_DRIVE * x + TUBE_BIAS).tanh() - TUBE_BIAS_TANH) / TUBE_NORM
}

/// Per-frame scaling that puts the running level at `DRIVE_TARGET_DB`.
///
/// Shared across channels, computed from the summed power, so the stage cannot
/// modulate the stereo image. Seeded from the file's own opening rather than
/// from zero: a one-pole starting at silence would ask for the maximum boost
/// over the first 400 ms and drive a loud intro into hard saturation.
fn drive_scale(audio: &DecodedAudio, frames: usize, fs: f64) -> Vec<f64> {
    let target = db_to_linear(DRIVE_TARGET_DB);
    let floor = db_to_linear(-MAX_TRIM_DB);
    let roof = db_to_linear(MAX_TRIM_DB);
    let coefficient = 1.0 - (-1.0 / (DRIVE_WINDOW_SEC * fs)).exp();
    let seed_len = frames.min((DRIVE_WINDOW_SEC * fs) as usize).max(1);
    let mut energy = (0..seed_len).map(|frame| power(audio, frame)).sum::<f64>() / seed_len as f64;
    (0..frames)
        .map(|frame| {
            energy += (power(audio, frame) - energy) * coefficient;
            let level = energy.sqrt();
            if level < 1e-9 {
                1.0
            } else {
                (target / level).clamp(floor, roof)
            }
        })
        .collect()
}

fn power(audio: &DecodedAudio, frame: usize) -> f64 {
    let total: f64 = audio
        .channels
        .iter()
        .map(|channel| {
            let value = channel[frame] as f64;
            value * value
        })
        .sum();
    total / audio.channels.len() as f64
}

/// One-pole high-pass at 5 Hz, for the asymmetric shaper only.
///
/// Subtracting `tanh(b)` in the shaper removes the *static* offset, but the
/// offset a bias produces is signal dependent, so a constant subtraction is not
/// enough. What is left eats headroom and shows up as a sample peak nobody can
/// hear.
struct DcBlocker {
    coefficient: f64,
    previous_in: f64,
    previous_out: f64,
}

impl DcBlocker {
    fn new(fs: f64) -> Self {
        Self {
            coefficient: (-2.0 * std::f64::consts::PI * 5.0 / fs.max(1.0)).exp(),
            previous_in: 0.0,
            previous_out: 0.0,
        }
    }

    fn process(&mut self, x: f64) -> f64 {
        let y = x - self.previous_in + self.coefficient * self.previous_out;
        self.previous_in = x;
        self.previous_out = y;
        y
    }
}

#[cfg(test)]
mod tests {
    use super::super::testkit;
    use super::*;

    #[test]
    fn none_is_bit_transparent() {
        let original = testkit::sine(1_000.0, 44_100.0, 44_100, 0.8);
        let mut audio = DecodedAudio::new(vec![original.clone(), original.clone()], 44_100);
        apply(&mut audio, Warmth::None);
        assert_eq!(audio.channels[0], original);
        assert_eq!(audio.channels[1], original);
    }

    /// The only test that catches insufficient oversampling. A 15 kHz tone's
    /// second harmonic lands at 30 kHz and, without enough oversampling, folds
    /// back to 14.1 kHz as a tone that moves the wrong way with the input.
    ///
    /// Measured on the tail, where the drive envelope has settled, so what is
    /// left below the tone is aliasing and nothing else.
    #[test]
    fn warmth_does_not_alias() {
        let fs = 44_100.0;
        for setting in [Warmth::Gentle, Warmth::Tube] {
            let tone = testkit::sine(15_000.0, fs, 44_100, db_to_linear(-6.0));
            let mut audio = DecodedAudio::new(vec![tone], 44_100);
            apply(&mut audio, setting);
            let tail = &audio.channels[0][44_100 - 16_384..];
            let spectrum = testkit::spectrum_dbfs(tail);
            let limit = (14_000.0 * 16_384.0 / fs) as usize;
            let worst = spectrum[1..limit]
                .iter()
                .fold(f64::NEG_INFINITY, |a, b| a.max(*b));
            assert!(worst < -70.0, "{setting:?} aliased to {worst:.1} dBFS");
            let tone_bin = (15_000.0 * 16_384.0 / fs).round() as usize;
            assert!(
                spectrum[tone_bin] > -12.0,
                "{setting:?} lost the tone itself: {:.1} dBFS",
                spectrum[tone_bin]
            );
        }
    }

    /// Warmth must not move the level, or it pollutes the Volume column and the
    /// loudness stage spends its gain undoing it.
    #[test]
    fn keeps_the_level() {
        let fs = 44_100.0;
        for setting in [Warmth::Gentle, Warmth::Tube] {
            let tone = testkit::sine(440.0, fs, 44_100, 0.1);
            let mut audio = DecodedAudio::new(vec![tone.clone()], 44_100);
            apply(&mut audio, setting);
            let before = rms(&tone[22_050..]);
            let after = rms(&audio.channels[0][22_050..]);
            let moved = 20.0 * (after / before).log10();
            assert!(
                moved.abs() < 0.5,
                "{setting:?} moved level by {moved:.3} dB"
            );
        }
    }

    /// It does have to do something, though, and something asymmetric: the tube
    /// setting is the one that is supposed to produce even harmonics.
    #[test]
    fn tube_produces_even_harmonics() {
        let fs = 44_100.0;
        let tone = testkit::sine(1_000.0, fs, 44_100, 0.3);
        let mut audio = DecodedAudio::new(vec![tone], 44_100);
        apply(&mut audio, Warmth::Tube);
        let spectrum = testkit::spectrum_dbfs(&audio.channels[0][44_100 - 16_384..]);
        let bin = |hz: f64| (hz * 16_384.0 / fs).round() as usize;
        let second = spectrum[bin(2_000.0)];
        let noise = spectrum[bin(1_500.0)];
        assert!(
            second > noise + 20.0,
            "no second harmonic: {second:.1} dB against a {noise:.1} dB floor"
        );
    }

    #[test]
    fn silence_stays_silent() {
        let mut audio = DecodedAudio::new(vec![vec![0.0; 8_192]], 44_100);
        apply(&mut audio, Warmth::Tube);
        assert!(audio.channels[0].iter().all(|s| *s == 0.0));
    }

    #[test]
    fn odd_input_does_not_panic() {
        let cases = vec![
            DecodedAudio::new(vec![vec![]], 44_100),
            DecodedAudio::new(vec![vec![0.5; 64]], 44_100),
            DecodedAudio::new(vec![vec![0.9; 4_096]], 96_000),
            DecodedAudio::new(vec![vec![0.5; 1_024], vec![-0.5; 1_024]], 8_000),
            DecodedAudio::new(Vec::new(), 44_100),
        ];
        for mut audio in cases {
            apply(&mut audio, Warmth::Tube);
            assert!(audio.channels.iter().flatten().all(|s| s.is_finite()));
        }
    }

    fn rms(samples: &[f32]) -> f64 {
        let total: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        (total / samples.len() as f64).sqrt()
    }
}
