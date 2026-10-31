//! Claim 8: nothing panics on any edge case, and silence stays silent rather
//! than having noise amplified into it by the normaliser.
//!
//! The second half is the one with a real bug behind it. A normaliser that
//! measures a near-silent file, finds it 60 dB under target and applies 60 dB
//! of gain turns the file's own noise floor into dance-floor-level hiss. The
//! chain's floor is the loudness standard's own absolute gate, and this is
//! what checks it is actually there.

use capstone_lib::audio::{analyze, DecodedAudio};
use capstone_lib::models::{OutputFormat, PitchMode, Template, Warmth};
use capstone_lib::templates;

use crate::support::{self, fixtures, measure};

/// Every stage switched on, so an edge case cannot slip past a branch that was
/// never entered: clip repair, pitch correction in the mode that changes
/// length, the four-band bank, and the harder of the two shapers.
fn everything_on() -> Template {
    Template {
        repair_clips: true,
        correct_pitch: true,
        pitch_mode: PitchMode::ChangeTempo,
        warmth: Warmth::Tube,
        ..templates::festival()
    }
}

/// The edge cases from the brief, each built here rather than read off disk.
fn cases() -> Vec<(&'static str, DecodedAudio)> {
    let programme = fixtures::programme(5.0, 44_100, -20.0);
    vec![
        ("pure silence", fixtures::silence(2.0, 44_100, 2)),
        // 0.2 s is shorter than one 400 ms gating block, so the integrated
        // loudness has no block to average and the meter answers -inf.
        ("0.2 s file", fixtures::tone(1_000.0, 0.2, 44_100, 0.3, 2)),
        ("mono", fixtures::to_mono(&programme, -20.0)),
        ("96 kHz", fixtures::programme(3.0, 96_000, -20.0)),
        (
            "already-limited -8 LUFS master",
            fixtures::limited_master(6.0, 44_100, -8.0),
        ),
    ]
}

#[test]
fn every_edge_case_survives_the_whole_pipeline() {
    for template in [templates::festival(), everything_on()] {
        for (name, signal) in cases() {
            let dir = fixtures::scratch("edges");
            let channels_in = signal.channel_count();
            let rate_in = signal.sample_rate;
            let (path, _) = fixtures::materialise(signal, &dir, "source.wav");

            for format in [OutputFormat::Wav, OutputFormat::Flac] {
                let settings = support::settings(format, &dir.join(format!("{format:?}")));
                let run = support::try_pipeline(&path, &template, &settings)
                    .unwrap_or_else(|e| panic!("{name} as {format:?}: {e}"));

                assert!(
                    run.processed
                        .channels
                        .iter()
                        .flatten()
                        .all(|s| s.is_finite()),
                    "{name} as {format:?}: the chain produced a non-finite sample"
                );
                assert!(
                    run.decoded.channels.iter().flatten().all(|s| s.is_finite()),
                    "{name} as {format:?}: the written file holds a non-finite sample"
                );
                assert_eq!(
                    run.decoded.channel_count(),
                    channels_in,
                    "{name} as {format:?}: channel count changed"
                );
                assert_eq!(
                    run.decoded.sample_rate, rate_in,
                    "{name} as {format:?}: sample rate changed"
                );
                assert!(
                    run.result.lufs_before.is_finite() && run.result.lufs_after.is_finite(),
                    "{name} as {format:?}: the report carries a non-finite loudness"
                );
                assert!(
                    run.result.gain_applied_db.is_finite(),
                    "{name} as {format:?}: the report carries a non-finite gain"
                );

                // Analysis has to survive the same input, since it runs on every
                // row the moment a file is added.
                let analysis = analyze::analyze(&run.decoded)
                    .unwrap_or_else(|e| panic!("{name} as {format:?}: analysis failed: {e}"));
                assert!(
                    analysis.lufs.is_finite()
                        && analysis.true_peak_db.is_finite()
                        && analysis.crest_db.is_finite()
                        && analysis.tuning_cents.is_finite(),
                    "{name} as {format:?}: analysis returned a non-finite figure"
                );
            }
        }
    }
}

/// Silence in, silence out. Exactly, in the buffer the chain produces: no gain
/// is applied, so there is nothing to amplify and nothing to amplify it into.
#[test]
fn silence_stays_silent() {
    for template in [templates::festival(), everything_on()] {
        let dir = fixtures::scratch("edges-silence");
        let (path, _) =
            fixtures::materialise(fixtures::silence(3.0, 44_100, 2), &dir, "silence.wav");

        let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
        let run = support::pipeline(&path, &template, &settings);

        assert_eq!(
            run.result.gain_applied_db, 0.0,
            "silence was given {:.3} dB of gain",
            run.result.gain_applied_db
        );
        assert!(
            run.processed
                .channels
                .iter()
                .flatten()
                .all(|sample| *sample == 0.0),
            "the chain put something into silence: peak {:.2} dBFS",
            measure::sample_peak_dbfs(&run.processed)
        );

        // On disk the only thing in the file is the 16-bit dither the encoder
        // adds on purpose, which is one least-significant bit: about -90 dBFS.
        // Anything louder than -80 is the normaliser, not the dither.
        let written = measure::sample_peak_dbfs(&run.decoded);
        assert!(
            written < -80.0,
            "the written file holds {written:.2} dBFS of something"
        );
    }
}

/// A near-silent file is the same bug with a quieter fixture: there is a real
/// signal in it, so the gate rather than a zero check has to be what stops the
/// gain. At -75 LUFS the chain must refuse to move it, since bringing it to
/// -12.5 would mean 62 dB of gain on a noise floor.
#[test]
fn a_file_under_the_absolute_gate_is_not_amplified() {
    let dir = fixtures::scratch("edges-floor");
    let (path, source) =
        fixtures::materialise(fixtures::programme(5.0, 44_100, -75.0), &dir, "whisper.wav");
    // Measured as RMS, not as integrated loudness: -75 LUFS is below the
    // standard's own -70 absolute gate, so every 400 ms block is excluded and
    // a gated meter answers -inf for this file and for silence alike. RMS is
    // also the quantity the claim is about, since the multiband bank sums to
    // allpass: it moves this fixture's sample peak by 2.5 dB while moving no
    // energy at all, which is not the normaliser doing anything.
    let before = measure::rms_dbfs(&source);
    assert!(
        !measure::integrated_lufs(&source).is_finite(),
        "the fixture is meant to sit under the absolute gate"
    );

    let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
    let run = support::pipeline(&path, &templates::festival(), &settings);

    assert_eq!(
        run.result.gain_applied_db, 0.0,
        "a -75 LUFS file was given {:.2} dB of gain",
        run.result.gain_applied_db
    );

    let after = measure::rms_dbfs(&run.processed);
    assert!(
        (after - before).abs() <= 1.0,
        "the energy moved from {before:.2} to {after:.2} dBFS RMS"
    );
}
