//! Where a processed file goes.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::audio::encode;
use crate::error::{CapstoneError, Result};
use crate::models::{CollisionPolicy, OutputFormat, OutputLocation, Settings};

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
///
/// The extension is **appended to the whole stem**, never applied with
/// `set_extension`. `set_extension` replaces everything after the last dot in
/// the name it is given, and a track title routinely contains one: a stem of
/// `Dom Dolla - Strangers ft. Mansionair` has an "extension" of ` Mansionair`
/// by that rule, so the destination came out as `Dom Dolla - Strangers ft.aiff`.
/// `Fred again.. - Delilah` became `Fred again..aiff` and `kai.wav & ...`
/// became `kai.aiff`. Two sources can truncate onto one name that way, and the
/// write is atomic-replace, so the second silently overwrote the first.
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

    let mut dest = dir.join(named(stem, "", settings.format));

    // Writing over the file we are reading would destroy the original, which is
    // the one outcome a batch processor must never produce. It happens whenever
    // the output format matches the input's extension and the location is the
    // source's own folder.
    if dest == source {
        dest = dir.join(named(stem, "-capstone", settings.format));
    }

    Ok(dest)
}

/// `<stem><suffix>.<ext>`, built by appending rather than by `set_extension`.
///
/// Kept as one function so the self-overwrite branch cannot drift from the
/// ordinary one, and so there is a single place the rule above is enforced.
fn named(stem: &OsStr, suffix: &str, format: OutputFormat) -> OsString {
    let mut name = stem.to_os_string();
    if !suffix.is_empty() {
        name.push(suffix);
    }
    name.push(".");
    name.push(encode::extension(format));
    name
}

// ---------------------------------------------------------------------------
// Collisions
// ---------------------------------------------------------------------------

/// What one source resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// Write here. `note` is set when the path is not the obvious one.
    Write { path: PathBuf, note: Option<String> },
    /// Leave the existing file alone and do not process the source at all.
    Skip { existing: PathBuf },
}

/// The destinations claimed so far in one run.
///
/// Exists because `output_path` is a pure function of one source, and two
/// different sources can resolve to one destination: `track.wav` and
/// `track.flac` both become `track.aiff`. The write is a temp-and-rename, so
/// the second would replace the first and the run would report two successes
/// and leave one file. The truncation bug made that common rather than rare —
/// every title with a dot in it collapsed onto a shorter name — but it was
/// always reachable, and a policy of `overwrite` must not mean "overwrite the
/// track I just mastered".
#[derive(Debug, Default)]
pub struct Destinations {
    claimed: Mutex<HashSet<PathBuf>>,
}

impl Destinations {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves one source, claiming whatever it returns.
    ///
    /// Called from the worker threads, so the claim and the existence check
    /// happen under one lock: two threads resolving onto the same free name
    /// would otherwise both take it.
    pub fn resolve(
        &self,
        source: &Path,
        source_root: Option<&Path>,
        settings: &Settings,
    ) -> Result<Destination> {
        let base = output_path(source, source_root, settings)?;
        let mut claimed = self
            .claimed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Within this run: always rename apart, whatever the policy says.
        if claimed.contains(&base) {
            let path = free_variant(&base, &claimed);
            claimed.insert(path.clone());
            let note = Some(format!(
                "another file in this run was already writing {}; written as {} instead",
                name_of(&base),
                name_of(&path)
            ));
            return Ok(Destination::Write { path, note });
        }

        // Already on disk before the run: the user's policy decides.
        if base.exists() {
            match settings.collision_policy {
                CollisionPolicy::Skip => {
                    return Ok(Destination::Skip { existing: base });
                }
                CollisionPolicy::Rename => {
                    let path = free_variant(&base, &claimed);
                    claimed.insert(path.clone());
                    let note = Some(format!(
                        "{} already existed; written as {} instead",
                        name_of(&base),
                        name_of(&path)
                    ));
                    return Ok(Destination::Write { path, note });
                }
                CollisionPolicy::Overwrite => {}
            }
        }

        claimed.insert(base.clone());
        Ok(Destination::Write { path: base, note: None })
    }
}

/// `name (2).aiff`, `name (3).aiff`, … — the first that is neither on disk nor
/// already claimed by this run.
///
/// The suffix goes before the extension and is appended to the *whole* stem,
/// for the reason in `output_path`: `set_extension` on a dotted title cuts it.
fn free_variant(base: &Path, claimed: &HashSet<PathBuf>) -> PathBuf {
    let dir = base.parent().unwrap_or(Path::new(""));
    let stem = base.file_stem().unwrap_or_default();
    let extension = base.extension().unwrap_or_default();

    // Bounded rather than `loop`: a folder holding thousands of variants of
    // one name is a bug somewhere else, and a runaway loop here would hang a
    // worker thread with the lock held.
    for n in 2..=9999 {
        let mut name = stem.to_os_string();
        name.push(format!(" ({n})"));
        if !extension.is_empty() {
            name.push(".");
            name.push(extension);
        }
        let candidate = dir.join(name);
        if !candidate.exists() && !claimed.contains(&candidate) {
            return candidate;
        }
    }

    // Nothing sensible is left; fall back to a name that cannot collide.
    let mut name = stem.to_os_string();
    name.push(format!("-{}", uuid::Uuid::new_v4()));
    if !extension.is_empty() {
        name.push(".");
        name.push(extension);
    }
    dir.join(name)
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
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
