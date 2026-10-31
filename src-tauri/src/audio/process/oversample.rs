//! Integer-factor oversampling for the warmth waveshaper.
//!
//! A memoryless shaper generates harmonics at `N * f`, and at the native rate
//! they fold back around Nyquist. The audible failure is specific: the second
//! harmonic of a 15 kHz tone at 44.1 kHz lands at 30 kHz and aliases to
//! 14.1 kHz, a descending tone that moves the wrong way with the input. That
//! is what makes an unoversampled "warmth" control sound like a broken codec.
//!
//! One polyphase stage per direction, not a cascade of halfband stages: at
//! these factors a single stage is both cheaper per output sample and far
//! easier to reason about, and this is an offline batch tool.
//!
//! The kernel is symmetric and odd-length with its centre at `24 * factor`, so
//! the round trip delays by exactly `TAPS_PER_PHASE` base-rate samples
//! whatever the factor. That matters: clip positions and loudness gating
//! blocks both assume the buffer stays sample-aligned, so the caller
//! compensates with an integer, never a fraction.

/// Taps each polyphase branch runs. Sets the transition width, which is what
/// decides whether a 15 kHz tone survives the round trip: 48 taps per branch
/// puts the passband edge near 17 kHz and the stopband edge near 24 kHz at
/// 44.1 kHz, so the tone passes and every harmonic it generates is 90 dB down
/// before decimation folds it.
const TAPS_PER_PHASE: usize = 48;

/// Kernel cutoff as a fraction of the *base* sample rate. 0.47 leaves a guard
/// band below Nyquist for the transition to sit in.
const CUTOFF_FRACTION: f64 = 0.47;

/// The largest factor any warmth setting asks for. Fixing it lets the
/// per-sample inner loop work out of a stack array.
const MAX_FACTOR: usize = 8;

pub struct Oversampler {
    factor: usize,
    /// One reversed tap set per phase, pre-scaled by `factor` to undo the DC
    /// loss that zero-stuffing costs.
    up: Vec<Vec<f64>>,
    down: Vec<f64>,
    up_hist: Ring,
    down_hist: Ring,
}

impl Oversampler {
    pub fn new(factor: usize) -> Self {
        let factor = factor.clamp(1, MAX_FACTOR);
        let kernel = kernel(factor);
        let up = (0..factor)
            .map(|phase| {
                let mut taps: Vec<f64> = kernel
                    .iter()
                    .skip(phase)
                    .step_by(factor)
                    .map(|h| h * factor as f64)
                    .collect();
                taps.reverse();
                taps
            })
            .collect::<Vec<_>>();
        let up_cap = up.iter().map(|t| t.len()).max().unwrap_or(1);
        let mut down = kernel;
        down.reverse();
        let down_cap = down.len() + factor - 1;
        Self {
            factor,
            up,
            down,
            up_hist: Ring::new(up_cap),
            down_hist: Ring::new(down_cap),
        }
    }

    /// Base-rate samples between a sample going in and coming back out.
    pub fn delay(&self) -> usize {
        TAPS_PER_PHASE
    }

    /// Pushes one base-rate sample through upsample, shaper and downsample, and
    /// returns the base-rate sample that falls out, which is the input from
    /// `delay()` samples ago.
    pub fn shape(&mut self, x: f64, shaper: fn(f64) -> f64) -> f64 {
        self.up_hist.push(x);
        let mut lifted = [0.0f64; MAX_FACTOR];
        let window = self.up_hist.window();
        for (phase, taps) in self.up.iter().enumerate() {
            let offset = window.len() - taps.len();
            let mut acc = 0.0;
            for (tap, sample) in taps.iter().zip(&window[offset..]) {
                acc += tap * sample;
            }
            lifted[phase] = shaper(acc);
        }
        for value in &lifted[..self.factor] {
            self.down_hist.push(*value);
        }
        let window = self.down_hist.window();
        let mut acc = 0.0;
        for (tap, sample) in self.down.iter().zip(window) {
            acc += tap * sample;
        }
        acc
    }
}

/// Windowed sinc, normalised to unity DC gain so the stage cannot shift level.
fn kernel(factor: usize) -> Vec<f64> {
    let taps = TAPS_PER_PHASE * factor + 1;
    let centre = (taps - 1) as f64 / 2.0;
    let cutoff = CUTOFF_FRACTION / factor as f64;
    let mut kernel: Vec<f64> = (0..taps)
        .map(|i| {
            let t = i as f64 - centre;
            let sinc = if t.abs() < 1e-12 {
                2.0 * cutoff
            } else {
                (2.0 * std::f64::consts::PI * cutoff * t).sin() / (std::f64::consts::PI * t)
            };
            sinc * blackman_harris(i, taps)
        })
        .collect();
    let sum: f64 = kernel.iter().sum();
    for tap in kernel.iter_mut() {
        *tap /= sum;
    }
    kernel
}

/// Four-term Blackman-Harris: about 92 dB of stopband rejection, which is what
/// buys the headroom the aliasing test asks for.
fn blackman_harris(index: usize, len: usize) -> f64 {
    let phase = 2.0 * std::f64::consts::PI * index as f64 / (len - 1) as f64;
    0.35875 - 0.48829 * phase.cos() + 0.14128 * (2.0 * phase).cos() - 0.01168 * (3.0 * phase).cos()
}

/// A history window that reads as one contiguous slice.
///
/// Every sample is written twice, `cap` apart, so the most recent `cap` samples
/// are always a plain subslice. The inner loop then runs a straight dot product
/// instead of a modulo per tap, which at 513 taps per sample is the difference
/// between seconds and minutes on a full track.
struct Ring {
    buf: Vec<f64>,
    cap: usize,
    pos: usize,
}

impl Ring {
    fn new(cap: usize) -> Self {
        let cap = cap.max(1);
        Self {
            buf: vec![0.0; cap * 2],
            cap,
            pos: 0,
        }
    }

    fn push(&mut self, x: f64) {
        self.buf[self.pos] = x;
        self.buf[self.pos + self.cap] = x;
        self.pos += 1;
        if self.pos == self.cap {
            self.pos = 0;
        }
    }

    /// Oldest first, newest last.
    fn window(&self) -> &[f64] {
        &self.buf[self.pos..self.pos + self.cap]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(x: f64) -> f64 {
        x
    }

    /// With the shaper bypassed the round trip has to give back the input,
    /// delayed by exactly `delay()`. If the delay were fractional this test
    /// would fail by a few per cent and every alignment downstream would be
    /// quietly wrong.
    #[test]
    fn round_trip_is_transparent_and_integer_aligned() {
        for factor in [2usize, 4, 8] {
            let fs = 44_100.0;
            let frames = 4_096;
            let input: Vec<f64> = (0..frames)
                .map(|n| (2.0 * std::f64::consts::PI * 1_000.0 * n as f64 / fs).sin() * 0.5)
                .collect();
            let mut os = Oversampler::new(factor);
            let delay = os.delay();
            let mut out = vec![0.0; frames];
            for i in 0..frames + delay {
                let x = if i < frames { input[i] } else { 0.0 };
                let y = os.shape(x, identity);
                if i >= delay {
                    out[i - delay] = y;
                }
            }
            // Skip the filter's own startup, where the window is still filling.
            let error = (delay * 2..frames - delay)
                .map(|i| (out[i] - input[i]).abs())
                .fold(0.0f64, f64::max);
            assert!(error < 1e-3, "factor {factor} error {error}");
        }
    }

    /// Unity DC gain, to within the kernel's own stopband floor: the per-phase
    /// sums differ by the filter's response at the images, which is 90 dB down
    /// but not zero.
    #[test]
    fn unity_dc_gain() {
        let mut os = Oversampler::new(4);
        let mut last = 0.0;
        for _ in 0..1_000 {
            last = os.shape(0.25, identity);
        }
        assert!((last - 0.25).abs() < 1e-3, "dc gain drifted: {last}");
    }
}
