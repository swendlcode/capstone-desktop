//! Claim 3: a file detuned by exactly +30 cents comes back within 2 cents of
//! concert pitch; `keep_tempo` holds the duration to within 0.5%, and
//! `change_tempo` moves it in the direction the arithmetic predicts.
//!
//! The residual is measured by following one known partial with an FFT and a
//! parabolic refinement, not by asking Capstone's own histogram estimator. A
//! tuning test that measured with the estimator under test would pass on a
//! chain that did nothing at all.

use capstone_lib::models::{OutputFormat, PitchMode, Template};
use capstone_lib::templates;

use crate::support::{self, fixtures, measure};

const DETUNE_CENTS: f64 = 30.0;
/// The partial the measurement follows: loud, well clear of its neighbours,
/// and high enough that a bin is a small fraction of a cent wide.
const REFERENCE_HZ: f64 = 440.0;
const SPAN_HZ: f64 = 15.0;

fn template(mode: PitchMode) -> Template {
    Template {
        correct_pitch: true,
        pitch_mode: mode,
        ..templates::festival()
    }
}

/// How far off concert pitch the reference partial sits, in cents.
fn residual_cents(audio: &capstone_lib::audio::DecodedAudio) -> f64 {
    let measured = measure::partial_frequency(
        &audio.channels[0],
        audio.sample_rate as f64,
        REFERENCE_HZ,
        SPAN_HZ,
    );
    measure::cents_between(measured, REFERENCE_HZ)
}

#[test]
fn keep_tempo_pulls_the_file_into_tune_without_moving_its_length() {
    let dir = fixtures::scratch("pitch-keep");
    let (signal, _) = fixtures::detuned(DETUNE_CENTS, 8.0, 44_100);
    let frames_in = signal.frame_count();
    let (path, before) = fixtures::materialise(signal, &dir, "detuned.wav");

    // The fixture is exactly where it claims to be.
    let offset = residual_cents(&before);
    assert!(
        (offset - DETUNE_CENTS).abs() < 0.5,
        "fixture sits {offset:.3} cents out, not {DETUNE_CENTS}"
    );

    let template = template(PitchMode::KeepTempo);
    let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
    let run = support::pipeline(&path, &template, &settings);

    // The correction is reported, and reported the right way round: a file
    // measured sharp is shifted down.
    assert!(
        (run.result.pitch_corrected_cents + DETUNE_CENTS).abs() < 3.0,
        "reported {:.3} cents of correction",
        run.result.pitch_corrected_cents
    );

    let residual = residual_cents(&run.decoded);
    assert!(
        residual.abs() <= 0.5,
        "left {residual:.3} cents of detune in the written file"
    );

    let drift = (run.decoded.frame_count() as f64 / frames_in as f64 - 1.0).abs();
    assert!(
        drift <= 0.005,
        "duration moved by {:.4}%, over the 0.5% keep_tempo allows",
        drift * 100.0
    );
}

#[test]
fn change_tempo_pulls_it_into_tune_and_lengthens_it_as_predicted() {
    let dir = fixtures::scratch("pitch-change");
    let (signal, _) = fixtures::detuned(DETUNE_CENTS, 8.0, 44_100);
    let frames_in = signal.frame_count();
    let (path, _) = fixtures::materialise(signal, &dir, "detuned.wav");

    let template = template(PitchMode::ChangeTempo);
    let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
    let run = support::pipeline(&path, &template, &settings);

    let applied = run.result.pitch_corrected_cents;
    assert!(
        (applied + DETUNE_CENTS).abs() < 3.0,
        "reported {applied:.3} cents of correction"
    );

    let residual = residual_cents(&run.decoded);
    assert!(
        residual.abs() <= 0.5,
        "left {residual:.3} cents of detune in the written file"
    );

    // Resampling to drop the pitch by `applied` cents reads the input more
    // slowly, so the output is longer by exactly the reciprocal of the pitch
    // multiplier: 2^(-applied/1200).
    let predicted = 2f64.powf(-applied / 1_200.0);
    let actual = run.decoded.frame_count() as f64 / frames_in as f64;
    assert!(
        actual > 1.0,
        "a downward shift should lengthen the file, ratio was {actual:.5}"
    );
    assert!(
        (actual - predicted).abs() < 0.004,
        "length ratio {actual:.5}, arithmetic predicts {predicted:.5}"
    );
}

/// The whole range the fold can answer, in both modes, because the error used
/// to be erratic rather than proportional: `keep_tempo` left 4.08 cents of a
/// 10-cent correction and 0.04 of a 50-cent one, so a single fixture at 30
/// cents - which happened to be one of its better results, at 1.48 - would
/// have reported the stage as working.
///
/// The range stops short of 50 because 50 is the fold's own seam; the test
/// below pins what happens there.
#[test]
fn both_modes_are_accurate_across_every_shift_the_fold_can_answer() {
    for mode in [PitchMode::KeepTempo, PitchMode::ChangeTempo] {
        for detune in [12.0f64, 20.0, 30.0, 40.0, 45.0] {
            let dir = fixtures::scratch("pitch-sweep");
            let (signal, _) = fixtures::detuned(detune, 8.0, 44_100);
            let frames_in = signal.frame_count();
            let (path, _) = fixtures::materialise(signal, &dir, "detuned.wav");

            let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
            let run = support::pipeline(&path, &template(mode), &settings);

            let residual = residual_cents(&run.decoded);
            assert!(
                residual.abs() <= 0.5,
                "{mode:?} at {detune} cents left {residual:.3} cents"
            );
            if mode == PitchMode::KeepTempo {
                let drift = (run.decoded.frame_count() as f64 / frames_in as f64 - 1.0).abs();
                assert!(
                    drift <= 0.005,
                    "{mode:?} at {detune} cents moved the duration by {:.4}%",
                    drift * 100.0
                );
            }
        }
    }
}

/// Exactly half a semitone is the one input the method cannot answer, pinned
/// here so it stays a known limit rather than a surprise.
///
/// Tuning is measured by folding every partial onto the nearest equal-tempered
/// semitone, and that fold puts -50 and +50 cents at the same place. At exactly
/// 50 the estimator has to pick, and it picks flat: a file 50 cents sharp is
/// read as 50 cents flat, corrected the wrong way, and comes back a full
/// semitone out. Nothing in the partials distinguishes the two cases, so this
/// is a property of the question rather than of the estimator - which is why
/// `MAX_CENTS` bounds the correction at 50 and why 60 cents is treated as a
/// different key rather than a mistuning.
#[test]
fn exactly_half_a_semitone_is_the_seam_the_fold_cannot_resolve() {
    let dir = fixtures::scratch("pitch-seam");
    let (signal, _) = fixtures::detuned(50.0, 8.0, 44_100);
    let (path, before) = fixtures::materialise(signal, &dir, "seam.wav");

    // The fixture really is 50 cents sharp, and the estimator really does read
    // it as 50 flat.
    let offset = residual_cents(&before);
    assert!(
        (offset - 50.0).abs() < 0.5,
        "fixture sits {offset:.3} cents out"
    );
    let estimated = capstone_lib::audio::analyze::tuning_cents(&before);
    assert!(
        (estimated + 50.0).abs() < 0.5,
        "the estimator read {estimated:.2} cents, not the -50 the fold forces"
    );

    // So the correction goes the long way round, and the shift it applies is
    // still exactly the shift it reported. That is the part worth holding: the
    // measurement is ambiguous, the stage that acts on it is not.
    let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
    let run = support::pipeline(&path, &template(PitchMode::KeepTempo), &settings);
    assert!(
        (run.result.pitch_corrected_cents - 50.0).abs() < 0.5,
        "reported {:.2} cents",
        run.result.pitch_corrected_cents
    );
    let landed = measure::cents_between(
        measure::partial_frequency(&run.decoded.channels[0], 44_100.0, 440.0, 60.0),
        440.0,
    );
    assert!(
        (landed - 100.0).abs() < 1.0,
        "landed {landed:.2} cents from concert pitch, not the 100 the fold implies"
    );
}

/// The gate: 4 cents is inside the default 10-cent tolerance, so the file has
/// to come back untouched. Nudging something already in tune is worse than
/// leaving it, and it would also move the duration in `change_tempo`.
#[test]
fn a_nearly_in_tune_file_is_not_touched_in_either_mode() {
    for mode in [PitchMode::KeepTempo, PitchMode::ChangeTempo] {
        let dir = fixtures::scratch("pitch-tolerance");
        let (signal, _) = fixtures::detuned(4.0, 8.0, 44_100);
        let frames_in = signal.frame_count();
        let (path, _) = fixtures::materialise(signal, &dir, "nearly.wav");

        let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
        let run = support::pipeline(&path, &template(mode), &settings);
        assert_eq!(
            run.result.pitch_corrected_cents, 0.0,
            "{mode:?} corrected a file inside the tolerance"
        );
        assert_eq!(
            run.decoded.frame_count(),
            frames_in,
            "{mode:?} changed the length of a file it was not meant to touch"
        );
    }
}
