//! Decoder behaviour that the round trips in `encode::tests` do not reach: what
//! the queue accepts, how duplicate tags resolve, and what a broken file does.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

fn scratch(extension: &str) -> PathBuf {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "capstone-decode-{}-{}.{}",
        std::process::id(),
        n,
        extension
    ))
}

#[test]
fn the_queue_filter_ignores_case_and_anything_unplayable() {
    assert!(is_audio_file(Path::new("/x/Track.WAV")));
    assert!(is_audio_file(Path::new("/x/track.mp3")));
    assert!(is_audio_file(Path::new("/x/track.m4a")));
    assert!(!is_audio_file(Path::new("/x/notes.txt")));
    assert!(!is_audio_file(Path::new("/x/cover.jpg")));
    assert!(!is_audio_file(Path::new("/x/no-extension")));
}

#[test]
fn the_first_value_for_a_field_is_the_one_kept() {
    let mut tags = SourceTags::default();
    // An ID3v2 header is read before the ID3v1 trailer, and the trailer holds a
    // version of the same title cut to 30 bytes.
    tags.insert(TagField::Title, "The Whole Title As Tagged");
    tags.insert(TagField::Title, "The Whole Title As Tagg");
    assert_eq!(tags.entries().len(), 1);
    assert_eq!(tags.entries()[0].1, "The Whole Title As Tagged");
}

#[test]
fn blank_and_padded_values_are_cleaned_up() {
    let mut tags = SourceTags::default();
    tags.insert(TagField::Artist, "   ");
    assert!(tags.is_empty(), "whitespace became a tag");
    tags.insert(TagField::Artist, "  Spaced Out  ");
    assert_eq!(tags.entries()[0].1, "Spaced Out");
}

#[test]
fn a_missing_file_is_an_error_not_a_panic() {
    let error = decode_to_f32(Path::new("/nowhere/at/all.wav"));
    assert!(matches!(error, Err(CapstoneError::Io(_))));
}

/// The single most likely real-world shape of a broken file: the extension
/// promises audio and the bytes are something else entirely.
#[test]
fn a_file_that_is_not_audio_is_an_error_not_a_panic() {
    let path = scratch("wav");
    std::fs::write(&path, b"this is a text file wearing a wav extension").unwrap();
    let error = decode_to_f32(&path);
    let _ = std::fs::remove_file(&path);
    assert!(error.is_err());
}

#[test]
fn a_truncated_file_is_an_error_not_a_panic() {
    let path = scratch("wav");
    // A valid RIFF header and then nothing, which is what a copy interrupted
    // part way through looks like.
    std::fs::write(&path, b"RIFF\x24\x00\x00\x00WAVEfmt ").unwrap();
    let error = decode_to_f32(&path);
    let _ = std::fs::remove_file(&path);
    assert!(error.is_err());
}
