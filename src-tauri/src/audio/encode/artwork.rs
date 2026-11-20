//! The picture that goes into the output file.
//!
//! A mastered track that comes back without its artwork looks, in a DJ
//! library, like a different record: the grid of covers is how a set gets
//! built, and a row of grey squares is the same problem as a missing title.
//! So the art rides through with the tags.
//!
//! Where it comes from, in order:
//!
//! 1. **An artwork folder**, matched on the filename — `Artwork/Vol. 2/Dom
//!    Dolla - Strangers.jpg` for `Songs/Vol. 2/Dom Dolla - Strangers.mp3`.
//!    This is the case worth supporting properly: downloaded tracks often
//!    carry no embedded art at all, and the covers were collected separately.
//! 2. **An image beside the source**, same stem.
//! 3. **The source's own embedded picture**, which is the common case and the
//!    one that must never be lost.
//!
//! Only JPEG and PNG, because those are the two formats every player and
//! every container agrees on, and they are the two this build can decode to
//! read a size out of.

use std::path::{Path, PathBuf};

use lofty::file::TaggedFileExt;
use lofty::picture::{MimeType, Picture, PictureType};

/// Extensions looked for beside a track, in preference order. JPEG first: it
/// is what covers actually are, and a `.png` sitting next to a track is more
/// often a screenshot.
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png"];

/// Pictures larger than this are passed over rather than embedded.
///
/// Not a performance guard — it is about what the file is for. A 30 MB
/// scanned sleeve embedded into every one of fifty masters is 1.5 GB of
/// artwork in a folder of audio, and no player draws it at that size.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artwork {
    /// `image/jpeg` or `image/png`.
    pub mime: String,
    pub data: Vec<u8>,
    /// Where it came from, for the report. Not used by any writer.
    pub source: ArtworkSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkSource {
    ArtworkFolder,
    BesideTheFile,
    Embedded,
}

impl ArtworkSource {
    pub fn as_str(self) -> &'static str {
        match self {
            ArtworkSource::ArtworkFolder => "artwork folder",
            ArtworkSource::BesideTheFile => "beside the file",
            ArtworkSource::Embedded => "the source file's own tags",
        }
    }
}

/// The artwork for one source file, if any can be found.
///
/// Never fails. Artwork is decoration: a folder that cannot be read, a JPEG
/// that is actually a text file, a tagger that wrote something strange —
/// none of those is a reason to fail a master.
pub fn find(source: &Path, artwork_dir: Option<&Path>) -> Option<Artwork> {
    let stem = source.file_stem()?;
    if let Some(dir) = artwork_dir {
        if let Some(found) = beside(dir, stem) {
            return read_image(&found, ArtworkSource::ArtworkFolder);
        }
    }
    if let Some(found) = source.parent().and_then(|dir| beside(dir, stem)) {
        return read_image(&found, ArtworkSource::BesideTheFile);
    }
    embedded(source)
}

/// `<dir>/<stem>.<ext>` for each extension, first hit wins.
///
/// The stem is matched exactly rather than fuzzily. A near-match is how one
/// track's cover ends up on another's, which is worse than no cover: the art
/// is what identifies the record at a glance.
fn beside(dir: &Path, stem: &std::ffi::OsStr) -> Option<PathBuf> {
    IMAGE_EXTENSIONS.iter().find_map(|ext| {
        // Appended rather than set: a stem like `Fred again..` loses its tail
        // to `set_extension`, which is the same bug the output namer had.
        let mut name = stem.to_os_string();
        name.push(".");
        name.push(ext);
        let candidate = dir.join(name);
        candidate.is_file().then_some(candidate)
    })
}

fn read_image(path: &Path, from: ArtworkSource) -> Option<Artwork> {
    let size = std::fs::metadata(path).ok()?.len();
    if size == 0 || size > MAX_BYTES {
        return None;
    }
    let data = std::fs::read(path).ok()?;
    Some(Artwork {
        mime: sniff(&data)?.to_string(),
        data,
        source: from,
    })
}

/// The source file's own front cover, in whatever container it is in.
fn embedded(source: &Path) -> Option<Artwork> {
    let tagged = lofty::read_from_path(source).ok()?;
    let pictures = || tagged.tags().iter().flat_map(|tag| tag.pictures());
    let picture: &Picture = pictures()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures().next())?;
    if picture.data().len() as u64 > MAX_BYTES {
        return None;
    }
    let mime = match picture.mime_type() {
        Some(MimeType::Jpeg) => "image/jpeg",
        Some(MimeType::Png) => "image/png",
        // Sniffed rather than trusted: plenty of taggers write `image/jpg`,
        // an empty mime, or none at all.
        _ => sniff(picture.data())?,
    };
    Some(Artwork {
        mime: mime.to_string(),
        data: picture.data().to_vec(),
        source: ArtworkSource::Embedded,
    })
}

/// The mime type from the bytes themselves.
///
/// The extension is a claim and the tag's mime field is a claim; the magic
/// number is what a player will actually act on. A `.jpg` that is really a
/// PNG embedded as `image/jpeg` draws nothing in some players and in others
/// draws fine, which is the worst kind of bug to chase.
fn sniff(data: &[u8]) -> Option<&'static str> {
    const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }
    if data.starts_with(PNG) {
        return Some("image/png");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jpeg_bytes() -> Vec<u8> {
        let mut data = vec![0xFF, 0xD8, 0xFF, 0xE0];
        data.extend_from_slice(&[0u8; 64]);
        data
    }

    fn png_bytes() -> Vec<u8> {
        let mut data = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        data.extend_from_slice(&[0u8; 64]);
        data
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("capstone-artwork-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    #[test]
    fn the_artwork_folder_is_preferred_over_an_image_beside_the_file() {
        let dir = scratch("preference");
        let songs = dir.join("Songs");
        let art = dir.join("Artwork");
        std::fs::create_dir_all(&songs).unwrap();
        std::fs::create_dir_all(&art).unwrap();
        let track = songs.join("Dom Dolla - Strangers.mp3");
        std::fs::write(&track, b"not audio").unwrap();
        std::fs::write(songs.join("Dom Dolla - Strangers.png"), png_bytes()).unwrap();
        std::fs::write(art.join("Dom Dolla - Strangers.jpg"), jpeg_bytes()).unwrap();

        let found = find(&track, Some(&art)).expect("artwork");
        assert_eq!(found.source, ArtworkSource::ArtworkFolder);
        assert_eq!(found.mime, "image/jpeg");
    }

    #[test]
    fn an_image_beside_the_file_is_found_without_an_artwork_folder() {
        let dir = scratch("beside");
        let track = dir.join("Track.wav");
        std::fs::write(&track, b"not audio").unwrap();
        std::fs::write(dir.join("Track.jpg"), jpeg_bytes()).unwrap();
        let found = find(&track, None).expect("artwork");
        assert_eq!(found.source, ArtworkSource::BesideTheFile);
    }

    /// The bug the output namer had, in a second place: a stem with a dot in
    /// it must keep all of it when the extension is appended.
    #[test]
    fn a_dotted_name_finds_its_own_artwork() {
        let dir = scratch("dotted");
        let track = dir.join("Fred again.. - Delilah.mp3");
        std::fs::write(&track, b"not audio").unwrap();
        std::fs::write(dir.join("Fred again.. - Delilah.jpg"), jpeg_bytes()).unwrap();
        assert!(find(&track, None).is_some());
    }

    #[test]
    fn a_near_match_is_not_used() {
        let dir = scratch("near");
        let track = dir.join("Track One.wav");
        std::fs::write(&track, b"not audio").unwrap();
        std::fs::write(dir.join("Track Two.jpg"), jpeg_bytes()).unwrap();
        assert!(find(&track, None).is_none());
    }

    #[test]
    fn a_file_that_is_not_an_image_is_passed_over() {
        let dir = scratch("notimage");
        let track = dir.join("Track.wav");
        std::fs::write(&track, b"not audio").unwrap();
        std::fs::write(dir.join("Track.jpg"), b"<html>nope</html>").unwrap();
        assert!(find(&track, None).is_none());
    }

    #[test]
    fn an_empty_image_file_is_passed_over() {
        let dir = scratch("empty");
        let track = dir.join("Track.wav");
        std::fs::write(&track, b"not audio").unwrap();
        std::fs::write(dir.join("Track.png"), b"").unwrap();
        assert!(find(&track, None).is_none());
    }

    #[test]
    fn the_mime_comes_from_the_bytes_not_the_extension() {
        let dir = scratch("mime");
        let track = dir.join("Track.wav");
        std::fs::write(&track, b"not audio").unwrap();
        // A PNG wearing a .jpg extension.
        std::fs::write(dir.join("Track.jpg"), png_bytes()).unwrap();
        assert_eq!(find(&track, None).expect("artwork").mime, "image/png");
    }

    #[test]
    fn sniffing_knows_the_two_formats_and_nothing_else() {
        assert_eq!(sniff(&jpeg_bytes()), Some("image/jpeg"));
        assert_eq!(sniff(&png_bytes()), Some("image/png"));
        assert_eq!(sniff(b"GIF89a"), None);
        assert_eq!(sniff(b""), None);
    }
}
