//! Tests for FLAC output, including the two faults in flacenc's own behaviour
//! that `flac::write` corrects.
//!
//! They live here rather than with the other round trips because both faults
//! need content and a channel count the shared tone fixture does not have:
//! loud broadband material at 24 bits, in mono.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::audio::decode;
use crate::audio::encode::{encode, EncodeSpec};
use crate::models::{Mp3Rate, OutputFormat};

const RATE: u32 = 44_100;
const SECONDS: f64 = 3.0;

/// A fresh path per call: these tests run on parallel threads in one process,
/// so a shared filename would have two of them writing the same file.
fn scratch() -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    std::env::temp_dir().join(format!(
        "capstone-flac-{}-{}.flac",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ))
}

fn spec(bit_depth: u16) -> EncodeSpec {
    EncodeSpec {
        sample_rate: RATE,
        channels: 2,
        bit_depth,
        mp3_rate: Mp3Rate::Cbr320,
    }
}

/// Deterministic broadband noise near full scale: the content a lossless
/// encoder has the hardest time with, and the content both faults need.
fn loud_noise(channels: usize) -> DecodedAudio {
    let frames = (f64::from(RATE) * SECONDS) as usize;
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    let samples: Vec<f32> = (0..frames)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (((state >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0) * 0.9) as f32
        })
        .collect();
    DecodedAudio::new(vec![samples; channels], RATE)
}

fn raw_size(audio: &DecodedAudio, bit_depth: u16) -> u64 {
    (audio.frame_count() * audio.channels.len() * (bit_depth as usize / 8)) as u64
}

/// Mono was never covered, and mono is where it showed: flacenc estimates each
/// Rice parameter's cost in a table saturated at 2^28 and breaks a tie toward
/// the *smaller* parameter, which is the most expensive code there is rather
/// than the cheapest. One 4,096-sample frame of 24-bit mono came out at 369 MB
/// inside an otherwise valid stream - the reference decoder read it, symphonia
/// skipped it, and the file reopened 4,096 frames short with no error reported
/// anywhere.
#[test]
fn flac_round_trips_in_mono() {
    for depth in [16u16, 24] {
        let path = scratch();
        let source = loud_noise(1);
        encode(&source, &spec(depth), OutputFormat::Flac, &path)
            .unwrap_or_else(|e| panic!("{depth}-bit mono flac failed: {e}"));
        let decoded = decode::decode_to_f32(&path).unwrap_or_else(|e| panic!("{depth}-bit: {e}"));
        let _ = std::fs::remove_file(&path);
        assert_eq!(decoded.channel_count(), 1);
        assert_eq!(
            decoded.frame_count(),
            source.frame_count(),
            "{depth}-bit mono flac came back {} frames instead of {}",
            decoded.frame_count(),
            source.frame_count()
        );
    }
}

/// The size bound that catches the same fault however it is reached.
///
/// Not `< raw`, because full-scale white noise is genuinely incompressible and
/// a verbatim subframe carries an extra bit per sample: 16-bit mono noise
/// measures 1.0013 times raw here, which is correct behaviour. 24-bit noise
/// measures 0.90, and the frame that went wrong measured 559.
#[test]
fn flac_never_grows_past_the_samples_it_encodes() {
    for channels in [1usize, 2] {
        for depth in [16u16, 24] {
            let path = scratch();
            let source = loud_noise(channels);
            encode(&source, &spec(depth), OutputFormat::Flac, &path)
                .unwrap_or_else(|e| panic!("{channels}ch {depth}-bit: {e}"));
            let written = std::fs::metadata(&path).unwrap().len();
            let _ = std::fs::remove_file(&path);
            let raw = raw_size(&source, depth);
            assert!(
                written < raw + raw / 8 + 64 * 1024,
                "{channels}ch {depth}-bit flac wrote {written} bytes for {raw} bytes of audio"
            );
        }
    }
}

/// The refusal itself, so the guard cannot quietly stop guarding. Exercised
/// directly rather than by hunting for content that still trips flacenc, since
/// a stream of that shape can only come from the encoder going wrong.
#[test]
fn a_bloated_stream_is_refused_rather_than_written() {
    let source = loud_noise(1);
    let raw = raw_size(&source, 24) as usize;
    assert!(would_reject(raw * 600, &source, 24));
    assert!(!would_reject(raw, &source, 24));
}
