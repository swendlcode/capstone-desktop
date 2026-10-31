//! MP3, through rusty_mp3.
//!
//! Every mature MP3 encoder binding is copyleft (mp3lame-encoder and
//! mp3lame-sys are LGPL-3.0, shine-rs is LGPL-2.0), so rusty_mp3 is the only
//! permissive option and the whole format therefore sits behind a Cargo
//! feature. It is also young: validate it on real programme material before
//! trusting a library to it, and if it disappoints, this module is the only
//! thing that has to change.

use std::path::Path;

use rusty_mp3::{Mp3Encoder, Mp3EncoderConfig};

use crate::audio::decode::SourceTags;
use crate::audio::encode::{tags, EncodeSpec};
use crate::audio::DecodedAudio;
use crate::error::{CapstoneError, Result};
use crate::models::Mp3Rate;

/// The nine MPEG Layer III sample rates, ascending. Anything else has to be
/// resampled before it reaches the encoder.
const MPEG_RATES: [u32; 9] = [
    8_000, 11_025, 12_000, 16_000, 22_050, 24_000, 32_000, 44_100, 48_000,
];

/// LAME's `-q:a 2`, which lands around 190 kbps on music. Chosen over a lower
/// number because the input here is already a finished master: the point of the
/// VBR option is a smaller file than CBR 320, not the smallest possible file.
const VBR_QUALITY: f32 = 2.0;

/// The rate this file must be at before encoding.
///
/// The highest MPEG rate that does not exceed the source, so the content is
/// never stretched upward and never loses more band than it has to: a 96 kHz
/// master goes to 48 kHz, an 88.2 kHz master goes to 48 kHz too, and anything
/// below 8 kHz is brought up to the lowest rate the format defines.
pub fn supported_rate(rate: u32) -> u32 {
    MPEG_RATES
        .iter()
        .rev()
        .copied()
        .find(|supported| *supported <= rate)
        .unwrap_or(MPEG_RATES[0])
}

pub fn write(
    audio: &DecodedAudio,
    spec: &EncodeSpec,
    dest: &Path,
    source_tags: &SourceTags,
) -> Result<()> {
    let channels = audio.channel_count();
    // rusty_mp3 would take the first two channels and drop the rest without
    // saying so. Losing four channels of a surround file silently is worse
    // than refusing it.
    if channels == 0 || channels > 2 {
        return Err(CapstoneError::Unsupported(format!(
            "mp3 carries 1 or 2 channels, this file has {}",
            channels
        )));
    }

    let config = match spec.mp3_rate {
        Mp3Rate::Cbr320 => Mp3EncoderConfig {
            bitrate_kbps: 320,
            vbr_quality: None,
        },
        Mp3Rate::Vbr => Mp3EncoderConfig {
            bitrate_kbps: 0,
            vbr_quality: Some(rusty_mp3::vbr_quality_index(VBR_QUALITY)),
        },
    };

    let mut encoder = Mp3Encoder::new(config);
    encoder
        .push_pcm_f32(&audio.to_interleaved(), channels, audio.sample_rate)
        .map_err(|e| CapstoneError::Encode(format!("mp3 encode: {}", e)))?;
    encoder.finish();

    // An ID3v2 tag belongs in front of the first frame, where every decoder
    // skips it by name rather than having to resynchronise past it.
    let mut bytes = tags::id3v2(source_tags).unwrap_or_default();
    // `next_packet` reports Again or Eof rather than failing; after `finish`
    // the only way out of this loop is Eof, which is the drain finishing.
    while let Ok(packet) = encoder.next_packet() {
        bytes.extend_from_slice(&packet);
    }

    std::fs::write(dest, &bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rates_snap_downward_to_a_legal_one() {
        assert_eq!(supported_rate(44_100), 44_100);
        assert_eq!(supported_rate(48_000), 48_000);
        assert_eq!(supported_rate(96_000), 48_000);
        assert_eq!(supported_rate(88_200), 48_000);
        assert_eq!(supported_rate(192_000), 48_000);
        assert_eq!(supported_rate(4_000), 8_000);
    }
}
