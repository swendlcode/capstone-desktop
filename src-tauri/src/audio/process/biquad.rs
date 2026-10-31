//! Second-order sections, and the Linkwitz-Riley pairs the crossover bank is
//! built from.
//!
//! State is f64 throughout, and that is not a preference. At a 120 Hz corner
//! and 44.1 kHz the coefficients cluster around 1 and -2, so the feedback path
//! subtracts two nearly equal numbers on every sample; f32 state turns the
//! residue into quantisation noise sitting in the low band, where it is
//! audible on a club system.

/// Butterworth alignment, which is the Q each half of a Linkwitz-Riley pair
/// needs so the cascade of two of them is -6 dB at the corner.
pub const BUTTERWORTH_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// Direct Form I. Chosen over DF2 because DF1 keeps the input history in its
/// own words rather than in a shared accumulator, which is the numerically
/// better behaved of the two at low corner frequencies.
#[derive(Debug, Clone, Copy, Default)]
pub struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    /// RBJ low-pass. `fc` is clamped into the band the formula is valid for, so
    /// a crossover left at 20 kHz on a 32 kHz file degenerates into something
    /// stable rather than into NaN.
    pub fn low_pass(fc: f64, fs: f64, q: f64) -> Self {
        let (cos_w, alpha, a0) = Self::common(fc, fs, q);
        let b0 = (1.0 - cos_w) / 2.0;
        Self::normalised(b0, 1.0 - cos_w, b0, cos_w, alpha, a0)
    }

    pub fn high_pass(fc: f64, fs: f64, q: f64) -> Self {
        let (cos_w, alpha, a0) = Self::common(fc, fs, q);
        let b0 = (1.0 + cos_w) / 2.0;
        Self::normalised(b0, -(1.0 + cos_w), b0, cos_w, alpha, a0)
    }

    fn common(fc: f64, fs: f64, q: f64) -> (f64, f64, f64) {
        let fs = fs.max(1.0);
        let fc = fc.clamp(1.0, fs * 0.49);
        let q = q.max(0.05);
        let w = 2.0 * std::f64::consts::PI * fc / fs;
        let alpha = w.sin() / (2.0 * q);
        (w.cos(), alpha, 1.0 + alpha)
    }

    fn normalised(b0: f64, b1: f64, b2: f64, cos_w: f64, alpha: f64, a0: f64) -> Self {
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: -2.0 * cos_w / a0,
            a2: (1.0 - alpha) / a0,
            ..Self::default()
        }
    }

    pub fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// A 4th-order Linkwitz-Riley branch: two identical Butterworth sections in
/// cascade. Each branch is -6 dB at the corner and the low and high branches
/// sum to an allpass.
///
/// LR2 needs the high-pass polarity inverted and LR4 does not. Inverting here
/// would put a deep notch at every crossover, which is the classic way this
/// goes wrong.
#[derive(Debug, Clone, Copy)]
pub struct Lr4 {
    first: Biquad,
    second: Biquad,
}

impl Lr4 {
    pub fn low_pass(fc: f64, fs: f64) -> Self {
        let section = Biquad::low_pass(fc, fs, BUTTERWORTH_Q);
        Self {
            first: section,
            second: section,
        }
    }

    pub fn high_pass(fc: f64, fs: f64) -> Self {
        let section = Biquad::high_pass(fc, fs, BUTTERWORTH_Q);
        Self {
            first: section,
            second: section,
        }
    }

    pub fn process(&mut self, x: f64) -> f64 {
        self.second.process(self.first.process(x))
    }
}

/// `LP + HP` of one Linkwitz-Riley corner, which is an allpass: flat
/// magnitude, 360 degrees of phase rotation.
///
/// Written as the literal sum of a second instance of the same filter pair
/// rather than as a hand-derived 4th-order allpass. It costs two extra biquads
/// per corner per band, which is nothing offline, and it cannot disagree with
/// the crossover it is compensating for.
#[derive(Debug, Clone, Copy)]
pub struct Allpass {
    low: Lr4,
    high: Lr4,
}

impl Allpass {
    pub fn new(fc: f64, fs: f64) -> Self {
        Self {
            low: Lr4::low_pass(fc, fs),
            high: Lr4::high_pass(fc, fs),
        }
    }

    pub fn process(&mut self, x: f64) -> f64 {
        self.low.process(x) + self.high.process(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// -6 dB per branch at the corner is what makes the pair sum to unity.
    #[test]
    fn lr4_branches_are_six_db_down_at_the_corner() {
        let fs = 48_000.0;
        let fc = 1_000.0;
        let mut low = Lr4::low_pass(fc, fs);
        let mut high = Lr4::high_pass(fc, fs);
        let mut low_peak = 0.0f64;
        let mut high_peak = 0.0f64;
        for n in 0..48_000 {
            let x = (2.0 * std::f64::consts::PI * fc * n as f64 / fs).sin();
            let l = low.process(x);
            let h = high.process(x);
            if n > 24_000 {
                low_peak = low_peak.max(l.abs());
                high_peak = high_peak.max(h.abs());
            }
        }
        assert!(
            (20.0 * low_peak.log10() + 6.0).abs() < 0.1,
            "low {low_peak}"
        );
        assert!(
            (20.0 * high_peak.log10() + 6.0).abs() < 0.1,
            "high {high_peak}"
        );
    }

    #[test]
    fn a_corner_above_nyquist_stays_finite() {
        let mut low = Lr4::low_pass(20_000.0, 32_000.0);
        for n in 0..1_000 {
            let y = low.process(if n == 0 { 1.0 } else { 0.0 });
            assert!(y.is_finite());
        }
    }
}
