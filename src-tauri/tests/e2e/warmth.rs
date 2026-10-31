//! Claim 5: `tube` adds measurably more harmonic content than `gentle`, which
//! adds more than `none`, compared via FFT.
//!
//! The measure is the energy in harmonics two through eight relative to the
//! fundamental. Both warmth settings are memoryless waveshapers, so their whole
//! output is the fundamental plus its harmonics and nothing else: more
//! harmonic energy is more saturation, with no second interpretation.

use capstone_lib::audio::{decode, process};
use capstone_lib::models::Warmth;

use crate::support::{fixtures, measure};

const TONE_HZ: f64 = 1_000.0;

/// Harmonic energy relative to the fundamental, in dB, for one warmth setting,
/// measured on a real file decoded off disk.
fn distortion_db(path: &std::path::Path, warmth: Warmth) -> f64 {
    let mut audio = decode::decode_to_f32(path).expect("decoded");
    process::warmth::apply(&mut audio, warmth);
    assert!(
        audio.channels.iter().flatten().all(|s| s.is_finite()),
        "{warmth:?} produced a non-finite sample"
    );
    measure::harmonic_distortion_db(&audio.channels[0], audio.sample_rate as f64, TONE_HZ)
}

#[test]
fn tube_distorts_more_than_gentle_and_gentle_more_than_none() {
    let dir = fixtures::scratch("warmth");
    // 0.3 full scale, which is a realistic place for a mix to sit before the
    // stage's own drive normalisation takes it to -18 dBFS.
    let (path, _) = fixtures::materialise(
        fixtures::tone(TONE_HZ, 2.0, 44_100, 0.3, 1),
        &dir,
        "tone.wav",
    );

    let none = distortion_db(&path, Warmth::None);
    let gentle = distortion_db(&path, Warmth::Gentle);
    let tube = distortion_db(&path, Warmth::Tube);

    // A clear margin rather than a bare inequality: three settings that
    // differed by a tenth of a decibel would satisfy an ordering test while
    // being indistinguishable in use.
    assert!(
        gentle > none + 10.0,
        "gentle added nothing audible: {gentle:.1} dB against {none:.1} dB for none"
    );
    assert!(
        tube > gentle + 6.0,
        "tube is not meaningfully warmer than gentle: {tube:.1} dB against {gentle:.1} dB"
    );

    // And neither may be so hard that it stops being warmth. 3% is the figure
    // the tube stage is documented around; -25 dB is 5.6%.
    assert!(
        tube < -25.0,
        "tube is distorting at {tube:.1} dB, which is a fault not a flavour"
    );
}

/// `none` has to be a genuine bypass, not a stage set to zero. The user who
/// picked "just regular sound" asked for their file back.
#[test]
fn none_is_bit_transparent_through_a_real_file() {
    let dir = fixtures::scratch("warmth-none");
    let (path, _) = fixtures::materialise(
        fixtures::programme(3.0, 44_100, -20.0),
        &dir,
        "programme.wav",
    );
    let original = decode::decode_to_f32(&path).expect("decoded");
    let mut audio = original.clone();
    process::warmth::apply(&mut audio, Warmth::None);
    assert_eq!(
        audio.channels, original.channels,
        "Warmth::None altered the buffer"
    );
}

/// The one failure mode that makes a saturator sound broken rather than warm.
/// A 15 kHz tone's second harmonic lands at 30 kHz; without enough
/// oversampling it folds back to 14.1 kHz as a tone that moves the wrong way
/// with the input.
#[test]
fn neither_setting_aliases() {
    let dir = fixtures::scratch("warmth-alias");
    let (path, _) = fixtures::materialise(
        fixtures::tone(15_000.0, 1.5, 44_100, measure::db_to_linear(-6.0), 1),
        &dir,
        "high.wav",
    );

    for setting in [Warmth::Gentle, Warmth::Tube] {
        let mut audio = decode::decode_to_f32(&path).expect("decoded");
        process::warmth::apply(&mut audio, setting);
        let fs = audio.sample_rate as f64;
        let window = measure::centre_window(&audio.channels[0], 16_384);
        let spectrum = measure::spectrum_dbfs(window);
        let limit = (14_000.0 * window.len() as f64 / fs) as usize;
        let worst = spectrum[1..limit]
            .iter()
            .fold(f64::NEG_INFINITY, |a, b| a.max(*b));
        assert!(
            worst < -70.0,
            "{setting:?} folded something back to {worst:.1} dBFS below 14 kHz"
        );
    }
}

/// Warmth is allowed to move the level, but only by the amount the chain is
/// built to absorb.
///
/// A saturator compresses, so it moves loudness by construction: the figure
/// `limiter::apply` is written around is one to three decibels from multiband
/// and warmth together, which is why that stage measures loudness for itself
/// rather than trusting the reading taken before them. What this guards is a
/// shaper whose trim has drifted - a stage leaking six or ten decibels would
/// still land on target, because the limiter would quietly spend its gain
/// undoing it, and the only visible symptom would be a Volume column nobody
/// could explain.
#[test]
fn warmth_stays_inside_the_level_budget_the_chain_allows() {
    let dir = fixtures::scratch("warmth-level");
    let (path, _) = fixtures::materialise(
        fixtures::programme(4.0, 44_100, -20.0),
        &dir,
        "programme.wav",
    );
    for setting in [Warmth::Gentle, Warmth::Tube] {
        let mut audio = decode::decode_to_f32(&path).expect("decoded");
        let before = measure::integrated_lufs(&audio);
        process::warmth::apply(&mut audio, setting);
        let after = measure::integrated_lufs(&audio);
        assert!(
            (after - before).abs() <= 3.0,
            "{setting:?} moved the level by {:.3} LU, past the 3 dB the chain budgets",
            after - before
        );
    }

    // A steady tone is the case where the drive normalisation really does hold
    // the level exactly, and the one that catches a wrong trim constant: the
    // shapers are scaled so their small-signal gain is one by construction.
    let (tone_path, _) =
        fixtures::materialise(fixtures::tone(440.0, 1.0, 44_100, 0.1, 1), &dir, "tone.wav");
    for setting in [Warmth::Gentle, Warmth::Tube] {
        let mut audio = decode::decode_to_f32(&tone_path).expect("decoded");
        let before = measure::integrated_lufs(&audio);
        process::warmth::apply(&mut audio, setting);
        let after = measure::integrated_lufs(&audio);
        assert!(
            (after - before).abs() < 0.5,
            "{setting:?} moved a steady tone's level by {:.3} LU",
            after - before
        );
    }
}
