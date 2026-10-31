//! AIFF, through aifc, which reads and writes both AIFF and AIFF-C.
//!
//! Tags survive here: AIFF carries an `ID3 ` chunk, which is the same ID3v2
//! payload the MP3 path writes, so one serialiser covers both.

use std::fs::File;
use std::io::BufWriter;
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
    // Plain AIFF accepts only signed big-endian integers. Float samples would
    // force AIFF-C, which some older DJ software still will not open, so a
    // float request is refused rather than quietly written as a format the
    // user's gear may reject.
    let sample_format = match spec.bit_depth {
        16 => aifc::SampleFormat::I16,
        24 => aifc::SampleFormat::I24,
        other => {
            return Err(CapstoneError::Unsupported(format!(
                "aiff output at {} bits",
                other
            )))
        }
    };

    let info = aifc::AifcWriteInfo {
        file_format: aifc::FileFormat::Aiff,
        channels: audio.channel_count() as i16,
        sample_rate: f64::from(audio.sample_rate),
        sample_format,
    };

    let mut file = BufWriter::new(File::create(dest)?);
    let mut writer = aifc::AifcWriter::new(&mut file, &info)?;

    let interleaved = audio.to_interleaved();
    match spec.bit_depth {
        16 => writer.write_samples_i16(&dither::to_i16(&interleaved))?,
        _ => writer.write_samples_i24(&dither::to_i24(&interleaved))?,
    }

    // After the samples, before finalize: an ID3 chunk at the end of the file
    // is where tag editors expect it, because it can then be rewritten without
    // moving the audio.
    if let Some(id3) = tags::id3v2(source_tags) {
        writer.write_chunk_id3(&id3)?;
    }

    writer.finalize()?;
    Ok(())
}
