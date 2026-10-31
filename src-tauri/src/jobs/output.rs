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
/// Mirroring reproduces the source's path *relative to the folder the user
/// dropped*, which is what "Mirror structure of input folders" promises. The
/// drop root rides on the queue row for exactly this reason; an earlier version
/// had only the file path and could reproduce one level, which silently
/// flattened a nested crate into a single folder.
///
/// `source_root` is `None` for an individually picked file. There is no tree to
/// mirror in that case, so the file lands directly in the output folder.
pub fn output_path(
    source: &Path,
    source_root: Option<&Path>,
    settings: &Settings,
) -> Result<PathBuf> {
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
        // The dropped folder's own name is kept, so dropping two crates with a
        // "Bass" subfolder each does not merge them.
        if let Some(root) = source_root {
            if let Ok(relative) = source.strip_prefix(root) {
                if let Some(sub) = relative.parent().filter(|p| !p.as_os_str().is_empty()) {
                    dir = match root.file_name() {
                        Some(root_name) => dir.join(root_name).join(sub),
                        None => dir.join(sub),
                    };
                } else if let Some(root_name) = root.file_name() {
                    dir = dir.join(root_name);
                }
            }
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
#[path = "output_tests.rs"]
mod tests;
