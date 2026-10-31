//! A minimal CAF writer: the container ALAC needs.
//!
//! No permissively licensed crate muxes ALAC frames into `.m4a` or `.caf`, so
//! this is ours to own and test. CAF was the choice because it is far simpler
//! than MP4: a `desc` chunk, the encoder's magic cookie in `kuki`, a `pakt`
//! packet table and the data. CoreAudio and FFmpeg both accept the result and
//! the round trip is bit-identical.
//!
//! Everything in a CAF file is big-endian.

use std::io::{Result, Write};

pub fn write_alac_caf<W: Write>(
    w: &mut W,
    sample_rate: f64,
    channels: u32,
    frames_per_packet: u32,
    total_frames: u64,
    magic_cookie: &[u8],
    packets: &[Vec<u8>],
) -> Result<()> {
    w.write_all(b"caff")?;
    w.write_all(&1u16.to_be_bytes())?; // file version
    w.write_all(&0u16.to_be_bytes())?; // file flags

    chunk(
        w,
        b"desc",
        &description(sample_rate, channels, frames_per_packet),
    )?;
    chunk(w, b"kuki", magic_cookie)?;
    chunk(w, b"pakt", &packet_table(total_frames, packets))?;
    chunk(w, b"data", &data(packets))
}

fn chunk<W: Write>(w: &mut W, four_cc: &[u8; 4], body: &[u8]) -> Result<()> {
    w.write_all(four_cc)?;
    w.write_all(&(body.len() as i64).to_be_bytes())?;
    w.write_all(body)
}

fn description(sample_rate: f64, channels: u32, frames_per_packet: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(32);
    out.extend_from_slice(&sample_rate.to_be_bytes());
    out.extend_from_slice(b"alac");
    out.extend_from_slice(&1u32.to_be_bytes()); // format flags: 1 = 16-bit source
    out.extend_from_slice(&0u32.to_be_bytes()); // bytes per packet: 0 = variable
    out.extend_from_slice(&frames_per_packet.to_be_bytes());
    out.extend_from_slice(&channels.to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes()); // bits per channel: 0 for compressed
    out
}

/// The header is exactly four fields, in this order. An extra leading field
/// made both CoreAudio and FFmpeg reject the file outright, with no useful
/// diagnostic from either, so do not add one.
fn packet_table(total_frames: u64, packets: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(24 + packets.len() * 2);
    out.extend_from_slice(&(packets.len() as i64).to_be_bytes());
    out.extend_from_slice(&(total_frames as i64).to_be_bytes());
    out.extend_from_slice(&0u32.to_be_bytes()); // priming frames
    out.extend_from_slice(&0u32.to_be_bytes()); // remainder frames
    for packet in packets {
        push_varint(&mut out, packet.len() as u64);
    }
    out
}

/// Base-128, most significant group first, high bit set on every byte but the
/// last. This is how the packet table states each packet's byte length.
fn push_varint(out: &mut Vec<u8>, mut n: u64) {
    let mut buf = [0u8; 10];
    let mut i = buf.len() - 1;
    buf[i] = (n & 0x7f) as u8;
    n >>= 7;
    while n > 0 {
        i -= 1;
        buf[i] = ((n & 0x7f) as u8) | 0x80;
        n >>= 7;
    }
    out.extend_from_slice(&buf[i..]);
}

fn data(packets: &[Vec<u8>]) -> Vec<u8> {
    let total: usize = packets.iter().map(|p| p.len()).sum();
    let mut out = Vec::with_capacity(total + 4);
    out.extend_from_slice(&0u32.to_be_bytes()); // edit count
    for packet in packets {
        out.extend_from_slice(packet);
    }
    out
}
