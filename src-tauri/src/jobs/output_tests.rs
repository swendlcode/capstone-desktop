//! Tests for [`super`]: where a processed file goes, and the temp-and-rename
//! that keeps a half-written one from ever appearing there.
//!
//! Split out of output.rs to keep that module inside the size cap.

use super::*;
use crate::models::{CollisionPolicy, Mp3Rate};

fn settings(location: OutputLocation, dir: Option<&str>, mirror: bool) -> Settings {
    Settings {
        output_location: location,
        output_dir: dir.map(str::to_string),
        mirror_input_structure: mirror,
        format: OutputFormat::Wav,
        mp3_rate: Mp3Rate::Cbr320,
        max_cores: 1,
        active_template_id: crate::templates::FESTIVAL_ID.to_string(),
        collision_policy: CollisionPolicy::Overwrite,
        keep_tags: true,
        embed_artwork: false,
        artwork_dir: None,
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

/// `set_extension` replaces everything after the last dot in the stem, and a
/// track title routinely has one. Every case here is a real file that came out
/// of Capstone truncated.
#[test]
fn a_dot_in_the_title_does_not_truncate_the_name() {
    let mut s = settings(OutputLocation::Specific, Some("/out"), false);
    s.format = OutputFormat::Aiff;

    for (source, expected) in [
        (
            "/in/Dom Dolla - Strangers ft. Mansionair.wav",
            "/out/Dom Dolla - Strangers ft. Mansionair.aiff",
        ),
        (
            "/in/Fred again.. - Delilah.wav",
            "/out/Fred again.. - Delilah.aiff",
        ),
        ("/in/kai.wav & ....wav", "/out/kai.wav & ....aiff"),
        // A leading dot is the whole name, not an extension: `file_stem` keeps
        // it, and appending must not eat it either.
        ("/in/.hidden track.wav", "/out/.hidden track.aiff"),
    ] {
        assert_eq!(
            output_path(Path::new(source), None, &s).unwrap(),
            PathBuf::from(expected),
            "{source}",
        );
    }
}

/// Two sources whose names differ only after a dot used to truncate onto one
/// destination, and `write_atomically` replaces, so the second silently
/// overwrote the first.
#[test]
fn names_that_differ_after_a_dot_keep_separate_destinations() {
    let mut s = settings(OutputLocation::Specific, Some("/out"), false);
    s.format = OutputFormat::Aiff;
    let a = output_path(Path::new("/in/Artist - Strangers ft. Mansionair.wav"), None, &s).unwrap();
    let b = output_path(Path::new("/in/Artist - Strangers ft. Nobody.wav"), None, &s).unwrap();
    assert_ne!(a, b);
}

/// The self-overwrite guard builds its name the same way, so a dotted title
/// must not truncate on that branch either.
#[test]
fn the_self_overwrite_suffix_keeps_a_dotted_title_whole() {
    let s = settings(OutputLocation::SameAsSource, None, false);
    let source = Path::new("/music/Fred again.. - Delilah.wav");
    let dest = output_path(source, None, &s).unwrap();
    assert_ne!(dest, source);
    assert_eq!(
        dest,
        PathBuf::from("/music/Fred again.. - Delilah-capstone.wav")
    );
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

// ---------------------------------------------------------------------------
// Collisions
// ---------------------------------------------------------------------------

/// Two sources resolving to one destination is the case the truncation bug
/// made routine, and the one where silently replacing loses a track.
#[test]
fn two_sources_in_one_run_never_land_on_the_same_file() {
    let dir = scratch("collide-run");
    let out = dir.to_string_lossy().to_string();
    let mut s = settings(OutputLocation::Specific, Some(&out), false);
    s.format = OutputFormat::Aiff;

    let claims = Destinations::new();
    let first = claims
        .resolve(Path::new("/in/track.wav"), None, &s)
        .unwrap();
    let second = claims
        .resolve(Path::new("/in/track.flac"), None, &s)
        .unwrap();

    let Destination::Write { path: a, note: none } = first else {
        panic!("the first file should write its own name");
    };
    assert_eq!(a, dir.join("track.aiff"));
    assert!(none.is_none());

    let Destination::Write { path: b, note } = second else {
        panic!("the second file must still be written, not skipped");
    };
    assert_eq!(b, dir.join("track (2).aiff"));
    assert!(note.unwrap().contains("another file in this run"));
}

/// The in-run rule holds whatever the policy is: `overwrite` is about the file
/// that was there before the run, never about the one we just wrote.
#[test]
fn overwrite_does_not_mean_overwriting_this_runs_own_output() {
    let dir = scratch("collide-overwrite");
    let out = dir.to_string_lossy().to_string();
    let mut s = settings(OutputLocation::Specific, Some(&out), false);
    s.collision_policy = CollisionPolicy::Overwrite;

    let claims = Destinations::new();
    let a = claims.resolve(Path::new("/in/t.wav"), None, &s).unwrap();
    let b = claims.resolve(Path::new("/in/t.aiff"), None, &s).unwrap();
    match (a, b) {
        (Destination::Write { path: first, .. }, Destination::Write { path: second, .. }) => {
            assert_ne!(first, second)
        }
        _ => panic!("both files must be written"),
    }
}

#[test]
fn skip_leaves_an_existing_output_alone() {
    let dir = scratch("collide-skip");
    let existing = dir.join("track.wav");
    std::fs::write(&existing, b"already mastered").unwrap();

    let out = dir.to_string_lossy().to_string();
    let mut s = settings(OutputLocation::Specific, Some(&out), false);
    s.collision_policy = CollisionPolicy::Skip;

    let claims = Destinations::new();
    assert_eq!(
        claims.resolve(Path::new("/in/track.flac"), None, &s).unwrap(),
        Destination::Skip { existing: existing.clone() }
    );
    // Untouched, which is the whole point.
    assert_eq!(std::fs::read(&existing).unwrap(), b"already mastered");
}

#[test]
fn rename_writes_beside_an_existing_output() {
    let dir = scratch("collide-rename");
    std::fs::write(dir.join("track.wav"), b"old").unwrap();
    let out = dir.to_string_lossy().to_string();
    let mut s = settings(OutputLocation::Specific, Some(&out), false);
    s.collision_policy = CollisionPolicy::Rename;

    let claims = Destinations::new();
    let Destination::Write { path, note } =
        claims.resolve(Path::new("/in/track.flac"), None, &s).unwrap()
    else {
        panic!("rename must still write");
    };
    // `settings()` builds a WAV-format Settings, so the destination is
    // `track.wav`, which already exists.
    assert_eq!(path, dir.join("track (2).wav"));
    assert!(note.unwrap().contains("already existed"));
}

#[test]
fn overwrite_replaces_an_existing_output() {
    let dir = scratch("collide-replace");
    std::fs::write(dir.join("track.wav"), b"old").unwrap();
    let out = dir.to_string_lossy().to_string();
    let s = settings(OutputLocation::Specific, Some(&out), false);

    let claims = Destinations::new();
    assert_eq!(
        claims.resolve(Path::new("/in/track.flac"), None, &s).unwrap(),
        Destination::Write { path: dir.join("track.wav"), note: None }
    );
}

/// The variant suffix goes before the extension and must not cut a dotted
/// title, the same rule `output_path` follows.
#[test]
fn a_renamed_variant_keeps_a_dotted_title_whole() {
    let dir = scratch("collide-dotted");
    let out = dir.to_string_lossy().to_string();
    let mut s = settings(OutputLocation::Specific, Some(&out), false);
    s.format = OutputFormat::Aiff;

    let claims = Destinations::new();
    claims
        .resolve(Path::new("/in/Fred again.. - Delilah.wav"), None, &s)
        .unwrap();
    let Destination::Write { path, .. } = claims
        .resolve(Path::new("/in/Fred again.. - Delilah.flac"), None, &s)
        .unwrap()
    else {
        panic!("expected a write");
    };
    assert_eq!(path, dir.join("Fred again.. - Delilah (2).aiff"));
}
