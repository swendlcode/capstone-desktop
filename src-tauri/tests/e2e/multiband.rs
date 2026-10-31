//! Claim 6: with every compressor bypassed, the bands sum flat to within
//! 0.1 dB.
//!
//! This is the test that has to pass before any of the band settings are
//! tuned by ear. A tree of crossovers with hand-derived allpass compensation
//! is the classic source of a notch at every band edge, and a notch is
//! something no single band's threshold or ratio can explain to a user.
//!
//! The response is read off a real impulse file: fed an impulse, the bank's
//! output *is* its transfer function, so there is nothing to infer.

use capstone_lib::audio::{decode, process};
use capstone_lib::models::BandSetting;
use capstone_lib::templates;

use crate::support::{fixtures, measure};

/// Thresholds above full scale, so no band can engage whatever is fed in.
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

#[test]
fn the_bands_sum_flat_when_every_compressor_is_bypassed() {
    let dir = fixtures::scratch("multiband");
    let len = 32_768;
    let rate = 44_100u32;
    let (path, _) = fixtures::materialise(fixtures::impulse(len, rate), &dir, "impulse.wav");

    let mut audio = decode::decode_to_f32(&path).expect("decoded");
    assert_eq!(
        audio.frame_count(),
        len,
        "the impulse did not survive a round trip"
    );
    process::multiband::apply(&mut audio, &bypassed());

    let response = measure::transfer_db(&audio.channels[0]);
    let bin = |hz: f64| (hz * len as f64 / rate as f64).round() as usize;

    let mut worst = 0.0f64;
    let mut worst_hz = 0.0f64;
    let audible = response
        .iter()
        .enumerate()
        .take(bin(20_000.0) + 1)
        .skip(bin(20.0));
    for (k, magnitude) in audible {
        let deviation = magnitude.abs();
        if deviation > worst {
            worst = deviation;
            worst_hz = k as f64 * rate as f64 / len as f64;
        }
    }
    assert!(
        worst <= 0.1,
        "band sum deviates by {worst:.4} dB at {worst_hz:.0} Hz"
    );
}

/// At 8 kHz the 5 kHz crossover does not exist, and the bank has to merge the
/// bands above it rather than build a filter at or past Nyquist. The sum still
/// has to be flat, because a template built for 44.1 kHz is the one that will
/// be pointed at an odd file.
#[test]
fn the_bands_still_sum_flat_at_a_rate_the_template_was_not_built_for() {
    let dir = fixtures::scratch("multiband-8k");
    let len = 16_384;
    let rate = 8_000u32;
    let (path, _) = fixtures::materialise(fixtures::impulse(len, rate), &dir, "impulse.wav");

    let mut audio = decode::decode_to_f32(&path).expect("decoded");
    process::multiband::apply(&mut audio, &bypassed());

    let response = measure::transfer_db(&audio.channels[0]);
    let bin = |hz: f64| (hz * len as f64 / rate as f64).round() as usize;
    let worst = (bin(20.0)..=bin(3_500.0))
        .map(|k| response[k].abs())
        .fold(0.0f64, f64::max);
    assert!(worst <= 0.1, "band sum deviates by {worst:.4} dB at 8 kHz");
}

/// And it has to engage when it is not bypassed, or the flatness above is the
/// flatness of a stage that does nothing.
#[test]
fn the_bands_engage_when_they_are_not_bypassed() {
    let dir = fixtures::scratch("multiband-active");
    let (path, _) = fixtures::materialise(
        fixtures::tone(2_000.0, 1.0, 44_100, 0.9, 1),
        &dir,
        "loud.wav",
    );
    let original = decode::decode_to_f32(&path).expect("decoded");
    let mut audio = original.clone();
    process::multiband::apply(&mut audio, &templates::default_bands());

    // Measured on the tail, where the detector has settled.
    let tail = original.frame_count() / 2;
    let before = original.channels[0][tail..]
        .iter()
        .fold(0.0f32, |a, s| a.max(s.abs()));
    let after = audio.channels[0][tail..]
        .iter()
        .fold(0.0f32, |a, s| a.max(s.abs()));
    assert!(
        after < before * 0.95,
        "a 0.9 full-scale tone came back at {after} from {before}"
    );
}
