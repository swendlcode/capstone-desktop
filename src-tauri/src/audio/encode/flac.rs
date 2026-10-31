//! FLAC, through flacenc, plus three corrections of its own behaviour.
//!
//! The first is `fix_flac_streaminfo`, without which Capstone cannot reopen
//! its own exports. The second is the VORBIS_COMMENT block: flacenc writes
//! none, so the tags are assembled here and added to the stream before it is
//! serialised. The third is `use_fixed`, below, which is the difference
//! between a 596 kB file and a 370 MB one.

use std::path::Path;

use flacenc::component::{BitRepr, MetadataBlockData};
use flacenc::error::Verify;

use crate::audio::decode::SourceTags;
use crate::audio::encode::{dither, fix_flac_streaminfo, tags, EncodeSpec};
use crate::audio::DecodedAudio;
use crate::error::{CapstoneError, Result};

/// The FLAC metadata block type for a Vorbis comment. flacenc exposes other
/// block types only as `Unknown`, which is all this needs: the body is ours to
/// build either way.
const VORBIS_COMMENT_BLOCK: u8 = 4;

pub fn write(
    audio: &DecodedAudio,
    spec: &EncodeSpec,
    dest: &Path,
    source_tags: &SourceTags,
) -> Result<()> {
    let interleaved = audio.to_interleaved();
    let samples: Vec<i32> = match spec.bit_depth {
        16 => dither::to_i16(&interleaved)
            .into_iter()
            .map(i32::from)
            .collect(),
        24 => dither::to_i24(&interleaved),
        other => {
            return Err(CapstoneError::Unsupported(format!(
                "flac output at {} bits",
                other
            )))
        }
    };

    let mut config = flacenc::config::Encoder::default();
    // The fixed predictor is switched off, and this is not a tuning
    // preference. flacenc estimates the cost of each Rice parameter in a u32
    // table saturated at 2^28 - 1, and when two parameters both saturate it
    // breaks the tie toward the *smaller* one, which is the most expensive
    // code there is rather than the cheapest. Saturation needs a mean residual
    // over about 2^17, which a fixed order-1 predictor on loud 24-bit audio
    // reaches easily and an LPC predictor does not. The result is a frame
    // coded at thousands of bits per sample: one 4,096-sample mono frame came
    // out at 369 MB inside an otherwise valid file, and symphonia skipped it,
    // so the file reopened 4,096 frames short with no error anywhere.
    //
    // Measured on five seconds of programme material at 24 bits, as a multiple
    // of the raw PCM size:
    //
    // ```text
    //                 use_fixed: true   false
    //   mono                   559.26x  0.90x
    //   stereo                   0.87x  0.76x
    //   loud stereo master       4.54x  0.93x
    // ```
    //
    // So it is not a trade either: the fixed predictor is never actually
    // winning here, it is only winning the comparison. At 16 bits residuals
    // cannot reach the saturation point and the two settings agree to within
    // 1.5%, which is why this went unnoticed while only 16-bit output was
    // being looked at.
    config.subframe_coding.use_fixed = false;
    let config = config
        .into_verified()
        .map_err(|e| CapstoneError::Encode(format!("flac config: {:?}", e)))?;
    let source = flacenc::source::MemSource::from_samples(
        &samples,
        audio.channels.len(),
        spec.bit_depth as usize,
        audio.sample_rate as usize,
    );

    let mut stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| CapstoneError::Encode(format!("flac encode: {:?}", e)))?;

    if let Some(comment) = tags::vorbis_comment(source_tags) {
        let block = MetadataBlockData::new_unknown(VORBIS_COMMENT_BLOCK, &comment)
            .map_err(|e| CapstoneError::Encode(format!("flac tags: {:?}", e)))?;
        stream.add_metadata_block(block);
    }

    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| CapstoneError::Encode(format!("flac write: {:?}", e)))?;

    let mut bytes = sink.as_slice().to_vec();
    reject_if_bloated(bytes.len(), audio, spec.bit_depth)?;
    fix_flac_streaminfo(&mut bytes)?;
    std::fs::write(dest, &bytes)?;
    Ok(())
}

/// Refuses a stream larger than the samples it encodes.
///
/// FLAC is lossless but not unboundedly so: every subframe can fall back to
/// VERBATIM, which stores the samples as they are, so a correctly encoded
/// stream can never need materially more room than the raw PCM. One that does
/// means the encoder has gone wrong, and the failure mode it has gone wrong in
/// is not subtle - see the `use_fixed` note above.
///
/// This is here rather than left to the `use_fixed` setting because that
/// setting avoids one known trigger of one known bug. The bound holds whatever
/// the cause, and the alternative to checking it is writing a file hundreds of
/// times too large and telling the user it worked.
fn reject_if_bloated(written: usize, audio: &DecodedAudio, bit_depth: u16) -> Result<()> {
    if would_reject(written, audio, bit_depth) {
        return Err(CapstoneError::Encode(format!(
            "flac encoder produced {written} bytes for {} bytes of audio, \
             which cannot be right; refusing to write it",
            raw_bytes(audio, bit_depth)
        )));
    }
    Ok(())
}

/// The bound on its own, so a test can exercise it without having to find
/// content that still trips the underlying encoder fault.
pub(super) fn would_reject(written: usize, audio: &DecodedAudio, bit_depth: u16) -> bool {
    let raw = raw_bytes(audio, bit_depth);
    // An eighth for VERBATIM's own frame headers and the extra bit per sample a
    // verbatim subframe carries, plus 64 kB for the metadata blocks, which
    // dominate on a very short file.
    written > raw + raw / 8 + 64 * 1024
}

fn raw_bytes(audio: &DecodedAudio, bit_depth: u16) -> usize {
    audio.frame_count() * audio.channels.len() * (bit_depth as usize / 8).max(1)
}

#[cfg(test)]
#[path = "flac_tests.rs"]
mod tests;
