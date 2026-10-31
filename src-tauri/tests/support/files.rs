//! Scratch directories and writing a fixture out as a real file.
//!
//! Split from `fixtures` so neither file outgrows the module size the house
//! rules set, and because this is the only part of the support code that
//! touches the filesystem.

use std::path::{Path, PathBuf};

use capstone_lib::audio::DecodedAudio;

/// A scratch directory of this run's own, removed when it goes out of scope.
///
/// Both halves earn their keep. The uuid is what lets libtest run these on
/// parallel threads without one test's fixture landing on another's, and the
/// `Drop` is because the fixtures are real audio files: one pass of this suite
/// writes 880 MB, and a suite that leaves that behind on every run is a suite
/// that fills a CI worker's disk by the afternoon.
///
/// `Deref` to `Path` so a scratch directory is passed and joined exactly like
/// the `PathBuf` it replaced.
pub struct Scratch {
    path: PathBuf,
}

impl std::ops::Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Best effort. A directory that could not be removed is not worth
        // failing a test that has already made its point, and on a panic this
        // runs during the unwind, so a failing test does not leave its working
        // files behind either.
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub fn scratch(tag: &str) -> Scratch {
    let path =
        std::env::temp_dir()
            .join("capstone-e2e")
            .join(format!("{}-{}", tag, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&path).expect("scratch directory");
    Scratch { path }
}

/// Writes planar f32 out as a 32-bit float WAV, through `hound` rather than
/// through Capstone's own encoder: a fixture written by the code under test
/// would make a broken encoder look like a broken DSP stage.
pub fn write_wav(audio: &DecodedAudio, dest: &Path) {
    let spec = hound::WavSpec {
        channels: audio.channel_count(),
        sample_rate: audio.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(dest, spec).expect("wav writer");
    let frames = audio.frame_count();
    for frame in 0..frames {
        for channel in &audio.channels {
            writer.write_sample(channel[frame]).expect("sample");
        }
    }
    writer.finalize().expect("finalize");
}

/// Builds the buffer, writes it, and hands back both.
pub fn materialise(audio: DecodedAudio, dir: &Path, name: &str) -> (PathBuf, DecodedAudio) {
    let path = dir.join(name);
    write_wav(&audio, &path);
    (path, audio)
}
