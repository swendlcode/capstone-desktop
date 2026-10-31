//! Where a processed file goes.

use std::path::{Path, PathBuf};

use crate::audio::encode;
use crate::error::{CapstoneError, Result};
use crate::models::{OutputFormat, OutputLocation, Settings};

/// Used when the user has chosen "Specific folder" but not yet said which one,
/// so a first run can write something without stopping to ask.
pub fn default_output_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join("Music").join("Capstone")
}

/// The folder the Open Output Folder button reveals. `SameAsSource` has no one
/// folder, so it falls back to the configured one, which is where anything
/// processed before the user switched modes will be.
pub fn resolved_output_dir(settings: &Settings) -> PathBuf {
    settings
        .output_dir
        .as_deref()
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(default_output_dir)
}

/// 16-bit for the formats whose own UI copy names it, and for ALAC because
/// alac-encoder hardcodes it. FLAC is the one place a wider word costs nothing,
/// so it keeps 24.
pub fn bit_depth_for(format: OutputFormat) -> u16 {
    match format {
        OutputFormat::Flac => 24,
        OutputFormat::Aiff | OutputFormat::Alac | OutputFormat::Wav => 16,
        // Not meaningful for a lossy codec; carried only so the spec is one shape.
        OutputFormat::Mp3 => 16,
    }
}

/// Resolves the destination for one source file.
///
/// Mirroring reproduces the source's immediate parent folder under the output
/// directory, not the whole tree from whatever folder was dropped: `QueuedFile`
/// carries no record of which folder a file arrived in, so one level is all
/// that can honestly be reconstructed here. Deepening this means carrying a
/// drop root on the queue row, which is an IPC contract change.
pub fn output_path(source: &Path, settings: &Settings) -> Result<PathBuf> {
    let stem = source
        .file_stem()
        .ok_or_else(|| CapstoneError::Other(format!("{} has no filename", source.display())))?;

    let mut dir = match settings.output_location {
        OutputLocation::SameAsSource => source
            .parent()
            .ok_or_else(|| CapstoneError::Other(format!("{} has no folder", source.display())))?
            .to_path_buf(),
        OutputLocation::Specific => resolved_output_dir(settings),
    };

    if settings.mirror_input_structure && settings.output_location == OutputLocation::Specific {
        if let Some(parent_name) = source.parent().and_then(|p| p.file_name()) {
            dir = dir.join(parent_name);
        }
    }

    let mut dest = dir.join(stem);
    dest.set_extension(encode::extension(settings.format));

    // Writing over the file we are reading would destroy the original, which is
    // the one outcome a batch processor must never produce. It happens whenever
    // the output format matches the input's extension and the location is the
    // source's own folder.
    if dest == source {
        let mut suffixed = stem.to_os_string();
        suffixed.push("-capstone");
        dest = dir.join(suffixed);
        dest.set_extension(encode::extension(settings.format));
    }

    Ok(dest)
}

/// Runs `write` against a temporary sibling of `dest` and moves the result into
/// place only once it has succeeded.
///
/// Nothing may ever appear at `dest` unless it is a complete file. An encode
/// that fails halfway, or a run the user cancelled while a file was being
/// written, would otherwise leave a truncated track in the output folder --
/// which the user finds weeks later in a set, not now.
///
/// The temporary file is a sibling rather than a file in the system temp
/// directory, because `rename` is only atomic within one filesystem and an
/// output folder is routinely an external drive. The name carries a uuid so two
/// workers can never pick the same scratch path.
pub fn write_atomically<F>(dest: &Path, write: F) -> Result<()>
where
    F: FnOnce(&Path) -> Result<()>,
{
    let parent = dest
        .parent()
        .ok_or_else(|| CapstoneError::Other(format!("{} has no folder", dest.display())))?;
    std::fs::create_dir_all(parent)?;

    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "output".to_string());
    let temp = parent.join(format!(".{}.{}.part", name, uuid::Uuid::new_v4()));

    if let Err(e) = write(&temp) {
        // Best effort: the encode already failed, and a scratch file we could
        // not remove is not a reason to report a different error than the one
        // that actually went wrong.
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }

    if let Err(e) = std::fs::rename(&temp, dest) {
        let _ = std::fs::remove_file(&temp);
        return Err(e.into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
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
        let dest = output_path(Path::new("/music/crate/track.aiff"), &s).unwrap();
        assert_eq!(dest, PathBuf::from("/out/track.flac"));
    }

    #[test]
    fn alac_lands_in_a_caf_container() {
        let mut s = settings(OutputLocation::Specific, Some("/out"), false);
        s.format = OutputFormat::Alac;
        let dest = output_path(Path::new("/music/track.wav"), &s).unwrap();
        assert_eq!(dest, PathBuf::from("/out/track.caf"));
    }

    #[test]
    fn same_as_source_writes_beside_the_original() {
        let mut s = settings(OutputLocation::SameAsSource, Some("/out"), false);
        s.format = OutputFormat::Flac;
        let dest = output_path(Path::new("/music/crate/track.wav"), &s).unwrap();
        assert_eq!(dest, PathBuf::from("/music/crate/track.flac"));
    }

    #[test]
    fn mirroring_reproduces_the_source_folder_name() {
        let s = settings(OutputLocation::Specific, Some("/out"), true);
        let dest = output_path(Path::new("/music/Tech House/track.aiff"), &s).unwrap();
        assert_eq!(dest, PathBuf::from("/out/Tech House/track.wav"));
    }

    /// Mirroring has no meaning without a folder to mirror into, and silently
    /// nesting a folder beside the original would scatter the library.
    #[test]
    fn mirroring_is_ignored_when_writing_beside_the_source() {
        let mut s = settings(OutputLocation::SameAsSource, Some("/out"), true);
        s.format = OutputFormat::Flac;
        let dest = output_path(Path::new("/music/crate/track.wav"), &s).unwrap();
        assert_eq!(dest, PathBuf::from("/music/crate/track.flac"));
    }

    /// The one outcome a batch processor must never produce.
    #[test]
    fn never_writes_over_the_file_it_is_reading() {
        let s = settings(OutputLocation::SameAsSource, None, false);
        let source = Path::new("/music/crate/track.wav");
        let dest = output_path(source, &s).unwrap();
        assert_ne!(dest, source);
        assert_eq!(dest, PathBuf::from("/music/crate/track-capstone.wav"));
    }

    #[test]
    fn an_unset_output_dir_falls_back_rather_than_failing() {
        let s = settings(OutputLocation::Specific, None, false);
        let dest = output_path(Path::new("/music/track.aiff"), &s).unwrap();
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
}
