//! Decoding, through symphonia.

use std::fs::File;
use std::path::Path;

use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, RawValue, StandardTag};

use crate::audio::DecodedAudio;
use crate::error::{CapstoneError, Result};

#[cfg(test)]
mod tests;

/// What `add_paths` accepts. Everything here is a format symphonia is built to
/// read in this crate's feature set, so a file that reaches the queue is a file
/// the pipeline can open.
pub const AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "wave", "aif", "aiff", "aifc", "flac", "mp3", "m4a", "mp4", "aac", "caf", "ogg", "oga",
    "opus", "alac",
];

pub fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.as_str()))
}

/// The fields a tag is reduced to on the way through.
///
/// Deliberately a short closed list rather than everything the source carried:
/// the five output formats agree on almost nothing, so a field earns a place
/// here only if more than one of them can write it. Artwork, ReplayGain and
/// MusicBrainz ids are dropped, which the `encode` module note records rather
/// than leaving to be discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagField {
    Title,
    Artist,
    Album,
    AlbumArtist,
    Genre,
    TrackNumber,
    Date,
    Comment,
    Bpm,
}

/// Tags lifted off the source file, in the order they were found.
#[derive(Debug, Clone, Default)]
pub struct SourceTags {
    entries: Vec<(TagField, String)>,
}

impl SourceTags {
    pub fn entries(&self) -> &[(TagField, String)] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// First writer wins, because revisions arrive oldest first and the oldest
    /// is the richest: a file carrying both an ID3v2 header and an ID3v1
    /// trailer has the whole title in the former and 30 truncated bytes of it
    /// in the latter.
    pub fn insert(&mut self, field: TagField, value: &str) {
        let value = value.trim();
        if value.is_empty() || self.entries.iter().any(|(f, _)| *f == field) {
            return;
        }
        self.entries.push((field, value.to_string()));
    }
}

/// Decodes a whole file into planar f32.
pub fn decode_to_f32(path: &Path) -> Result<DecodedAudio> {
    decode_with_tags(path).map(|(audio, _)| audio)
}

/// Decodes a whole file and reads its tags in the same pass.
///
/// One probe, one walk of the packets: decoding is by far the most expensive
/// thing Capstone does per file, so the tags a re-encode needs are collected
/// here rather than by opening the file again. They are read *after* the packet
/// loop on purpose, since a trailing ID3v1 block only reaches the metadata log
/// once the reader has walked past the audio.
///
/// Three things the implementation has to get right, each of which has already
/// cost someone a day:
///
/// 1. symphonia 0.6 is a breaking change from 0.5, which is what nearly every
///    example online targets. `get_probe().probe()` returns the format reader
///    directly, tracks come from `default_track(TrackType::Audio)`, decoders
///    from `make_audio_decoder()`, and `Packet::track_id` is a field.
/// 2. The FLAC reader signals the end of a stream as `Error::IoError` with
///    `ErrorKind::UnexpectedEof` rather than `Ok(None)`. Treat that as a clean
///    end alongside `Ok(None)` in the packet loop, or a perfectly good file is
///    reported as truncated.
/// 3. Encoder delay is not ours to trim, and trimming it is a bug. The MP3
///    reader reads the LAME/Xing tag and *applies* it: a file holding 89,856
///    samples, tagged delay 1,105 and padding 551, arrives here as exactly the
///    88,200 the source had. Trimming `Track::delay` and `Track::padding` on
///    top of that quietly removed another 37 ms from the head and tail of
///    every MP3. The MP4 reader is the opposite case and reports delay 0 while
///    applying neither the iTunSMPB atom nor the edit list, so an .m4a or
///    .adts source really does arrive with about 1,912 priming samples of
///    silence at the head, shifting every clip position and loudness block by
///    43 ms. There is nothing in the track fields to act on, so that one
///    cannot be fixed here.
pub fn decode_with_tags(path: &Path) -> Result<(DecodedAudio, SourceTags)> {
    let mut format = open(path)?;
    let (track_id, params) = audio_track(format.as_ref(), path)?;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())?;

    let (channels, sample_rate) = decode_packets(format.as_mut(), decoder.as_mut(), track_id)?;
    if channels.first().is_none_or(|c| c.is_empty()) || sample_rate == 0 {
        return Err(CapstoneError::Other(format!(
            "{} decoded to no audio",
            path.display()
        )));
    }
    let tags = read_tags(format.as_mut());
    Ok((DecodedAudio::new(channels, sample_rate), tags))
}

fn open(path: &Path) -> Result<Box<dyn FormatReader>> {
    let file = File::open(path)?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());

    // The extension is a hint, not a decision: symphonia probes the bytes
    // either way, so a .wav that is really an AIFF still opens. It only saves
    // the probe some guessing on containers whose magic is weak.
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(extension);
    }

    Ok(symphonia::default::get_probe().probe(
        &hint,
        stream,
        FormatOptions::default(),
        MetadataOptions::default(),
    )?)
}

type TrackInfo = (u32, symphonia::core::codecs::audio::AudioCodecParameters);

fn audio_track(format: &dyn FormatReader, path: &Path) -> Result<TrackInfo> {
    let track = format.default_track(TrackType::Audio).ok_or_else(|| {
        CapstoneError::Unsupported(format!("{} has no audio track", path.display()))
    })?;
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| {
            CapstoneError::Unsupported(format!("{} has no decodable audio", path.display()))
        })?;
    Ok((track.id, params.clone()))
}

/// Walks every packet of one track into planar f32.
///
/// A malformed packet is skipped rather than fatal: symphonia's own contract is
/// that `DecodeError` and `IoError` from `decode` mean "discard this packet and
/// carry on", and a bought library has files with a corrupt frame or two in the
/// middle that still play fine everywhere else. Refusing a whole file over one
/// bad frame is the wrong trade for a batch processor.
fn decode_packets(
    format: &mut dyn FormatReader,
    decoder: &mut dyn AudioDecoder,
    track_id: u32,
) -> Result<(Vec<Vec<f32>>, u32)> {
    let mut channels: Vec<Vec<f32>> = Vec::new();
    // Reused across packets. `copy_to_vecs_planar` resizes it to fit each
    // buffer, so one allocation serves the whole file.
    let mut planes: Vec<Vec<f32>> = Vec::new();
    let mut sample_rate = 0u32;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(e) => return Err(e.into()),
        };
        if packet.track_id != track_id {
            continue;
        }

        let buffer = match decoder.decode(&packet) {
            Ok(buffer) => buffer,
            Err(SymphoniaError::DecodeError(_) | SymphoniaError::IoError(_)) => continue,
            Err(e) => return Err(e.into()),
        };
        sample_rate = buffer.spec().rate();
        buffer.copy_to_vecs_planar::<f32>(&mut planes);

        if channels.is_empty() {
            channels.resize(planes.len(), Vec::new());
        } else if planes.len() != channels.len() {
            return Err(CapstoneError::Unsupported(
                "the channel layout changes part way through this file".to_string(),
            ));
        }
        for (channel, plane) in channels.iter_mut().zip(planes.iter()) {
            channel.extend_from_slice(plane);
        }
    }

    Ok((channels, sample_rate))
}

fn read_tags(format: &mut dyn FormatReader) -> SourceTags {
    let mut tags = SourceTags::default();
    let mut metadata = format.metadata();
    loop {
        if let Some(revision) = metadata.current() {
            collect_revision(revision, &mut tags);
        }
        // `pop` discards the revision just read and returns it, so this walks
        // oldest to newest and ends when there is nothing newer.
        if metadata.pop().is_none() {
            break;
        }
    }
    tags
}

fn collect_revision(revision: &MetadataRevision, tags: &mut SourceTags) {
    for tag in &revision.media.tags {
        let Some(field) = tag.std.as_ref().and_then(standard_field) else {
            continue;
        };
        // Artwork and bare flags have no text form worth putting in a title,
        // and `RawValue`'s own Display renders a binary blob as hex.
        if matches!(tag.raw.value, RawValue::Binary(_) | RawValue::Flag) {
            continue;
        }
        tags.insert(field, &tag.raw.value.to_string());
    }
}

/// symphonia's standard tag list is long and container-specific; this is the
/// slice of it the output formats can carry. Both date variants land on `Date`
/// because `SourceTags::insert` keeps the first, and a release date is the one a
/// DJ expects to read in a year column.
fn standard_field(std: &StandardTag) -> Option<TagField> {
    Some(match std {
        StandardTag::TrackTitle(_) => TagField::Title,
        StandardTag::Artist(_) => TagField::Artist,
        StandardTag::Album(_) => TagField::Album,
        StandardTag::AlbumArtist(_) => TagField::AlbumArtist,
        StandardTag::Genre(_) => TagField::Genre,
        StandardTag::TrackNumber(_) => TagField::TrackNumber,
        StandardTag::ReleaseDate(_) | StandardTag::RecordingDate(_) => TagField::Date,
        StandardTag::Comment(_) => TagField::Comment,
        StandardTag::Bpm(_) => TagField::Bpm,
        _ => return None,
    })
}

