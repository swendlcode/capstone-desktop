//! End-to-end tests for the chain: the behaviour the product is judged on,
//! exercised on buffers rather than files so no decode or encode is involved.
//!
//! Kept out of `mod.rs` so that file stays inside the module size the house
//! rules set, and declared as a child module there so it can still reach the
//! private loudness helper it has to measure with.

use super::*;
use crate::models::{BandSetting, PitchMode, Warmth};
use crate::templates;

fn dense_stereo(seconds: usize) -> DecodedAudio {
    let frames = 44_100 * seconds;
    let left = testkit::transient_noise(frames, 0.6);
    let mut right = testkit::sine(220.0, 44_100.0, frames, 0.4);
    for (sample, noise) in right.iter_mut().zip(&left) {
        *sample += noise * 0.5;
    }
    DecodedAudio::new(vec![left, right], 44_100)
}

/// A tonal buffer sitting `cents` away from concert pitch: four partials of a
/// chord, loud enough and long enough for the estimator to have something to
/// work with.
fn detuned_stereo(cents: f64) -> DecodedAudio {
    let fs = 44_100.0;
    let frames = 44_100 * 4;
    let ratio = 2f64.powf(cents / 1200.0);
    let mut mix = vec![0.0f32; frames];
    for (index, partial) in [220.0, 277.18, 329.63, 440.0].iter().enumerate() {
        let tone = testkit::sine(partial * ratio, fs, frames, 0.2 / (index as f64 + 1.0));
        for (slot, value) in mix.iter_mut().zip(&tone) {
            *slot += value;
        }
    }
    DecodedAudio::new(vec![mix.clone(), mix], 44_100)
}

/// The pitch branch end to end, in both modes: a file 25 cents sharp has to be
/// measured, corrected, reported, and come back in tune.
#[test]
fn corrects_a_detuned_file() {
    for mode in [PitchMode::KeepTempo, PitchMode::ChangeTempo] {
        let mut audio = detuned_stereo(25.0);
        let template = Template {
            correct_pitch: true,
            pitch_mode: mode,
            ..templates::festival()
        };
        let report = process(&mut audio, &template).expect("processed");
        assert!(
            (report.pitch_corrected_cents + 25.0).abs() < 2.0,
            "{mode:?} reported {}",
            report.pitch_corrected_cents
        );
        let residual = crate::audio::analyze::tuning_cents(&audio);
        assert!(
            residual.abs() < 6.0,
            "{mode:?} left {residual} cents of detune"
        );
    }
}

/// And the tolerance it is gated on: 4 cents is inside the default 10, so
/// nothing should be touched. Nudging a file that was already in tune is worse
/// than leaving it.
#[test]
fn leaves_a_nearly_in_tune_file_alone() {
    let mut audio = detuned_stereo(4.0);
    let template = Template {
        correct_pitch: true,
        ..templates::festival()
    };
    let report = process(&mut audio, &template).expect("processed");
    assert_eq!(report.pitch_corrected_cents, 0.0);
}

/// The headline behaviour: whatever arrives, the file leaves on target and
/// under the ceiling.
#[test]
fn the_default_template_hits_its_target() {
    let mut audio = dense_stereo(3);
    let report = process(&mut audio, &templates::festival()).expect("processed");
    assert!(
        (report.lufs_after + 12.5).abs() < 0.5,
        "landed on {:.3} LUFS",
        report.lufs_after
    );
    assert!(
        testkit::true_peak_dbtp(&audio) <= -1.4,
        "{:.3} dBTP over a -1.5 dB ceiling",
        testkit::true_peak_dbtp(&audio)
    );
    assert_eq!(report.output_path, "", "the chain must not touch the disk");
    assert_eq!(report.pitch_corrected_cents, 0.0);
}

/// A quiet source has to come up, not just be left alone.
#[test]
fn a_quiet_source_is_brought_up() {
    let mut audio = dense_stereo(3);
    audio.apply_gain(0.05);
    let report = process(&mut audio, &templates::festival()).expect("processed");
    assert!(
        report.gain_applied_db > 10.0,
        "only {:.2} dB of gain",
        report.gain_applied_db
    );
    assert!((report.lufs_after + 12.5).abs() < 0.5);
}

/// Nothing enabled, nothing asked for, and the output must be bit-identical
/// to the input. If it is not, some stage is not truly bypassed - which is
/// the only way to be sure the chain is doing exactly what the template
/// says and nothing else.
#[test]
fn null_test_is_bit_identical() {
    let mut audio = dense_stereo(2);
    audio.apply_gain(0.7);
    let original = audio.channels.clone();
    let measured = integrated_lufs(&audio).expect("measurable");
    let template = Template {
        target_lufs: measured,
        repair_clips: false,
        correct_pitch: false,
        warmth: Warmth::None,
        // An empty band list is the real bypass. Idle compressors are not:
        // the signal would still go through the crossover bank and come back
        // phase-rotated, which is allpass but not identical.
        bands: Vec::new(),
        // Ceiling at full scale, with the material a few decibels below it,
        // so the limiter has nothing to do either.
        headroom_db: 0.0,
        ..templates::festival()
    };
    let report = process(&mut audio, &template).expect("processed");
    assert_eq!(report.gain_applied_db, 0.0);
    assert_eq!(audio.channels, original, "the null path altered the buffer");
}

/// Users re-run their whole library periodically, so a second pass has to be
/// a near no-op. A tool that degrades every time it is run is a steady
/// stream of bug reports.
#[test]
fn processing_twice_is_a_near_no_op() {
    let mut audio = dense_stereo(3);
    let first = process(&mut audio, &templates::festival()).expect("first pass");
    let second = process(&mut audio, &templates::festival()).expect("second pass");
    assert!(
        (second.lufs_after - first.lufs_after).abs() < 0.3,
        "{:.3} then {:.3} LUFS",
        first.lufs_after,
        second.lufs_after
    );
    assert_eq!(
        second.clips_repaired, 0,
        "the second pass found new clipping"
    );
    assert!(testkit::true_peak_dbtp(&audio) <= -1.4);
}

/// Clipped input: the repair has to find it, and the report has to say so.
#[test]
fn reports_repaired_clips() {
    let frames = 44_100 * 2;
    let clipped: Vec<f32> = (0..frames)
        .map(|n| {
            let x = 1.3 * (2.0 * std::f64::consts::PI * 110.0 * n as f64 / 44_100.0).sin();
            x.clamp(-1.0, 1.0) as f32
        })
        .collect();
    let mut audio = DecodedAudio::new(vec![clipped], 44_100);
    let report = process(&mut audio, &templates::festival()).expect("processed");
    assert!(
        report.clips_repaired > 100,
        "found {}",
        report.clips_repaired
    );
}

/// The repair gate: a quiet file with flat tops was mastered that way.
#[test]
fn quiet_flat_tops_are_left_alone() {
    let frames = 44_100 * 2;
    let quiet: Vec<f32> = (0..frames)
        .map(|n| {
            let x = 1.3 * (2.0 * std::f64::consts::PI * 110.0 * n as f64 / 44_100.0).sin();
            (x.clamp(-1.0, 1.0) * 0.02) as f32
        })
        .collect();
    let mut audio = DecodedAudio::new(vec![quiet], 44_100);
    let report = process(&mut audio, &templates::festival()).expect("processed");
    assert_eq!(report.clips_repaired, 0);
}

#[test]
fn odd_input_does_not_panic() {
    let template = Template {
        warmth: Warmth::Tube,
        pitch_mode: PitchMode::ChangeTempo,
        bands: vec![BandSetting::default()],
        ..templates::festival()
    };
    let cases = vec![
        DecodedAudio::new(vec![vec![]], 44_100),
        DecodedAudio::new(vec![vec![0.0; 128]], 44_100),
        DecodedAudio::new(vec![vec![1.0; 2_048]], 96_000),
        DecodedAudio::new(vec![vec![0.5; 8_000], vec![-0.5; 8_000]], 8_000),
        DecodedAudio::new(Vec::new(), 44_100),
        DecodedAudio::new(vec![vec![0.0; 44_100]], 44_100),
    ];
    for mut audio in cases {
        let report = process(&mut audio, &template).expect("no hard failure");
        assert!(report.lufs_before.is_finite() && report.lufs_after.is_finite());
        assert!(audio.channels.iter().flatten().all(|s| s.is_finite()));
    }
}
