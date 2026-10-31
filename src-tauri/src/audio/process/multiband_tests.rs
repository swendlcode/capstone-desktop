//! Tests for the multiband bank.
//!
//! Kept out of `multiband.rs` so that file stays inside the module size the
//! house rules set, and declared as a child module there so it can still reach
//! the private stages it has to exercise.

use super::super::testkit;
use super::*;
use crate::templates;

/// Thresholds so high that no band can engage, so `apply` reduces to the
/// crossover bank and nothing else.
fn bypassed() -> Vec<BandSetting> {
    templates::default_bands()
        .into_iter()
        .map(|band| BandSetting {
            threshold_db: 24.0,
            makeup_db: 0.0,
            ..band
        })
        .collect()
}

/// The one test that has to pass before any of this is tuned by ear: with
/// every compressor idle the band sum must be flat across the audible
/// range. Anything else is a notch at a crossover.
#[test]
fn bands_sum_flat() {
    let fs = 44_100.0;
    let len = 32_768;
    let mut impulse = vec![0.0f32; len];
    impulse[0] = 1.0;
    let mut audio = DecodedAudio::new(vec![impulse], fs as u32);
    apply(&mut audio, &bypassed());

    let response = testkit::magnitude(&audio.channels[0]);
    let bin = |hz: f64| (hz * len as f64 / fs).round() as usize;
    let worst = (bin(20.0)..=bin(20_000.0))
        .map(|k| (20.0 * response[k].log10()).abs())
        .fold(0.0f64, f64::max);
    assert!(worst < 0.01, "band sum deviates by {worst:.4} dB");
}

/// And it has to engage when asked: a loud band should come back quieter.
#[test]
fn compresses_above_the_threshold() {
    let fs = 44_100.0;
    let loud = testkit::sine(2_000.0, fs, 44_100, 0.9);
    let mut audio = DecodedAudio::new(vec![loud.clone()], fs as u32);
    apply(&mut audio, &templates::default_bands());
    let before = loud[22_050..].iter().fold(0.0f32, |a, s| a.max(s.abs()));
    let after = audio.channels[0][22_050..]
        .iter()
        .fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(after < before * 0.95, "{before} -> {after}");
}

#[test]
fn odd_input_does_not_panic() {
    let cases = vec![
        DecodedAudio::new(vec![vec![]], 44_100),
        DecodedAudio::new(vec![vec![0.0; 16]], 44_100),
        DecodedAudio::new(vec![vec![0.5; 4_096]], 96_000),
        // 8 kHz: the 5 kHz crossover does not exist at this rate.
        DecodedAudio::new(vec![vec![0.5; 4_096], vec![-0.5; 4_096]], 8_000),
        DecodedAudio::new(Vec::new(), 44_100),
    ];
    for mut audio in cases {
        apply(&mut audio, &templates::default_bands());
        assert!(audio.channels.iter().flatten().all(|s| s.is_finite()));
    }
}

#[test]
fn a_single_band_still_works() {
    let mut audio = DecodedAudio::new(vec![testkit::sine(1_000.0, 44_100.0, 4_096, 0.5)], 44_100);
    apply(&mut audio, &[BandSetting::default()]);
    assert!(audio.channels[0].iter().all(|s| s.is_finite()));
}
