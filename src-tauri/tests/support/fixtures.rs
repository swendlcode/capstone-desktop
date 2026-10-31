//! The signals these tests run on, generated here and written to real files.
//!
//! Nothing reads a file off this machine. Every fixture is built from a
//! deterministic generator and written out as 32-bit float WAV through `hound`
//! directly, so the bytes on disk are exactly the samples that were generated
//! and a fixture cannot drift between runs or between machines.
//!
//! Float, specifically, because the fixtures carry claims that integer
//! quantisation would blur: fixture (b) is clipped at exactly +/-1.0 and the
//! clip detector's flatness tolerance is two least-significant bits of 16-bit.

use capstone_lib::audio::DecodedAudio;

use super::measure;

// Re-exported so a test asks `fixtures` for a fixture and a place to put it,
// without having to know which of the two files it came from.
pub use super::files::{materialise, scratch};

/// Deterministic, because a DSP test that fails one run in fifty is a test
/// nobody trusts. xorshift64, the same generator the in-crate tests use.
pub struct Noise {
    state: u64,
}

impl Noise {
    pub fn new(seed: u64) -> Self {
        Self { state: seed | 1 }
    }

    pub fn next(&mut self) -> f64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        (self.state >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0
    }
}

/// Scales the buffer so an independent R128 pass reads exactly `target_lufs`.
///
/// Gain is linear and loudness is a log of a mean square, so one measurement
/// and one multiply is exact; there is nothing to iterate.
pub fn normalise_to(mut audio: DecodedAudio, target_lufs: f64) -> DecodedAudio {
    let measured = measure::integrated_lufs(&audio);
    assert!(
        measured.is_finite(),
        "fixture is too short or too quiet to normalise"
    );
    audio.apply_gain(measure::db_to_linear(target_lufs - measured) as f32);
    audio
}

/// Fixture (a): 30 s of programme-like stereo at 44.1 kHz.
///
/// Mixed sines with slow envelopes, a noise bed, and two families of noise
/// burst - a 30 ms tick every 250 ms and a 120 ms hit every second - so the
/// signal has both sustained tonal content for the pitch and warmth measures
/// and real transients for the limiter to work against. Normalised to exactly
/// the requested loudness on the way out.
pub fn programme(seconds: f64, rate: u32, target_lufs: f64) -> DecodedAudio {
    let fs = rate as f64;
    let frames = (seconds * fs) as usize;
    let mut noise = Noise::new(0x2545_F491_4F6C_DD1D);
    let mut left = vec![0.0f32; frames];
    let mut right = vec![0.0f32; frames];

    // One draw per frame, shared by both channels with different weights, so
    // the two sides are correlated the way a real mix is rather than being two
    // independent noise fields.
    for frame in 0..frames {
        let t = frame as f64 / fs;
        let bed = noise.next();

        let mut l = 0.0;
        let mut r = 0.0;
        for (index, (partial, level)) in [
            (55.0, 0.45),
            (110.0, 0.30),
            (220.0, 0.22),
            (440.0, 0.16),
            (1_320.0, 0.09),
        ]
        .iter()
        .enumerate()
        {
            // A slow envelope per partial, each at its own rate, so no two
            // layers peak together and the sum is not a static chord.
            let envelope =
                0.6 + 0.4 * (std::f64::consts::TAU * (0.11 + 0.037 * index as f64) * t).sin();
            let phase = std::f64::consts::TAU * partial * t;
            l += level * envelope * phase.sin();
            // A fixed per-partial phase offset on the right: decorrelated
            // enough to be stereo, not so much that it sums to mono.
            r += level * envelope * (phase + 0.4 * (index as f64 + 1.0)).sin();
        }

        l += bed * 0.02;
        r += bed * 0.018;

        // 30 ms tick every 250 ms, 120 ms hit every second. Exponential decay
        // from the burst's own start, so each one is a transient rather than a
        // gate on the noise bed.
        let tick = burst(t, 0.25, 0.030, 0.55);
        let hit = burst(t, 1.0, 0.120, 0.85);
        let percussive = bed * (tick + hit);
        l += percussive;
        r += percussive * 0.9;

        left[frame] = l as f32;
        right[frame] = r as f32;
    }

    normalise_to(DecodedAudio::new(vec![left, right], rate), target_lufs)
}

/// Envelope of a burst that restarts every `period` seconds and decays over
/// `length`. Zero outside the burst.
fn burst(t: f64, period: f64, length: f64, amplitude: f64) -> f64 {
    let since = t % period;
    if since >= length {
        return 0.0;
    }
    amplitude * (-since / (length / 3.0)).exp()
}

/// Fixture (b): the given signal driven `drive_db` into the rail and hard
/// clipped, which is what an overcooked master or a hot bounce looks like.
pub fn clipped(source: &DecodedAudio, drive_db: f64) -> DecodedAudio {
    let gain = measure::db_to_linear(drive_db) as f32;
    let frames = source.frame_count();
    let channels: Vec<Vec<f32>> = source
        .channels
        .iter()
        .map(|c| {
            c[..frames]
                .iter()
                .map(|s| (s * gain).clamp(-1.0, 1.0))
                .collect()
        })
        .collect();
    DecodedAudio::new(channels, source.sample_rate)
}

/// A clipped sine, whose number of flat tops is known by arithmetic rather
/// than by asking a detector.
///
/// A sine of frequency `f` held above the rail for `seconds` flattens one top
/// per half cycle, so the file carries `2 * f * seconds` flat tops per channel,
/// give or take the partial cycle at either end. `overdrive_db` sets how far
/// into the clip it is driven, which decides how many samples wide each flat
/// top is: at 110 Hz, 44.1 kHz and 1.5 dB over, each one is about twelve
/// samples, well inside the three-sample minimum and the 20 ms maximum a
/// repair will act on.
///
/// Returns the signal and the number of flat tops to expect per channel.
pub fn clipped_sine(
    frequency: f64,
    seconds: f64,
    rate: u32,
    overdrive_db: f64,
    channels: usize,
) -> (DecodedAudio, u64) {
    let fs = rate as f64;
    let frames = (seconds * fs) as usize;
    let amplitude = measure::db_to_linear(overdrive_db);
    let samples: Vec<f32> = (0..frames)
        .map(|n| {
            let value = amplitude * (std::f64::consts::TAU * frequency * n as f64 / fs).sin();
            value.clamp(-1.0, 1.0) as f32
        })
        .collect();
    // Half cycles that are entirely inside the buffer. The first flat top
    // starts a fraction of a half cycle in and the last one may be cut off, so
    // the count is floored rather than rounded.
    let half_cycles = (2.0 * frequency * frames as f64 / fs).floor() as u64;
    (
        DecodedAudio::new(vec![samples; channels], rate),
        half_cycles,
    )
}

/// Fixture (c): a sustained chord sitting exactly `cents` from concert pitch.
///
/// Five partials of an A minor triad, every one multiplied by the same ratio,
/// which is what a tape running fast or a turntable off speed does to a record.
/// The frequencies are returned alongside, so the pitch assertions can follow a
/// known partial rather than whichever bin happens to be loudest.
pub fn detuned(cents: f64, seconds: f64, rate: u32) -> (DecodedAudio, Vec<f64>) {
    let fs = rate as f64;
    let frames = (seconds * fs) as usize;
    let ratio = 2f64.powf(cents / 1200.0);
    let concert = vec![220.0, 261.63, 329.63, 440.0, 880.0];
    let levels = [0.26, 0.22, 0.20, 0.18, 0.10];

    let mut mix = vec![0.0f32; frames];
    for (frame, slot) in mix.iter_mut().enumerate() {
        let t = frame as f64 / fs;
        let mut value = 0.0;
        for (partial, level) in concert.iter().zip(&levels) {
            value += level * (std::f64::consts::TAU * partial * ratio * t).sin();
        }
        *slot = value as f32;
    }
    (
        DecodedAudio::new(vec![mix.clone(), mix], rate),
        concert.into_iter().map(|hz| hz * ratio).collect(),
    )
}

/// A steady sine, for the warmth and format measures.
pub fn tone(
    frequency: f64,
    seconds: f64,
    rate: u32,
    amplitude: f64,
    channels: usize,
) -> DecodedAudio {
    let fs = rate as f64;
    let frames = (seconds * fs) as usize;
    let samples: Vec<f32> = (0..frames)
        .map(|n| (amplitude * (std::f64::consts::TAU * frequency * n as f64 / fs).sin()) as f32)
        .collect();
    DecodedAudio::new(vec![samples; channels], rate)
}

/// A single sample at full scale and nothing else. Fed through a filter bank
/// this *is* its transfer function, which is what the multiband flatness claim
/// needs.
pub fn impulse(frames: usize, rate: u32) -> DecodedAudio {
    let mut samples = vec![0.0f32; frames];
    samples[0] = 1.0;
    DecodedAudio::new(vec![samples], rate)
}

/// The left channel on its own, re-normalised so it still sits where the
/// stereo fixture did. A mono file is its own case in the pipeline: every
/// stage links its gain across channels, and a channel count of one is the
/// degenerate form of that.
pub fn to_mono(audio: &DecodedAudio, target_lufs: f64) -> DecodedAudio {
    let frames = audio.frame_count();
    let mono = audio.channels[0][..frames].to_vec();
    normalise_to(
        DecodedAudio::new(vec![mono], audio.sample_rate),
        target_lufs,
    )
}

/// Pure digital silence.
pub fn silence(seconds: f64, rate: u32, channels: usize) -> DecodedAudio {
    let frames = (seconds * rate as f64) as usize;
    DecodedAudio::new(vec![vec![0.0f32; frames]; channels], rate)
}

/// An already-limited master: dense, flat noise squared off at the rail and
/// normalised to a loudness no further gain should be given to.
///
/// Its peak-to-loudness ratio is a few decibels, which is the profile the
/// limiter has to leave alone: the only thing that should happen to this file
/// is being turned down.
pub fn limited_master(seconds: f64, rate: u32, target_lufs: f64) -> DecodedAudio {
    let frames = (seconds * rate as f64) as usize;
    let mut noise = Noise::new(0x9E37_79B9_7F4A_7C15);
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    for _ in 0..frames {
        let value = noise.next();
        // A touch of soft clipping, so the crest factor is a master's rather
        // than raw noise's.
        let squared = (value * 2.2).tanh() / 2.2f64.tanh();
        left.push((squared * 0.95) as f32);
        right.push((squared * 0.92) as f32);
    }
    normalise_to(DecodedAudio::new(vec![left, right], rate), target_lufs)
}
