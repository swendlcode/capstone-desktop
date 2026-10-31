//! The limiter's true-peak sidechain: ITU-R BS.1770-5 Annex 2, order-48 FIR as
//! four phases of twelve.
//!
//! The displayed dBTP figure comes from `ebur128`, which is conformance-tested
//! and has no business being hand-rolled. This exists for the one job that
//! crate cannot do: a streaming, sample-accurate detector the limiter can read
//! per sample. A sample-peak sidechain instead leaves 0.3 to 1 dB of
//! inter-sample overshoot in the written file, which becomes audible
//! distortion after any lossy transcode or on a cheap DAC's reconstruction
//! filter.
//!
//! The 4x table is used at every sample rate. Above 96 kHz the spec allows a
//! lower factor, and running 4x there simply measures more finely than it has
//! to, which is the safe direction for a ceiling.

/// Verbatim from the published table. Kept exactly as printed so it can be
/// checked against the spec, and normalised in `new` rather than edited here.
const PHASES: [[f64; 12]; 4] = [
    [
        0.001_708_984_375,
        0.010_986_328_125,
        -0.019_653_320_312_5,
        0.033_203_125,
        -0.059_448_242_187_5,
        0.137_329_101_562_5,
        0.972_167_968_75,
        -0.102_294_921_875,
        0.047_607_421_875,
        -0.026_611_328_125,
        0.014_892_578_125,
        -0.008_300_781_25,
    ],
    [
        -0.029_174_804_687_5,
        0.029_296_875,
        -0.051_757_812_5,
        0.089_111_328_125,
        -0.166_503_906_25,
        0.465_087_890_625,
        0.779_785_156_25,
        -0.200_317_382_812_5,
        0.101_562_5,
        -0.058_227_539_062_5,
        0.033_081_054_687_5,
        -0.018_920_898_437_5,
    ],
    [
        -0.018_920_898_437_5,
        0.033_081_054_687_5,
        -0.058_227_539_062_5,
        0.101_562_5,
        -0.200_317_382_812_5,
        0.779_785_156_25,
        0.465_087_890_625,
        -0.166_503_906_25,
        0.089_111_328_125,
        -0.051_757_812_5,
        0.029_296_875,
        -0.029_174_804_687_5,
    ],
    [
        -0.008_300_781_25,
        0.014_892_578_125,
        -0.026_611_328_125,
        0.047_607_421_875,
        -0.102_294_921_875,
        0.972_167_968_75,
        0.137_329_101_562_5,
        -0.059_448_242_187_5,
        0.033_203_125,
        -0.019_653_320_312_5,
        0.010_986_328_125,
        0.001_708_984_375,
    ],
];

/// How far back in the input the interpolated peak for sample `n` actually
/// sits. The dominant tap of phases 0 and 1 is index 6 and of phases 2 and 3 is
/// index 5, so the four phases describe the waveform between `n - 6` and
/// `n - 5`.
///
/// This is load-bearing. Reading the detector at `n` and applying the gain at
/// `n` would arrive six samples late, and those six samples would already have
/// been multiplied by a larger gain: an overshoot path that no amount of
/// look-ahead closes.
pub const GROUP_DELAY: usize = 6;

/// Samples of zero that have to be pushed past the end of a buffer before the
/// detector has reported on the last real sample.
pub const FLUSH: usize = 12;

/// One channel's detector. Twelve samples of history, four dot products, max of
/// the four magnitudes.
#[derive(Debug, Clone)]
pub struct TruePeak {
    phases: [[f64; 12]; 4],
    history: [f64; 12],
}

impl TruePeak {
    /// Each phase normalised to unity DC gain.
    ///
    /// The published phases do not have it: phases 0 and 3 sum to 1.001587 and
    /// phases 1 and 2 to 0.973022, which is -0.2376 dB. A sidechain built
    /// verbatim therefore under-reads low-frequency peaks by up to 0.24 dB,
    /// and under-reading is the unsafe direction for a ceiling. Normalising
    /// departs from the letter of the spec by at most that, in the
    /// conservative direction, and only inside the limiter: the number shown
    /// in the UI stays the conformance-tested one.
    pub fn normalised() -> Self {
        let mut phases = PHASES;
        for phase in phases.iter_mut() {
            let sum: f64 = phase.iter().sum();
            for tap in phase.iter_mut() {
                *tap /= sum;
            }
        }
        Self {
            phases,
            history: [0.0; 12],
        }
    }

    /// Pushes one sample and returns the largest interpolated magnitude across
    /// the four phases.
    pub fn push(&mut self, x: f64) -> f64 {
        self.history.copy_within(0..11, 1);
        self.history[0] = x;
        let mut peak = 0.0f64;
        for phase in &self.phases {
            let mut acc = 0.0;
            for (tap, sample) in phase.iter().zip(&self.history) {
                acc += tap * sample;
            }
            peak = peak.max(acc.abs());
        }
        peak
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_sum_to_one_after_normalising() {
        let detector = TruePeak::normalised();
        for (index, phase) in detector.phases.iter().enumerate() {
            let sum: f64 = phase.iter().sum();
            assert!((sum - 1.0).abs() < 1e-12, "phase {index} sums to {sum}");
        }
    }

    /// The case the whole stage exists for: a sine at a quarter of the sample
    /// rate, phased so the samples straddle the peak, reads 3.01 dB low on a
    /// sample-peak meter. The interpolator has to find the real peak.
    #[test]
    fn finds_the_peak_between_samples() {
        let fs = 48_000.0;
        let f = fs / 4.0;
        let mut detector = TruePeak::normalised();
        let mut sample_peak = 0.0f64;
        let mut true_peak = 0.0f64;
        for n in 0..4_000 {
            let phase = 2.0 * std::f64::consts::PI * f * n as f64 / fs + std::f64::consts::PI / 4.0;
            let x = phase.sin();
            sample_peak = sample_peak.max(x.abs());
            let reported = detector.push(x);
            if n > 24 {
                true_peak = true_peak.max(reported);
            }
        }
        assert!(
            (20.0 * sample_peak.log10() + 3.01).abs() < 0.05,
            "sample peak should under-read by 3 dB, got {sample_peak}"
        );
        assert!(
            20.0 * true_peak.log10() > -0.6,
            "true peak under-read: {} dBTP",
            20.0 * true_peak.log10()
        );
    }

    #[test]
    fn silence_reports_nothing() {
        let mut detector = TruePeak::normalised();
        for _ in 0..100 {
            assert_eq!(detector.push(0.0), 0.0);
        }
    }
}
