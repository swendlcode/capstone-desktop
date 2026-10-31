//! Embedded cover art, as a small `data:` URI the table can put straight in an
//! `<img src>`.
//!
//! Two things shape this module, and both are about size.
//!
//! A full-size cover is commonly 1400x1400 and a megabyte or two. Fifty of
//! those, base64'd (which adds a third again) and held in React state, is
//! hundreds of megabytes of renderer heap for artwork drawn into a 28px square.
//! So nothing full-size ever leaves this file: the picture is decoded, scaled
//! to `THUMB_PX` on its long edge and re-encoded before it is encoded to text.
//!
//! And the scaling is the expensive part, so the result is memoised per path.
//! Scrolling the queue re-renders the same rows over and over; without the
//! cache each pass would reopen the file and redecode the JPEG.

use std::collections::HashMap;
use std::path::Path;

use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use image::imageops::FilterType;
use lofty::file::TaggedFileExt;
use lofty::picture::{Picture, PictureType};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

/// Long edge of the thumbnail, in pixels. The table draws it small; this is
/// roughly 2x that, so it still looks right on a retina panel.
const THUMB_PX: u32 = 64;

/// Path -> data URI. `None` is cached as well as `Some`: most files in a real
/// library carry no art at all, and re-probing every one of them on every
/// scroll is the same wasted work as re-decoding.
///
/// Unbounded on purpose. An entry is a few kilobytes at this size and the keys
/// are bounded by the paths the user added, so the ceiling is the queue. Keyed
/// on path alone, so a file re-tagged underneath us keeps the old thumbnail
/// until the app restarts -- an acceptable trade for not stat'ing on every row.
static CACHE: Lazy<Mutex<HashMap<String, Option<String>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// The thumbnail for `path`, or `None` when the file has no embedded picture,
/// cannot be read, or holds a picture in a format we do not decode.
///
/// Never fails: artwork is decoration, and a tagger that wrote something
/// strange should not turn into an error in the UI.
pub fn cover_art(path: &str) -> Option<String> {
    if let Some(hit) = CACHE.lock().get(path) {
        return hit.clone();
    }

    // Deliberately not holding the lock across the decode. Two rows asking for
    // the same path at once will both do the work, which is cheap and rare;
    // blocking every other row behind one slow file is neither.
    let thumb = read_thumbnail(Path::new(path));
    CACHE.lock().insert(path.to_owned(), thumb.clone());
    thumb
}

/// Drops everything memoised. For a caller that knows the files changed.
pub fn clear_cache() {
    CACHE.lock().clear();
}

fn read_thumbnail(path: &Path) -> Option<String> {
    let tagged = lofty::read_from_path(path).ok()?;
    let picture = pick_picture(&tagged)?;
    let data_uri = encode_thumbnail(picture.data())?;
    Some(data_uri)
}

/// The front cover if the file says which one that is, otherwise the first
/// picture in the first tag that has one.
///
/// Every tag is searched rather than just the primary one: a file converted
/// along the way can easily end up with the art in an ID3v2 block while its
/// primary tag is something else, and reading only the primary tag reports no
/// artwork on a file that visibly has some.
fn pick_picture(tagged: &lofty::file::TaggedFile) -> Option<&Picture> {
    let pictures = || tagged.tags().iter().flat_map(|tag| tag.pictures());
    pictures()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| pictures().next())
}

/// Scale to `THUMB_PX` and re-encode as PNG, base64'd into a `data:` URI.
///
/// PNG rather than JPEG because the input may carry an alpha channel, and the
/// JPEG encoder refuses anything but RGB -- at this size the two are within a
/// few kilobytes of each other anyway, and PNG needs no quality decision.
fn encode_thumbnail(raw: &[u8]) -> Option<String> {
    let full = image::load_from_memory(raw).ok()?;

    // `resize` fits inside the box and keeps the aspect ratio, so a non-square
    // cover is letterboxed by the caller's CSS rather than squashed here.
    let thumb = full.resize(THUMB_PX, THUMB_PX, FilterType::Lanczos3);

    let mut png = Vec::new();
    thumb
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .ok()?;

    Some(format!("data:image/png;base64,{}", STANDARD.encode(&png)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_none_not_an_error() {
        assert!(cover_art("/nonexistent/capstone/none.wav").is_none());
    }

    #[test]
    fn thumbnail_is_a_png_data_uri_within_the_box() {
        let mut png = Vec::new();
        image::DynamicImage::new_rgba8(1400, 1400)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("encode fixture");

        let uri = encode_thumbnail(&png).expect("thumbnail");
        assert!(uri.starts_with("data:image/png;base64,"));

        let bytes = STANDARD
            .decode(uri.trim_start_matches("data:image/png;base64,"))
            .expect("base64");
        let decoded = image::load_from_memory(&bytes).expect("decode thumbnail");
        assert_eq!((decoded.width(), decoded.height()), (THUMB_PX, THUMB_PX));
    }

    #[test]
    fn junk_bytes_are_none() {
        assert!(encode_thumbnail(b"not a picture").is_none());
    }
}
