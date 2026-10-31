//! Tests for clipped-peak repair.
//!
//! Kept out of `clip_repair.rs` so that file stays inside the module size the
//! house rules set, and declared as a child module there so it can still reach
//! the private stages it has to exercise.

use super::*;
use crate::audio::linear_to_db;
use crate::templates;

fn template(threshold_pct: f64) -> Template {
    Template {
        clip_threshold_pct: threshold_pct,
        ..templates::festival()
    }
}

/// Clip a sine at a known overdrive, repair it, and check the recovered
/// peak. Within 0.3 dB up to 2 dB of overdrive is this reconstruction's
/// measured behaviour, and the reason the default headroom is 1.5 dB.
#[test]
fn recovers_the_original_peak() {
    for overdrive_db in [0.5f64, 1.0, 2.0] {
        let amplitude = 10f64.powf(overdrive_db / 20.0);
        let fs = 48_000.0;
        let clipped: Vec<f32> = (0..9_600)
            .map(|n| {
                let x = amplitude * (2.0 * std::f64::consts::PI * 110.0 * n as f64 / fs).sin();
                x.clamp(-1.0, 1.0) as f32
            })
            .collect();
        let mut audio = DecodedAudio::new(vec![clipped], 48_000);
        let runs = repair(&mut audio, &template(99.7));
        assert!(runs > 0, "{overdrive_db} dB over: nothing detected");
        let peak = audio.peak() as f64;
        assert!(
            (linear_to_db(peak) - overdrive_db).abs() < 0.3,
            "{overdrive_db} dB over recovered as {:.3} dB",
            linear_to_db(peak)
        );
    }
}

/// Flat tops are the signal. An unclipped sine that merely passes through
/// the threshold on its way to full scale must come back untouched.
#[test]
fn leaves_an_unclipped_sine_alone() {
    let fs = 44_100.0;
    let original: Vec<f32> = (0..44_100)
        .map(|n| (2.0 * std::f64::consts::PI * 60.0 * n as f64 / fs).sin() as f32)
        .collect();
    let mut audio = DecodedAudio::new(vec![original.clone()], 44_100);
    let runs = repair(&mut audio, &template(99.7));
    assert_eq!(runs, 0, "a clean sine was treated as clipped");
    assert_eq!(audio.channels[0], original);
}

/// Flat for far longer than 20 ms: somebody's square-wave bass patch, not
/// damage, and a cubic across a gap that long would be inventing data.
#[test]
fn skips_long_flat_runs() {
    let samples: Vec<f32> = (0..44_100)
        .map(|n| if (n / 1_500) % 2 == 0 { 1.0 } else { -1.0 })
        .collect();
    let mut audio = DecodedAudio::new(vec![samples.clone()], 44_100);
    assert_eq!(repair(&mut audio, &template(99.7)), 0);
    assert_eq!(audio.channels[0], samples);
}

#[test]
fn odd_input_does_not_panic() {
    let cases = vec![
        DecodedAudio::new(vec![vec![]], 44_100),
        DecodedAudio::new(vec![vec![1.0; 4]], 44_100),
        DecodedAudio::new(vec![vec![0.0; 2_000]], 96_000),
        DecodedAudio::new(vec![vec![1.0; 50], vec![-1.0; 50]], 8_000),
        DecodedAudio::new(Vec::new(), 44_100),
    ];
    for mut audio in cases {
        repair(&mut audio, &template(99.7));
    }
}
