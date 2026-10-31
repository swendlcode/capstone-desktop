//! WAV, through hound.
//!
//! One ceiling to know about: hound has no RF64 or BWF support, so a file over
//! 4 GB is not expressible. For single tracks that will not bite.
//!
//! WAV's tags are a `LIST`/`INFO` chunk, and hound writes no chunk but `fmt `
//! and `data`. The chunk is therefore built here and spliced in, which is also
//! why the tagged path renders to memory first: the chunk has to land *before*
//! `data`, because every reader stops parsing chunks the moment it reaches the
//! audio. Appending after `data` is legal RIFF that half the world ignores,
//! and it would mean a WAV written by Capstone came back untagged the next time
//! Capstone read it.

use std::io::{Cursor, Seek, Write};
use std::path::Path;

use crate::audio::decode::SourceTags;
use crate::audio::encode::{dither, tags, EncodeSpec};
use crate::audio::DecodedAudio;
use crate::error::{CapstoneError, Result};

pub fn write(
    audio: &DecodedAudio,
    spec: &EncodeSpec,
    dest: &Path,
    source_tags: &SourceTags,
) -> Result<()> {
    let format = match spec.bit_depth {
        16 | 24 => hound::SampleFormat::Int,
        32 => hound::SampleFormat::Float,
        other => {
            return Err(CapstoneError::Unsupported(format!(
                "wav output at {} bits",
                other
            )))
        }
    };

    // The buffer is the authority on rate and channel count, not the spec: a
    // header that disagreed with the samples would play back at the wrong
    // speed or with the channels crossed, which is worse than refusing.
    let wav_spec = hound::WavSpec {
        channels: audio.channel_count(),
        sample_rate: audio.sample_rate,
        bits_per_sample: spec.bit_depth,
        sample_format: format,
    };

    match tags::riff_info_chunk(source_tags) {
        // Nothing to splice, so the samples stream straight to disk and the
        // file is never held in memory.
        None => {
            let mut writer = hound::WavWriter::create(dest, wav_spec)?;
            write_samples(&mut writer, audio, spec.bit_depth)?;
            writer.finalize()?;
        }
        Some(chunk) => {
            let mut buffer = Cursor::new(Vec::new());
            {
                let mut writer = hound::WavWriter::new(&mut buffer, wav_spec)?;
                write_samples(&mut writer, audio, spec.bit_depth)?;
                writer.finalize()?;
            }
            let bytes = splice_before_data(buffer.into_inner(), &chunk)?;
            std::fs::write(dest, bytes)?;
        }
    }
    Ok(())
}

fn write_samples<W: Write + Seek>(
    writer: &mut hound::WavWriter<W>,
    audio: &DecodedAudio,
    bit_depth: u16,
) -> Result<()> {
    let interleaved = audio.to_interleaved();
    match bit_depth {
        16 => {
            for sample in dither::to_i16(&interleaved) {
                writer.write_sample(sample)?;
            }
        }
        24 => {
            for sample in dither::to_i24(&interleaved) {
                writer.write_sample(sample)?;
            }
        }
        // Float output is the one path that must be bit-transparent: it is what
        // the null test writes, so nothing may be scaled, rounded or dithered.
        _ => {
            for sample in interleaved {
                writer.write_sample(sample)?;
            }
        }
    }
    Ok(())
}

/// Inserts a chunk ahead of the `data` chunk and restores the RIFF size.
fn splice_before_data(wav: Vec<u8>, chunk: &[u8]) -> Result<Vec<u8>> {
    let at = data_chunk_offset(&wav).ok_or_else(|| {
        CapstoneError::Encode("wav output has no data chunk to splice before".to_string())
    })?;

    let mut out = Vec::with_capacity(wav.len() + chunk.len());
    out.extend_from_slice(&wav[..at]);
    out.extend_from_slice(chunk);
    out.extend_from_slice(&wav[at..]);

    let size = u32::try_from(out.len() - 8)
        .map_err(|_| CapstoneError::Encode("wav is too large for a RIFF header".to_string()))?;
    out[4..8].copy_from_slice(&size.to_le_bytes());
    Ok(out)
}

/// Walks the chunk list for `data`. Every RIFF chunk is a four-byte id, a
/// little-endian length and a body padded to an even length, so the walk is
/// exact rather than a search for the bytes "data", which could match audio.
fn data_chunk_offset(wav: &[u8]) -> Option<usize> {
    let mut at = 12; // past "RIFF", the file size and "WAVE"
    while at + 8 <= wav.len() {
        if &wav[at..at + 4] == b"data" {
            return Some(at);
        }
        let size = u32::from_le_bytes(wav[at + 4..at + 8].try_into().ok()?) as usize;
        at += 8 + size + (size & 1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::decode::TagField;
    use crate::models::Mp3Rate;

    fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }

    fn tagged_wav() -> Vec<u8> {
        let mut tags = SourceTags::default();
        tags.insert(TagField::Title, "Tone Test");
        tags.insert(TagField::Artist, "Capstone");

        let frames = vec![0.25f32; 1_001]; // odd, so the data chunk needs a pad
        let audio = DecodedAudio::new(vec![frames.clone(), frames], 44_100);
        let spec = EncodeSpec {
            sample_rate: 44_100,
            channels: 2,
            bit_depth: 16,
            mp3_rate: Mp3Rate::Cbr320,
        };
        // A fresh path per call, not one per process: two tests in this module
        // call this helper and libtest runs them on parallel threads, so a path
        // keyed only on the pid had one of them deleting the file the other was
        // still reading.
        let path = std::env::temp_dir().join(format!("capstone-wav-{}.wav", uuid::Uuid::new_v4()));
        write(&audio, &spec, &path, &tags).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        bytes
    }

    /// The tags cannot be checked by decoding the file: symphonia 0.6.1 parses
    /// the INFO chunk and then discards it (`WavReader::try_new` builds a
    /// metadata log from it, then constructs the reader with
    /// `opts.external_data.metadata` instead). So this reads the bytes. ffprobe
    /// and CoreAudio both read the tags back, which is what the DJ software
    /// downstream does too.
    #[test]
    fn the_info_chunk_lands_ahead_of_the_audio() {
        let bytes = tagged_wav();
        let list = find(&bytes, b"LIST").expect("no LIST chunk");
        let data = find(&bytes, b"data").expect("no data chunk");
        assert!(list < data, "the INFO chunk is behind the audio");
        assert!(find(&bytes, b"INAM").is_some_and(|at| at < data));
        assert!(find(&bytes, b"Tone Test\0".as_slice()).is_some());
    }

    /// A RIFF length that no longer matches the file is how a spliced chunk
    /// turns a valid WAV into one half the world refuses to open.
    #[test]
    fn the_riff_length_is_corrected_after_splicing() {
        let bytes = tagged_wav();
        let stated = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        assert_eq!(stated, bytes.len() - 8);
    }

    #[test]
    fn the_chunk_walk_finds_data_rather_than_matching_audio() {
        // "data" appearing inside the samples must not be mistaken for the
        // chunk header, which is why the walk steps by stated lengths.
        let mut wav = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        wav.extend_from_slice(&4u32.to_le_bytes());
        wav.extend_from_slice(b"data"); // the fmt chunk's body, not a header
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&2u32.to_le_bytes());
        wav.extend_from_slice(&[0, 0]);
        assert_eq!(data_chunk_offset(&wav), Some(24));
    }

    #[test]
    fn a_wav_with_no_data_chunk_is_refused() {
        let wav = b"RIFF\0\0\0\0WAVE".to_vec();
        assert!(splice_before_data(wav, b"LIST\0\0\0\0").is_err());
    }
}
