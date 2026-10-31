//! Tests for the look-ahead brickwall limiter.
//!
//! Kept out of `limiter.rs` so that file stays inside the module size the house
//! rules set, and declared as a child module there so it can still reach the
//! private stages it has to exercise.

use super::super::testkit;
use super::*;

/// The test the whole stage exists to pass. Across every character setting,
/// a range of input gains that force 0 to 20 dB of reduction, and several
/// ceilings: the written file must not exceed the ceiling, and the backstop
/// must never have to do anything.
#[test]
fn holds_the_ceiling() {
    let fs = 44_100;
    let frames = fs / 2;
    for character in 0u8..=10 {
        for gain_db in [0.0f64, 10.0, 20.0] {
            for ceiling_db in [-0.3f64, -1.5] {
                let gain = db_to_linear(gain_db) as f32;
                let left: Vec<f32> = testkit::transient_noise(frames, 0.7)
                    .iter()
                    .map(|s| s * gain)
                    .collect();
                let right: Vec<f32> = testkit::sine(220.0, fs as f64, frames, 0.9)
                    .iter()
                    .map(|s| s * gain)
                    .collect();
                let mut audio = DecodedAudio::new(vec![left, right], fs as u32);
                let clipped = limit(&mut audio, ceiling_db, character);
                let measured = testkit::true_peak_dbtp(&audio);
                assert_eq!(
                    clipped, 0,
                    "character {character}, +{gain_db} dB, ceiling {ceiling_db}: backstop \
                     clamped {clipped} samples"
                );
                assert!(
                    measured <= ceiling_db + 0.1,
                    "character {character}, +{gain_db} dB: {measured:.3} dBTP over a \
                     {ceiling_db} dB ceiling"
                );
            }
        }
    }
}

/// Nothing over the ceiling means nothing happens at all, bit for bit. If
/// this fails, something in the chain is not truly bypassed.
#[test]
fn transparent_below_the_ceiling() {
    let quiet = testkit::sine(1_000.0, 44_100.0, 44_100, 0.1);
    let mut audio = DecodedAudio::new(vec![quiet.clone()], 44_100);
    assert_eq!(limit(&mut audio, -1.5, 4), 0);
    assert_eq!(audio.channels[0], quiet);
}

/// At a -12.5 LUFS target and a -1.5 dBTP ceiling the limiter only engages
/// once the source's peak-to-loudness ratio passes 11 dB. Dense material
/// sits under that, so the gain move is the whole story and the file has to
/// land exactly on target - which is most of a DJ library.
#[test]
fn lands_on_the_target_when_the_limiter_is_idle() {
    let fs = 44_100;
    let dense = testkit::noise(fs * 3, 0.6);
    let mut audio = DecodedAudio::new(vec![dense.clone(), dense], fs as u32);
    apply(&mut audio, -12.5, -1.5, 4);
    let after = super::super::integrated_lufs(&audio).expect("measurable");
    let peak = testkit::true_peak_dbtp(&audio);
    assert!(peak <= -1.5, "the limiter engaged: {peak:.3} dBTP");
    assert!((after + 12.5).abs() < 0.1, "landed on {after:.3} LUFS");
}

/// A vinyl-rip profile: quiet overall, high peaks. It has to come up to
/// target and still respect the ceiling, which means the limiter working.
#[test]
fn a_wide_dynamic_source_comes_up_and_stays_under() {
    let fs = 44_100;
    let mut samples = testkit::transient_noise(fs * 3, 0.3);
    for (index, sample) in samples.iter_mut().enumerate() {
        if index % 20_000 < 2 {
            *sample = 0.95;
        }
    }
    let mut audio = DecodedAudio::new(vec![samples.clone(), samples], fs as u32);
    let gain = apply(&mut audio, -12.5, -1.5, 4);
    assert!(gain > 3.0, "a quiet source should be turned up, got {gain}");
    assert!(testkit::true_peak_dbtp(&audio) <= -1.4);
}

#[test]
fn odd_input_does_not_panic() {
    let cases = vec![
        DecodedAudio::new(vec![vec![]], 44_100),
        DecodedAudio::new(vec![vec![0.0; 64]], 44_100),
        DecodedAudio::new(vec![vec![1.0; 200]], 96_000),
        DecodedAudio::new(vec![vec![0.0; 8_000]], 8_000),
        DecodedAudio::new(Vec::new(), 44_100),
    ];
    for mut audio in cases {
        let gain = apply(&mut audio, -12.5, -1.5, 10);
        assert!(gain.is_finite());
        assert!(audio.channels.iter().flatten().all(|s| s.is_finite()));
    }
}
