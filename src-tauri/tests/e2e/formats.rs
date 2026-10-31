//! Claim 7: a round trip through every supported output format, confirming
//! each file is valid and decodes back to the expected length and rate.
//!
//! "Valid" means the reader the app itself uses can reopen it, which is the
//! only definition that matters here: a file Capstone writes and cannot read
//! back is a file the user's next run will report as broken.

use capstone_lib::audio::encode;
use capstone_lib::jobs::output::bit_depth_for;
use capstone_lib::models::OutputFormat;
use capstone_lib::templates;

use crate::support::{self, fixtures, measure};

const RATE: u32 = 44_100;
const SECONDS: f64 = 5.0;

/// Every format in the contract. MP3 sits behind a Cargo feature, so it is in
/// the list only when the build has it - and `encode` refuses it explicitly
/// otherwise, which the last test here checks.
fn formats() -> Vec<OutputFormat> {
    let mut list = vec![
        OutputFormat::Aiff,
        OutputFormat::Alac,
        OutputFormat::Flac,
        OutputFormat::Wav,
    ];
    if cfg!(feature = "mp3") {
        list.push(OutputFormat::Mp3);
    }
    list
}

/// How far a format is allowed to move the loudness. The lossless four should
/// not move it at all beyond dither; MP3 is a lossy codec and is given room.
fn loudness_slack(format: OutputFormat) -> f64 {
    match format {
        OutputFormat::Mp3 => 0.6,
        _ => 0.1,
    }
}

#[test]
fn every_output_format_round_trips() {
    let dir = fixtures::scratch("formats");
    let (path, source) = fixtures::materialise(
        fixtures::programme(SECONDS, RATE, -20.0),
        &dir,
        "programme.wav",
    );
    let frames_in = source.frame_count();
    let template = templates::festival();

    for format in formats() {
        let settings = support::settings(format, &dir.join(format!("{format:?}")));
        let run = support::pipeline(&path, &template, &settings);

        // The file is where the app said it would be, carries the right
        // extension, and is not a stub.
        assert_eq!(
            run.output.extension().and_then(|e| e.to_str()),
            Some(encode::extension(format)),
            "{format:?} did not land on its own extension"
        );
        let size = std::fs::metadata(&run.output)
            .unwrap_or_else(|e| panic!("{format:?}: {e}"))
            .len();
        assert!(size > 10_000, "{format:?} wrote only {size} bytes");
        // And an upper bound, which is not pedantry: a FLAC encode of loud
        // 24-bit audio used to come out at 559 times the raw PCM size - 370 MB
        // for five seconds of mono - because one frame was coded at thousands
        // of bits per sample. The file was still readable by the reference
        // decoder, so nothing short of looking at the size caught it.
        let raw = frames_in as u64 * 2 * u64::from(bit_depth_for(format) / 8);
        assert!(
            size < raw + raw / 8 + 64 * 1024,
            "{format:?} wrote {size} bytes for {raw} bytes of audio"
        );

        // It decodes back at the rate and length it went in at. No stage in the
        // chain changes either, so an exact match is the right expectation -
        // including for MP3, whose encoder writes a Xing/Info tag the reader
        // applies, which is what keeps a DJ's cue points where they were.
        assert_eq!(
            run.decoded.sample_rate, RATE,
            "{format:?} came back at {} Hz",
            run.decoded.sample_rate
        );
        assert_eq!(
            run.decoded.channel_count(),
            2,
            "{format:?} came back with {} channels",
            run.decoded.channel_count()
        );
        assert_eq!(
            run.decoded.frame_count(),
            frames_in,
            "{format:?} came back {} frames long, expected {frames_in}",
            run.decoded.frame_count()
        );

        // And it still hits the target it was processed to.
        let after = measure::integrated_lufs(&run.decoded);
        let slack = loudness_slack(format);
        assert!(
            (after - template.target_lufs).abs() <= 0.5 + slack,
            "{format:?} reads {after:.3} LUFS against a {:.1} target",
            template.target_lufs
        );
        assert!(
            (after - run.result.lufs_after).abs() <= slack,
            "{format:?}: the report says {:.3} LUFS, the file reads {after:.3}",
            run.result.lufs_after
        );

        // Nothing broke on the way through.
        assert!(
            run.decoded.channels.iter().flatten().all(|s| s.is_finite()),
            "{format:?} decoded to a non-finite sample"
        );
        // The ceiling survives the encode, lossy coding included: the slack is
        // the codec's, not the limiter's.
        let ceiling = support::ceiling_db(&template);
        let peak = measure::true_peak_dbtp(&run.decoded);
        let peak_slack = if format == OutputFormat::Mp3 {
            1.0
        } else {
            0.1
        };
        assert!(
            peak <= ceiling + peak_slack,
            "{format:?} peaks at {peak:.3} dBTP over a {ceiling:.1} dB ceiling"
        );
    }
}

/// A 96 kHz source through every format. MP3 has nine legal rates and 96 kHz
/// is not one of them, so that one is expected to land at 48 kHz; everything
/// else keeps the rate it arrived at.
#[test]
fn a_high_rate_source_round_trips_through_every_format() {
    let dir = fixtures::scratch("formats-96k");
    let (path, source) = fixtures::materialise(
        fixtures::programme(2.0, 96_000, -20.0),
        &dir,
        "programme.wav",
    );
    let frames_in = source.frame_count();
    let template = templates::festival();

    for format in formats() {
        let settings = support::settings(format, &dir.join(format!("{format:?}")));
        let run = support::pipeline(&path, &template, &settings);
        if format == OutputFormat::Mp3 {
            assert_eq!(
                run.decoded.sample_rate, 48_000,
                "MP3 should have been brought into a legal rate"
            );
        } else {
            assert_eq!(
                run.decoded.sample_rate, 96_000,
                "{format:?} changed the sample rate"
            );
            assert_eq!(
                run.decoded.frame_count(),
                frames_in,
                "{format:?} changed the length"
            );
        }
        assert!(run.decoded.channels.iter().flatten().all(|s| s.is_finite()));
    }
}

/// MP3 behind a Cargo feature is only safe if the refusal is explicit. A build
/// without it has to say so rather than write a file with the wrong bytes in it.
#[cfg(not(feature = "mp3"))]
#[test]
fn mp3_is_refused_rather_than_mishandled_without_the_feature() {
    let dir = fixtures::scratch("formats-no-mp3");
    let (path, _) =
        fixtures::materialise(fixtures::programme(1.0, RATE, -20.0), &dir, "programme.wav");
    let settings = support::settings(OutputFormat::Mp3, &dir.join("out"));
    let outcome = support::try_pipeline(&path, &templates::festival(), &settings);
    let message = outcome.err().expect("mp3 should have been refused");
    assert!(
        message.contains("MP3"),
        "the refusal does not mention MP3: {message}"
    );
}
