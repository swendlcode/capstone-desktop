//! Tests for [`super`]: where a processed file goes, and the temp-and-rename
//! that keeps a half-written one from ever appearing there.
//!
//! Split out of output.rs to keep that module inside the size cap.

use super::*;
use crate::models::Mp3Rate;

fn settings(location: OutputLocation, dir: Option<&str>, mirror: bool) -> Settings {
    Settings {
        output_location: location,
        output_dir: dir.map(str::to_string),
        mirror_input_structure: mirror,
        format: OutputFormat::Wav,
        mp3_rate: Mp3Rate::Cbr320,
        max_cores: 1,
        active_template_id: crate::templates::FESTIVAL_ID.to_string(),
    }
}

#[test]
fn writes_into_the_chosen_folder_with_the_format_extension() {
    let mut s = settings(OutputLocation::Specific, Some("/out"), false);
    s.format = OutputFormat::Flac;
    let dest = output_path(Path::new("/music/crate/track.aiff"), None, &s).unwrap();
    assert_eq!(dest, PathBuf::from("/out/track.flac"));
}

#[test]
fn alac_lands_in_a_caf_container() {
    let mut s = settings(OutputLocation::Specific, Some("/out"), false);
    s.format = OutputFormat::Alac;
    let dest = output_path(Path::new("/music/track.wav"), None, &s).unwrap();
    assert_eq!(dest, PathBuf::from("/out/track.caf"));
}

#[test]
fn same_as_source_writes_beside_the_original() {
    let mut s = settings(OutputLocation::SameAsSource, Some("/out"), false);
    s.format = OutputFormat::Flac;
    let dest = output_path(Path::new("/music/crate/track.wav"), None, &s).unwrap();
    assert_eq!(dest, PathBuf::from("/music/crate/track.flac"));
}

#[test]
fn mirroring_reproduces_the_whole_tree_below_the_dropped_folder() {
    let s = settings(OutputLocation::Specific, Some("/out"), true);
    let dest = output_path(
        Path::new("/music/Tech House/Bass/Deep/track.aiff"),
        Some(Path::new("/music/Tech House")),
        &s,
    )
    .unwrap();
    assert_eq!(dest, PathBuf::from("/out/Tech House/Bass/Deep/track.wav"));
}

#[test]
fn mirroring_keeps_the_dropped_folders_own_name() {
    // Two crates each with a Bass subfolder must not merge into one.
    let s = settings(OutputLocation::Specific, Some("/out"), true);
    let a = output_path(
        Path::new("/music/Crate A/Bass/t.wav"),
        Some(Path::new("/music/Crate A")),
        &s,
    )
    .unwrap();
    let b = output_path(
        Path::new("/music/Crate B/Bass/t.wav"),
        Some(Path::new("/music/Crate B")),
        &s,
    )
    .unwrap();
    assert_eq!(a, PathBuf::from("/out/Crate A/Bass/t.wav"));
    assert_eq!(b, PathBuf::from("/out/Crate B/Bass/t.wav"));
    assert_ne!(a, b);
}

#[test]
fn mirroring_a_file_at_the_dropped_root_keeps_only_that_folder() {
    let s = settings(OutputLocation::Specific, Some("/out"), true);
    let dest = output_path(
        Path::new("/music/Tech House/track.aiff"),
        Some(Path::new("/music/Tech House")),
        &s,
    )
    .unwrap();
    assert_eq!(dest, PathBuf::from("/out/Tech House/track.wav"));
}

#[test]
fn an_individually_picked_file_has_no_tree_to_mirror() {
    let s = settings(OutputLocation::Specific, Some("/out"), true);
    let dest = output_path(Path::new("/music/Tech House/track.aiff"), None, &s).unwrap();
    assert_eq!(dest, PathBuf::from("/out/track.wav"));
}

/// Mirroring has no meaning without a folder to mirror into, and silently
/// nesting a folder beside the original would scatter the library.
#[test]
fn mirroring_is_ignored_when_writing_beside_the_source() {
    let mut s = settings(OutputLocation::SameAsSource, Some("/out"), true);
    s.format = OutputFormat::Flac;
    let dest = output_path(Path::new("/music/crate/track.wav"), None, &s).unwrap();
    assert_eq!(dest, PathBuf::from("/music/crate/track.flac"));
}

/// The one outcome a batch processor must never produce.
#[test]
fn never_writes_over_the_file_it_is_reading() {
    let s = settings(OutputLocation::SameAsSource, None, false);
    let source = Path::new("/music/crate/track.wav");
    let dest = output_path(source, None, &s).unwrap();
    assert_ne!(dest, source);
    assert_eq!(dest, PathBuf::from("/music/crate/track-capstone.wav"));
}

#[test]
fn an_unset_output_dir_falls_back_rather_than_failing() {
    let s = settings(OutputLocation::Specific, None, false);
    let dest = output_path(Path::new("/music/track.aiff"), None, &s).unwrap();
    assert_eq!(dest, default_output_dir().join("track.wav"));

    // An empty string is the same as unset: it is what a cleared text field
    // sends, and joining it would produce a path relative to the cwd.
    let blank = settings(OutputLocation::Specific, Some(""), false);
    assert_eq!(resolved_output_dir(&blank), default_output_dir());
}

#[test]
fn flac_is_the_only_format_that_keeps_a_wider_word() {
    assert_eq!(bit_depth_for(OutputFormat::Flac), 24);
    for format in [
        OutputFormat::Aiff,
        OutputFormat::Alac,
        OutputFormat::Wav,
        OutputFormat::Mp3,
    ] {
        assert_eq!(bit_depth_for(format), 16);
    }
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join("capstone-tests")
        .join(format!("{}-{}", tag, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_successful_write_lands_at_the_destination() {
    let dest = scratch("atomic-ok").join("deep").join("track.wav");
    write_atomically(&dest, |temp| {
        std::fs::write(temp, b"finished")?;
        Ok(())
    })
    .unwrap();

    assert_eq!(std::fs::read(&dest).unwrap(), b"finished");
    // The folder holds the finished file and nothing else.
    let left: Vec<_> = std::fs::read_dir(dest.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left.len(), 1);
}

/// The whole point of the temp-and-rename: a failure leaves the output
/// folder exactly as it was.
#[test]
fn a_failed_write_leaves_nothing_behind() {
    let dir = scratch("atomic-fail");
    let dest = dir.join("track.wav");
    let outcome = write_atomically(&dest, |temp| {
        // A half-written file, then the failure, which is what a cancelled
        // or broken encode actually looks like.
        std::fs::write(temp, b"half")?;
        Err(CapstoneError::Encode("ran out of samples".to_string()))
    });

    assert!(outcome.is_err());
    assert!(!dest.exists());
    assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 0);
}

/// The scratch file is a sibling, because `rename` is only atomic within one
/// filesystem and an output folder is routinely an external drive.
#[test]
fn the_scratch_file_sits_beside_the_destination() {
    let dir = scratch("atomic-sibling");
    let dest = dir.join("track.wav");
    write_atomically(&dest, |temp| {
        assert_eq!(temp.parent(), Some(dir.as_path()));
        std::fs::write(temp, b"x")?;
        Ok(())
    })
    .unwrap();
}

/// Reprocessing a file the user has already processed replaces the output
/// rather than failing on the existing one.
#[test]
fn an_existing_output_is_replaced() {
    let dir = scratch("atomic-replace");
    let dest = dir.join("track.wav");
    std::fs::write(&dest, b"old").unwrap();

    write_atomically(&dest, |temp| {
        std::fs::write(temp, b"new")?;
        Ok(())
    })
    .unwrap();

    assert_eq!(std::fs::read(&dest).unwrap(), b"new");
}
