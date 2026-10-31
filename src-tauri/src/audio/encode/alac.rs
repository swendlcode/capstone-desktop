//! Apple Lossless: alac-encoder for the frames, our own CAF for the container.
//!
//! Two ceilings, both deliberate and both visible to the user rather than
//! discovered later:
//!
//! * **16-bit only.** `FormatDescription::alac()` hardcodes
//!   `bits_per_channel: 16` and its fields are private, so 24-bit would mean
//!   writing the ALAC magic cookie by hand. `spec.bit_depth` is ignored here.
//! * **No tags.** The CAF writer has no `info` chunk, so an ALAC export loses
//!   the source's tags. FLAC is the lossless format to choose if they matter.

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use alac_encoder::{AlacEncoder, FormatDescription};

use crate::audio::encode::caf;
use crate::audio::encode::dither;
use crate::audio::DecodedAudio;
use crate::error::{CapstoneError, Result};

/// One ALAC packet, in frames. 4096 is what Apple's own encoder uses and what
/// the packet table is sized against.
const FRAMES_PER_PACKET: u32 = 4096;

/// ALAC's own limit, and the limit on `FormatDescription`'s channel maps.
/// Checked here because those constructors assert rather than return, and a
/// batch processor must not die on one odd file.
const MAX_CHANNELS: u16 = 8;

pub fn write(audio: &DecodedAudio, dest: &Path) -> Result<()> {
    let channels = audio.channel_count();
    if channels == 0 || channels > MAX_CHANNELS {
        return Err(CapstoneError::Unsupported(format!(
            "apple lossless carries 1 to {} channels, this file has {}",
            MAX_CHANNELS, channels
        )));
    }

    let rate = f64::from(audio.sample_rate);
    let input_format = FormatDescription::pcm::<i16>(rate, u32::from(channels));
    let output_format = FormatDescription::alac(rate, FRAMES_PER_PACKET, u32::from(channels));

    let mut encoder = AlacEncoder::new(&output_format);
    let magic_cookie = encoder.magic_cookie();
    let mut scratch = vec![0u8; output_format.max_packet_size()];

    // Little-endian i16 bytes are what the PCM format description above
    // declares, so the encoder reads them directly with no conversion.
    let pcm: Vec<u8> = dither::to_i16(&audio.to_interleaved())
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    let bytes_per_packet = FRAMES_PER_PACKET as usize * channels as usize * 2;

    let mut packets = Vec::with_capacity(pcm.len() / bytes_per_packet + 1);
    for chunk in pcm.chunks(bytes_per_packet) {
        let written = encoder.encode(&input_format, chunk, &mut scratch);
        packets.push(scratch[..written].to_vec());
    }

    let mut file = BufWriter::new(File::create(dest)?);
    caf::write_alac_caf(
        &mut file,
        rate,
        u32::from(channels),
        FRAMES_PER_PACKET,
        audio.frame_count() as u64,
        &magic_cookie,
        &packets,
    )?;
    Ok(())
}
