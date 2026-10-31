//! Claim 1: a programme-like file processed with the Festival template lands
//! within half a loudness unit of the -12.5 LUFS target, measured back off the
//! written file by an independent R128 pass.

use capstone_lib::models::OutputFormat;
use capstone_lib::templates;

use crate::support::{self, fixtures, measure};

#[test]
fn festival_lands_on_its_target_from_a_real_file() {
    let dir = fixtures::scratch("loudness");
    let (path, source) = fixtures::materialise(
        fixtures::programme(30.0, 44_100, -20.0),
        &dir,
        "programme.wav",
    );

    // The fixture is what it claims to be. Measured through the same meter the
    // assertion below uses, so the two figures are comparable.
    let before = measure::integrated_lufs(&source);
    assert!(
        (before + 20.0).abs() < 0.05,
        "fixture is at {before:.3} LUFS, not -20"
    );

    let template = templates::festival();
    let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
    let run = support::pipeline(&path, &template, &settings);

    let after = measure::integrated_lufs(&run.decoded);
    assert!(
        (after - template.target_lufs).abs() <= 0.5,
        "landed on {after:.3} LUFS against a {:.1} target",
        template.target_lufs
    );

    // The row's own figure has to agree with the independent pass, or the table
    // is telling the user something the file does not say.
    assert!(
        (run.result.lufs_after - after).abs() < 0.3,
        "the report says {:.3} LUFS, the file reads {after:.3}",
        run.result.lufs_after
    );
    assert!(
        (run.result.lufs_before - before).abs() < 0.3,
        "the report says the source was {:.3} LUFS, it was {before:.3}",
        run.result.lufs_before
    );

    // Encoding must not move the level. 16-bit dither sits near -90 dBFS, so
    // anything here beyond a few hundredths is a scaling mistake.
    let in_memory = measure::integrated_lufs(&run.processed);
    assert!(
        (in_memory - after).abs() < 0.1,
        "the encode moved the level: {in_memory:.3} in memory, {after:.3} on disk"
    );

    // A file 7.5 dB under target has to be turned up, not merely left alone.
    assert!(
        run.result.gain_applied_db > 4.0,
        "only {:.2} dB of gain for a -20 LUFS source",
        run.result.gain_applied_db
    );
}

/// The other direction, which is most of a real library: a master already
/// louder than the target has to come down and land on it just as precisely.
#[test]
fn a_loud_master_is_brought_down_onto_the_target() {
    let dir = fixtures::scratch("loudness-down");
    let (path, source) = fixtures::materialise(
        fixtures::limited_master(12.0, 44_100, -8.0),
        &dir,
        "master.wav",
    );
    let before = measure::integrated_lufs(&source);
    assert!(
        (before + 8.0).abs() < 0.05,
        "fixture is at {before:.3} LUFS"
    );

    let template = templates::festival();
    let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
    let run = support::pipeline(&path, &template, &settings);

    let after = measure::integrated_lufs(&run.decoded);
    assert!(
        (after - template.target_lufs).abs() <= 0.5,
        "landed on {after:.3} LUFS against a {:.1} target",
        template.target_lufs
    );
    assert!(
        run.result.gain_applied_db < 0.0,
        "a -8 LUFS master was given {:.2} dB of gain",
        run.result.gain_applied_db
    );
}

/// Re-running a library is something users do, so the second pass has to be a
/// no-op rather than a second helping of processing.
#[test]
fn a_second_pass_over_the_output_changes_almost_nothing() {
    let dir = fixtures::scratch("loudness-twice");
    let (path, _) = fixtures::materialise(
        fixtures::programme(12.0, 44_100, -20.0),
        &dir,
        "programme.wav",
    );
    let template = templates::festival();

    let first = support::pipeline(
        &path,
        &template,
        &support::settings(OutputFormat::Wav, &dir.join("one")),
    );
    let second = support::pipeline(
        &first.output,
        &template,
        &support::settings(OutputFormat::Wav, &dir.join("two")),
    );

    let one = measure::integrated_lufs(&first.decoded);
    let two = measure::integrated_lufs(&second.decoded);
    assert!(
        (one - two).abs() < 0.3,
        "{one:.3} LUFS then {two:.3} LUFS: the second pass is not idempotent"
    );
    assert_eq!(
        second.result.clips_repaired, 0,
        "the second pass found clipping in Capstone's own output"
    );
}
