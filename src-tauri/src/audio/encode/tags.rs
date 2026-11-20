//! Writing the source's tags back out.
//!
//! These are DJ libraries: a track whose artist and title vanished is worse
//! than a track that was never processed, because the owner has to find it
//! again by ear. So the tags ride through the pipeline and are written back in
//! whatever form the output format accepts.
//!
//! Three serialisations cover four of the five formats:
//!
//! * `id3v2` for MP3 (prepended to the stream) and AIFF (an `ID3 ` chunk).
//! * `vorbis_comment` for FLAC, as a metadata block.
//! * `riff_info_chunk` for WAV, as a `LIST`/`INFO` chunk.
//!
//! CAF gets nothing: the writer here has no `info` chunk, so an Apple Lossless
//! export loses its tags. That gap is a few lines if it turns out to matter.

use crate::audio::decode::{SourceTags, TagField};
use crate::audio::encode::artwork::Artwork;

/// Hand-rolled rather than taken from a crate: an ID3v2.3 text frame is a
/// four-byte id, a length and a string, and the dependency set is already
/// large enough that adding one for that is the wrong trade.
///
/// Version 2.3 with UTF-16 text, not 2.4 with UTF-8. 2.3 is what every DJ
/// program and every hardware player reads, and 2.3 permits exactly two text
/// encodings: Latin-1 and UTF-16 with a byte-order mark. UTF-8 in a 2.3 frame
/// is the common shortcut and is simply invalid, so the one that cannot lose a
/// character is UTF-16.
pub fn id3v2(tags: &SourceTags, art: Option<&Artwork>) -> Option<Vec<u8>> {
    if tags.is_empty() && art.is_none() {
        return None;
    }
    let mut frames = Vec::new();
    for (field, value) in tags.entries() {
        match field {
            TagField::Title => text_frame(&mut frames, b"TIT2", value),
            TagField::Artist => text_frame(&mut frames, b"TPE1", value),
            TagField::Album => text_frame(&mut frames, b"TALB", value),
            TagField::AlbumArtist => text_frame(&mut frames, b"TPE2", value),
            TagField::Genre => text_frame(&mut frames, b"TCON", value),
            TagField::TrackNumber => text_frame(&mut frames, b"TRCK", value),
            TagField::Bpm => text_frame(&mut frames, b"TBPM", value),
            // 2.3 has no combined date frame: TYER is a bare four-digit year,
            // and a source date of "2019-05-02" has to be cut down to one.
            TagField::Date => {
                if let Some(year) = year_of(value) {
                    text_frame(&mut frames, b"TYER", year);
                }
            }
            TagField::Comment => comment_frame(&mut frames, value),
        }
    }
    if let Some(art) = art {
        picture_frame(&mut frames, art);
    }
    if frames.is_empty() {
        return None;
    }

    let mut tag = Vec::with_capacity(frames.len() + 10);
    tag.extend_from_slice(b"ID3");
    tag.extend_from_slice(&[0x03, 0x00]); // version 2.3.0
    tag.push(0x00); // no unsynchronisation, no extended header
    tag.extend_from_slice(&syncsafe(frames.len() as u32));
    tag.extend_from_slice(&frames);
    Some(tag)
}

/// `APIC`: the front cover.
///
/// Latin-1 is named as the text encoding even though the rest of the tag is
/// UTF-16, and that is correct rather than inconsistent: the two strings in
/// this frame are a mime type and a description, the mime type is ASCII by
/// definition, and the description is written empty. Latin-1 terminates on a
/// single zero byte, so an empty description is one byte instead of the two
/// a UTF-16 terminator needs — and every reader agrees on how to parse it.
fn picture_frame(out: &mut Vec<u8>, art: &Artwork) {
    const FRONT_COVER: u8 = 0x03;
    let mut body = vec![0x00];
    body.extend_from_slice(art.mime.as_bytes());
    body.push(0x00);
    body.push(FRONT_COVER);
    body.push(0x00); // empty description
    body.extend_from_slice(&art.data);
    frame(out, b"APIC", &body);
}

/// A FLAC `METADATA_BLOCK_PICTURE` body.
///
/// Big-endian throughout, which is the opposite of the VORBIS_COMMENT block
/// sitting next to it in the same file. The four dimension fields are written
/// as zero: the format defines zero as "unknown", and decoding the image to
/// fill them in would be work for a number no player needs — it reads the
/// picture itself.
pub fn flac_picture(art: &Artwork) -> Vec<u8> {
    const FRONT_COVER: u32 = 3;
    let mut out = Vec::with_capacity(art.data.len() + 64);
    out.extend_from_slice(&FRONT_COVER.to_be_bytes());
    out.extend_from_slice(&(art.mime.len() as u32).to_be_bytes());
    out.extend_from_slice(art.mime.as_bytes());
    out.extend_from_slice(&0u32.to_be_bytes()); // empty description
    for _ in 0..4 {
        // width, height, colour depth, number of colours: all unknown.
        out.extend_from_slice(&0u32.to_be_bytes());
    }
    out.extend_from_slice(&(art.data.len() as u32).to_be_bytes());
    out.extend_from_slice(&art.data);
    out
}

fn text_frame(out: &mut Vec<u8>, id: &[u8; 4], value: &str) {
    let mut body = vec![0x01]; // UTF-16 with BOM
    body.extend_from_slice(&utf16_with_bom(value));
    frame(out, id, &body);
}

/// COMM is not a plain text frame: it carries a language and a short
/// description before the comment itself, and both terminators have to be there
/// or readers run the description into the text.
fn comment_frame(out: &mut Vec<u8>, value: &str) {
    let mut body = vec![0x01];
    body.extend_from_slice(b"eng");
    body.extend_from_slice(&[0xFF, 0xFE, 0x00, 0x00]); // empty description
    body.extend_from_slice(&utf16_with_bom(value));
    frame(out, b"COMM", &body);
}

fn frame(out: &mut Vec<u8>, id: &[u8; 4], body: &[u8]) {
    out.extend_from_slice(id);
    // Plain big-endian, not syncsafe: frame sizes only became syncsafe in 2.4,
    // and writing a syncsafe size here is how a 2.3 tag ends up unreadable.
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&[0x00, 0x00]); // frame flags
    out.extend_from_slice(body);
}

fn utf16_with_bom(value: &str) -> Vec<u8> {
    let mut out = vec![0xFF, 0xFE];
    for unit in value.encode_utf16() {
        out.extend_from_slice(&unit.to_le_bytes());
    }
    out.extend_from_slice(&[0x00, 0x00]); // string terminator
    out
}

/// The tag's own length, seven bits per byte. The high bit stays clear so the
/// length can never look like the start of an MPEG frame, which is the whole
/// point of the encoding.
fn syncsafe(size: u32) -> [u8; 4] {
    [
        ((size >> 21) & 0x7F) as u8,
        ((size >> 14) & 0x7F) as u8,
        ((size >> 7) & 0x7F) as u8,
        (size & 0x7F) as u8,
    ]
}

fn year_of(value: &str) -> Option<&str> {
    let year = value.get(..4)?;
    year.chars().all(|c| c.is_ascii_digit()).then_some(year)
}

/// A FLAC VORBIS_COMMENT block body: vendor string, then `KEY=value` entries.
/// Every length here is little-endian, which is the one surprise in an
/// otherwise big-endian container.
pub fn vorbis_comment(tags: &SourceTags) -> Option<Vec<u8>> {
    if tags.is_empty() {
        return None;
    }
    const VENDOR: &[u8] = b"Capstone";

    let mut out = Vec::new();
    out.extend_from_slice(&(VENDOR.len() as u32).to_le_bytes());
    out.extend_from_slice(VENDOR);

    let entries: Vec<String> = tags
        .entries()
        .iter()
        .map(|(field, value)| format!("{}={}", vorbis_key(*field), value))
        .collect();
    out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for entry in entries {
        out.extend_from_slice(&(entry.len() as u32).to_le_bytes());
        out.extend_from_slice(entry.as_bytes());
    }
    Some(out)
}

/// A RIFF `LIST`/`INFO` chunk, ready to splice into a WAV ahead of its audio.
///
/// Each entry is a four-byte id, a little-endian length, the text with a null
/// terminator, and a pad byte when that lands on an odd length. The lengths are
/// little-endian here and big-endian in AIFF, which is the same difference that
/// runs through the two containers.
pub fn riff_info_chunk(tags: &SourceTags) -> Option<Vec<u8>> {
    if tags.is_empty() {
        return None;
    }
    let mut body = b"INFO".to_vec();
    for (field, value) in tags.entries() {
        let Some(id) = riff_info_id(*field) else {
            continue;
        };
        let mut text = value.as_bytes().to_vec();
        text.push(0);
        body.extend_from_slice(id);
        body.extend_from_slice(&(text.len() as u32).to_le_bytes());
        body.extend_from_slice(&text);
        if text.len() % 2 == 1 {
            body.push(0);
        }
    }
    if body.len() == 4 {
        return None;
    }

    let mut chunk = b"LIST".to_vec();
    chunk.extend_from_slice(&(body.len() as u32).to_le_bytes());
    chunk.extend_from_slice(&body);
    Some(chunk)
}

/// RIFF INFO has no album-artist or BPM id. Inventing one would write a chunk
/// nothing reads, so those two fields are dropped for WAV rather than guessed
/// at.
fn riff_info_id(field: TagField) -> Option<&'static [u8; 4]> {
    Some(match field {
        TagField::Title => b"INAM",
        TagField::Artist => b"IART",
        TagField::Album => b"IPRD",
        TagField::Genre => b"IGNR",
        TagField::TrackNumber => b"ITRK",
        TagField::Date => b"ICRD",
        TagField::Comment => b"ICMT",
        TagField::AlbumArtist | TagField::Bpm => return None,
    })
}

fn vorbis_key(field: TagField) -> &'static str {
    match field {
        TagField::Title => "TITLE",
        TagField::Artist => "ARTIST",
        TagField::Album => "ALBUM",
        TagField::AlbumArtist => "ALBUMARTIST",
        TagField::Genre => "GENRE",
        TagField::TrackNumber => "TRACKNUMBER",
        TagField::Date => "DATE",
        TagField::Comment => "COMMENT",
        TagField::Bpm => "BPM",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tags_produce_nothing() {
        let tags = SourceTags::default();
        assert!(id3v2(&tags, None).is_none());
        assert!(vorbis_comment(&tags).is_none());
    }

    #[test]
    fn syncsafe_keeps_the_high_bit_clear() {
        assert_eq!(syncsafe(0), [0, 0, 0, 0]);
        assert_eq!(syncsafe(255), [0, 0, 1, 0x7F]);
        assert!(syncsafe(u32::MAX >> 4).iter().all(|b| *b < 0x80));
    }

    #[test]
    fn a_date_is_cut_down_to_a_year() {
        assert_eq!(year_of("2019-05-02"), Some("2019"));
        assert_eq!(year_of("2019"), Some("2019"));
        assert_eq!(year_of("unknown"), None);
        assert_eq!(year_of("19"), None);
    }

    #[test]
    fn id3_header_states_the_frame_length() {
        let mut body = Vec::new();
        text_frame(&mut body, b"TIT2", "Test");
        let mut tags = SourceTags::default();
        tags.insert(TagField::Title, "Test");
        let tag = id3v2(&tags, None).unwrap();
        assert_eq!(&tag[0..3], b"ID3");
        assert_eq!(tag.len(), 10 + body.len());
        assert_eq!(syncsafe(body.len() as u32), tag[6..10]);
    }

    fn art() -> Artwork {
        Artwork {
            mime: "image/jpeg".into(),
            data: vec![0xFF, 0xD8, 0xFF, 0x01, 0x02],
            source: crate::audio::encode::artwork::ArtworkSource::Embedded,
        }
    }

    /// Artwork alone is a reason to write a tag: a file can have a cover and
    /// no title.
    #[test]
    fn a_picture_with_no_text_still_produces_a_tag() {
        let tag = id3v2(&SourceTags::default(), Some(&art())).expect("a tag");
        assert_eq!(&tag[0..3], b"ID3");
        assert!(find_frame(&tag, b"APIC").is_some());
    }

    #[test]
    fn the_picture_frame_carries_the_mime_the_type_and_the_bytes() {
        let tag = id3v2(&SourceTags::default(), Some(&art())).expect("a tag");
        let body = find_frame(&tag, b"APIC").expect("an APIC frame");
        assert_eq!(body[0], 0x00, "latin-1 text encoding");
        let mime_end = body[1..].iter().position(|b| *b == 0).unwrap() + 1;
        assert_eq!(&body[1..mime_end], b"image/jpeg");
        assert_eq!(body[mime_end + 1], 0x03, "front cover");
        assert_eq!(body[mime_end + 2], 0x00, "empty description");
        assert_eq!(&body[mime_end + 3..], art().data.as_slice());
    }

    /// The lengths in a FLAC picture block are big-endian, unlike the
    /// comment block beside it, and a reader that disagrees sees garbage.
    #[test]
    fn the_flac_picture_block_is_big_endian_and_self_describing() {
        let block = flac_picture(&art());
        assert_eq!(&block[0..4], &3u32.to_be_bytes(), "front cover");
        assert_eq!(&block[4..8], &10u32.to_be_bytes(), "mime length");
        assert_eq!(&block[8..18], b"image/jpeg");
        assert_eq!(&block[18..22], &0u32.to_be_bytes(), "no description");
        // Four unknown dimensions, then the data length and the data.
        let data_len_at = 22 + 16;
        assert_eq!(
            &block[data_len_at..data_len_at + 4],
            &(art().data.len() as u32).to_be_bytes()
        );
        assert_eq!(&block[data_len_at + 4..], art().data.as_slice());
    }

    /// Walks the frames to find one by id, the way a reader does.
    fn find_frame<'a>(tag: &'a [u8], id: &[u8; 4]) -> Option<&'a [u8]> {
        let mut at = 10;
        while at + 10 <= tag.len() {
            let size = u32::from_be_bytes([tag[at + 4], tag[at + 5], tag[at + 6], tag[at + 7]])
                as usize;
            let body = tag.get(at + 10..at + 10 + size)?;
            if &tag[at..at + 4] == id {
                return Some(body);
            }
            at += 10 + size;
        }
        None
    }
}
