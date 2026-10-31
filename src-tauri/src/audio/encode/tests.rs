//! Round trips: every format Capstone writes, decoded back through the same
//! reader the app itself uses.
//!
//! The signal is a 440 Hz tone at two different amplitudes, 0.5 on the left and
//! 0.25 on the right. A level check per channel then catches an interleaving or
//! channel-order mistake without needing an FFT, which a single-amplitude tone
//! would hide.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::audio::decode;
use crate::audio::decode::TagField;

const RATE: u32 = 44_100;
const SECONDS: f64 = 0.5;
const LEFT: f32 = 0.5;
const RIGHT: f32 = 0.25;

fn tone(rate: u32) -> DecodedAudio {
    let frames = (f64::from(rate) * SECONDS) as usize;
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    for frame in 0..frames {
        let phase = std::f64::consts::TAU * 440.0 * frame as f64 / f64::from(rate);
        let value = phase.sin() as f32;
        left.push(value * LEFT);
        right.push(value * RIGHT);
    }
    DecodedAudio::new(vec![left, right], rate)
}

fn spec(bit_depth: u16, rate: u32, mp3_rate: Mp3Rate) -> EncodeSpec {
    EncodeSpec {
        sample_rate: rate,
        channels: 2,
        bit_depth,
        mp3_rate,
    }
}

fn sample_tags() -> SourceTags {
    let mut tags = SourceTags::default();
    tags.insert(TagField::Title, "Tone Test");
    tags.insert(TagField::Artist, "Capstone");
    tags.insert(TagField::Date, "2019-05-02");
    tags
}

/// A fresh path per call: the tests run in parallel threads in one process, so
/// a shared filename would have two of them writing the same file.
fn scratch(format: OutputFormat) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "capstone-encode-{}-{}.{}",
        std::process::id(),
        n,
        extension(format)
    ))
}

/// Encode the tone, decode it back, delete the file, hand back what came out.
fn round_trip(format: OutputFormat, spec: &EncodeSpec) -> (DecodedAudio, SourceTags) {
    let path = scratch(format);
    let source = tone(RATE);
    encode_with_tags(&source, spec, format, &path, &sample_tags())
        .unwrap_or_else(|e| panic!("encoding {:?} failed: {}", format, e));
    let decoded = decode::decode_with_tags(&path)
        .unwrap_or_else(|e| panic!("reading back {:?} failed: {}", format, e));
    let _ = std::fs::remove_file(&path);
    decoded
}

fn channel_peak(audio: &DecodedAudio, channel: usize) -> f32 {
    audio.channels[channel]
        .iter()
        .fold(0.0f32, |acc, s| acc.max(s.abs()))
}

/// Shape and levels, with a tolerance a format is allowed to spend.
fn assert_tone(audio: &DecodedAudio, rate: u32, level_tolerance: f32, frame_tolerance: usize) {
    assert_eq!(audio.sample_rate, rate, "sample rate changed");
    assert_eq!(audio.channel_count(), 2, "channel count changed");

    let expected = (f64::from(rate) * SECONDS) as usize;
    let drift = audio.frame_count().abs_diff(expected);
    assert!(
        drift <= frame_tolerance,
        "{} frames, expected about {}",
        audio.frame_count(),
        expected
    );

    let (left, right) = (channel_peak(audio, 0), channel_peak(audio, 1));
    assert!(
        (left - LEFT).abs() < level_tolerance,
        "left peak {}, expected {}",
        left,
        LEFT
    );
    assert!(
        (right - RIGHT).abs() < level_tolerance,
        "right peak {}, expected {} (channels swapped?)",
        right,
        RIGHT
    );
}

fn assert_tags_survived(tags: &SourceTags) {
    let find = |field: TagField| {
        tags.entries()
            .iter()
            .find(|(f, _)| *f == field)
            .map(|(_, v)| v.as_str())
    };
    assert_eq!(find(TagField::Title), Some("Tone Test"));
    assert_eq!(find(TagField::Artist), Some("Capstone"));
}

#[test]
fn wav_16_bit_round_trips() {
    let (audio, _) = round_trip(OutputFormat::Wav, &spec(16, RATE, Mp3Rate::Cbr320));
    // One LSB of 16-bit is 3e-5; the dither adds one more. 0.001 is loose
    // enough not to be flaky and tight enough to catch a scaling mistake.
    assert_tone(&audio, RATE, 0.001, 0);
}

#[test]
fn wav_24_bit_round_trips() {
    let (audio, _) = round_trip(OutputFormat::Wav, &spec(24, RATE, Mp3Rate::Cbr320));
    assert_tone(&audio, RATE, 0.0001, 0);
}

/// The null test from the DSP plan: nothing in the float path may scale, round
/// or dither, so the samples that come back must be the samples that went in.
/// Written with tags, so it also proves splicing the INFO chunk in front of the
/// audio leaves the audio alone.
#[test]
fn wav_float_is_bit_transparent() {
    let path = scratch(OutputFormat::Wav);
    let source = tone(RATE);
    encode_with_tags(
        &source,
        &spec(32, RATE, Mp3Rate::Cbr320),
        OutputFormat::Wav,
        &path,
        &sample_tags(),
    )
    .unwrap();
    let decoded = decode::decode_to_f32(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(
        decoded.channels, source.channels,
        "float path is not lossless"
    );
}

#[test]
fn aiff_round_trips_with_tags() {
    let (audio, tags) = round_trip(OutputFormat::Aiff, &spec(16, RATE, Mp3Rate::Cbr320));
    assert_tone(&audio, RATE, 0.001, 0);
    assert_tags_survived(&tags);
}

#[test]
fn flac_round_trips_with_tags() {
    let (audio, tags) = round_trip(OutputFormat::Flac, &spec(24, RATE, Mp3Rate::Cbr320));
    assert_tone(&audio, RATE, 0.0001, 0);
    assert_tags_survived(&tags);
}

/// The reason `fix_flac_streaminfo` exists. Half a second at 44.1 kHz is
/// 22,050 frames, which is not a multiple of the 4,096-sample block size, so
/// this is exactly the case that used to produce a file Capstone could not
/// reopen. `round_trip` reading it back at all is the assertion.
#[test]
fn flac_with_a_short_final_frame_reopens() {
    let (audio, _) = round_trip(OutputFormat::Flac, &spec(16, RATE, Mp3Rate::Cbr320));
    assert_eq!(audio.frame_count(), (f64::from(RATE) * SECONDS) as usize);
}

#[test]
fn alac_round_trips() {
    let (audio, _) = round_trip(OutputFormat::Alac, &spec(16, RATE, Mp3Rate::Cbr320));
    assert_tone(&audio, RATE, 0.001, 0);
}

#[cfg(feature = "mp3")]
#[test]
fn mp3_cbr_round_trips_with_tags() {
    let (audio, tags) = round_trip(OutputFormat::Mp3, &spec(16, RATE, Mp3Rate::Cbr320));
    // Lossy, so the level is allowed to move. The length is not: rusty_mp3
    // writes a correct Xing/Info tag and the reader applies it, so the round
    // trip is gapless to the sample. A drift here means the tag is wrong, and a
    // DJ with cue points would find every one of them moved.
    assert_tone(&audio, RATE, 0.05, 0);
    assert_tags_survived(&tags);
}

#[cfg(feature = "mp3")]
#[test]
fn mp3_vbr_round_trips() {
    let (audio, _) = round_trip(OutputFormat::Mp3, &spec(16, RATE, Mp3Rate::Vbr));
    assert_tone(&audio, RATE, 0.05, 0);
}

/// An output rate the spec asks for that the buffer is not at. 48 kHz out of a
/// 44.1 kHz tone is the conversion a DJ library actually hits.
#[test]
fn a_different_output_rate_resamples() {
    let path = scratch(OutputFormat::Wav);
    let source = tone(RATE);
    encode(
        &source,
        &spec(24, 48_000, Mp3Rate::Cbr320),
        OutputFormat::Wav,
        &path,
    )
    .unwrap();
    let decoded = decode::decode_to_f32(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_tone(&decoded, 48_000, 0.01, 64);
}

/// MP3 has nine legal rates and 96 kHz is not one of them. Without the snap in
/// `target_rate` the encoder refuses the file outright.
#[cfg(feature = "mp3")]
#[test]
fn an_illegal_mp3_rate_is_brought_into_range() {
    let path = scratch(OutputFormat::Mp3);
    let source = tone(96_000);
    encode(
        &source,
        &spec(16, 96_000, Mp3Rate::Cbr320),
        OutputFormat::Mp3,
        &path,
    )
    .unwrap();
    let decoded = decode::decode_to_f32(&path).unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(decoded.sample_rate, 48_000);
}

#[test]
fn an_empty_buffer_is_refused_rather_than_written() {
    let path = scratch(OutputFormat::Wav);
    let empty = DecodedAudio::new(vec![Vec::new(), Vec::new()], RATE);
    let error = encode(
        &empty,
        &spec(16, RATE, Mp3Rate::Cbr320),
        OutputFormat::Wav,
        &path,
    );
    assert!(error.is_err());
    assert!(!path.exists(), "an empty buffer left a file behind");
}

#[test]
fn a_bit_depth_no_format_has_is_refused() {
    let path = scratch(OutputFormat::Wav);
    let error = encode(
        &tone(RATE),
        &spec(12, RATE, Mp3Rate::Cbr320),
        OutputFormat::Wav,
        &path,
    );
    let _ = std::fs::remove_file(&path);
    assert!(matches!(error, Err(CapstoneError::Unsupported(_))));
}

#[test]
fn the_flac_fix_rejects_something_that_is_not_flac() {
    let mut bytes = b"RIFF....WAVEfmt ".to_vec();
    assert!(fix_flac_streaminfo(&mut bytes).is_err());
}
