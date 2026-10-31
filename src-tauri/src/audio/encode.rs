//! Encoding: one module per output format, behind one dispatch.
//!
//! Each format is its own crate, because no single permissive crate covers the
//! five. The licence constraint is the reason: every mature MP3 encoder binding
//! is LGPL, and nothing permissive muxes ALAC into a container at all, which is
//! why `caf` below is code this project owns.
//!
//! Two things happen here that no single encoder does:
//!
//! * **Rate conversion.** `spec.sample_rate` is the rate the file must land at;
//!   when the buffer is at another rate it goes through `resample` first. MP3
//!   also has only nine legal rates, so a 96 kHz master cannot be handed to it
//!   untouched.
//! * **Tags.** `encode_with_tags` carries the source's tags through. These are
//!   DJ libraries: a file that comes back without its artist and title has to
//!   be identified by ear, which is worse than not having been processed. MP3,
//!   WAV, AIFF, FLAC and MP3 all keep them, each in its own form. ALAC does
//!   not, for the reason in that module.

pub mod aiff;
pub mod alac;
pub mod caf;
pub mod dither;
pub mod flac;
#[cfg(feature = "mp3")]
pub mod mp3;
pub mod resample;
pub mod tags;
pub mod wav;

#[cfg(test)]
mod tests;

use std::path::Path;

use crate::audio::decode::SourceTags;
use crate::audio::DecodedAudio;
use crate::error::{CapstoneError, Result};
use crate::models::{Mp3Rate, OutputFormat};

/// How the samples should land on disk. `mp3_rate` rides along rather than
/// sitting in a parallel argument so the dispatch signature stays the same for
/// all five formats.
///
/// `sample_rate` is a target: the audio is resampled to it when it differs.
/// `channels` is informational — the buffer being written is the authority on
/// its own channel count, and a header that disagreed with the samples would
/// play back garbled rather than fail.
#[derive(Debug, Clone, Copy)]
pub struct EncodeSpec {
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_depth: u16,
    pub mp3_rate: Mp3Rate,
}

pub fn extension(format: OutputFormat) -> &'static str {
    match format {
        OutputFormat::Aiff => "aiff",
        // CAF, not .m4a: nothing permissive writes an MP4 container, and
        // CoreAudio reads CAF natively. See the module note on `caf`.
        OutputFormat::Alac => "caf",
        OutputFormat::Flac => "flac",
        OutputFormat::Mp3 => "mp3",
        OutputFormat::Wav => "wav",
    }
}

/// Writes the buffer out, discarding whatever tags the source had.
///
/// Prefer `encode_with_tags` wherever the source's tags are at hand. This one
/// exists so a caller with no tags does not have to invent an empty set, not as
/// the normal path.
pub fn encode(
    audio: &DecodedAudio,
    spec: &EncodeSpec,
    format: OutputFormat,
    dest: &Path,
) -> Result<()> {
    encode_with_tags(audio, spec, format, dest, &SourceTags::default())
}

pub fn encode_with_tags(
    audio: &DecodedAudio,
    spec: &EncodeSpec,
    format: OutputFormat,
    dest: &Path,
    source_tags: &SourceTags,
) -> Result<()> {
    if audio.is_empty() {
        return Err(CapstoneError::Encode(
            "there is no audio to write".to_string(),
        ));
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Resampling is skipped rather than turned into a no-op clone: a
    // six-minute stereo track is about 60 MB of f32 per worker, and the common
    // case is that the rate already matches.
    let target = target_rate(audio, spec, format);
    let converted = if target == audio.sample_rate {
        None
    } else {
        Some(resample::to_rate(audio, target)?)
    };
    let audio = converted.as_ref().unwrap_or(audio);

    match format {
        OutputFormat::Wav => wav::write(audio, spec, dest, source_tags),
        OutputFormat::Aiff => aiff::write(audio, spec, dest, source_tags),
        OutputFormat::Flac => flac::write(audio, spec, dest, source_tags),
        OutputFormat::Alac => alac::write(audio, dest),
        #[cfg(feature = "mp3")]
        OutputFormat::Mp3 => mp3::write(audio, spec, dest, source_tags),
        // rusty_mp3 is the only permissively licensed MP3 encoder and it is
        // very new, so MP3 sits behind a Cargo feature. The refusal lives here
        // rather than at the UI, so it cannot be bypassed by a settings file
        // that still names mp3.
        #[cfg(not(feature = "mp3"))]
        OutputFormat::Mp3 => Err(CapstoneError::Unsupported(
            "this build was made without MP3 support".to_string(),
        )),
    }
}

/// The rate the file has to be at by the time an encoder sees it.
///
/// A zero in the spec means "whatever the audio already is", which keeps a
/// caller that does not care about rate conversion from having to restate the
/// buffer's own rate.
fn target_rate(audio: &DecodedAudio, spec: &EncodeSpec, format: OutputFormat) -> u32 {
    let requested = if spec.sample_rate == 0 {
        audio.sample_rate
    } else {
        spec.sample_rate
    };
    match format {
        #[cfg(feature = "mp3")]
        OutputFormat::Mp3 => mp3::supported_rate(requested),
        _ => requested,
    }
}

/// Five fields of STREAMINFO, after every FLAC encode. Two separate faults in
/// flacenc's header, both of which make symphonia - and so Capstone itself, on
/// the next run - misread a file Capstone has just written.
///
/// **The block size.** flacenc writes the short final frame's length into
/// `min_blocksize`. Since `min != max` the stream is then formally
/// variable-blocksize while its frame headers still carry fixed-strategy frame
/// numbers, so symphonia reads it as variable and runs off the end: Capstone
/// could not reopen its own FLAC exports for any file whose length is not a
/// whole multiple of the block size, which is nearly every real file. The
/// reference encoder keeps min == max.
///
/// **The frame sizes.** `min_framesize` and `max_framesize` come out wrong,
/// and not wrong by a consistent margin either way. A 4,096-sample mono frame
/// of 24-bit audio cannot exceed about 12.3 kB, yet the header has carried
/// 64,599; on another file it carried a maximum of 7,647 against a minimum of
/// 10,639, which symphonia refuses outright with "maximum frame length is less
/// than the minimum frame length". On a third it was merely inconsistent
/// enough to cost a whole 4,096-sample block on readback - a file that opened,
/// played, and was silently 93 ms short.
///
/// Both fields are optional in the format: zero means "unknown", and a reader
/// told nothing cannot be misled. Recomputing them would mean walking every
/// frame of the serialised stream for no benefit, since nothing in Capstone
/// reads them.
pub fn fix_flac_streaminfo(bytes: &mut [u8]) -> Result<()> {
    // "fLaC" magic, then a 4-byte metadata block header, then STREAMINFO:
    // min_blocksize at byte 8 and max_blocksize at byte 10, both big-endian
    // u16, then min_framesize at 12 and max_framesize at 15, both 24-bit.
    if bytes.len() < 18 || &bytes[0..4] != b"fLaC" {
        return Err(CapstoneError::Encode(
            "flac output is not a FLAC stream".to_string(),
        ));
    }
    let max_block = u16::from_be_bytes([bytes[10], bytes[11]]);
    bytes[8..10].copy_from_slice(&max_block.to_be_bytes());
    bytes[12..18].fill(0);
    Ok(())
}
