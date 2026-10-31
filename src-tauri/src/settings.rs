//! Settings load/save. One JSON file in the app data dir, no database: there is
//! exactly one of these and it is small enough that a torn write is cheaper to
//! recover from than a schema is to maintain.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::models::{Mp3Rate, OutputFormat, OutputLocation, Settings};

const FILE_NAME: &str = "settings.json";

impl Default for Settings {
    fn default() -> Self {
        Self {
            output_location: OutputLocation::Specific,
            // None means "not chosen yet". The job runner falls back to a
            // Capstone folder under the user's audio directory, so a first run
            // never has to ask before it can write anything.
            output_dir: None,
            mirror_input_structure: false,
            // AIFF 16/44 is the safe default: every DJ program on both
            // platforms opens it and nothing is thrown away.
            format: OutputFormat::Aiff,
            mp3_rate: Mp3Rate::Cbr320,
            max_cores: default_max_cores(),
            active_template_id: crate::templates::FESTIVAL_ID.to_string(),
        }
    }
}

/// All of them, which is what the Processor Load pane shows as the default.
pub fn default_max_cores() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Never fails: a missing, unreadable or malformed file means the defaults.
/// Refusing to start because settings.json has a stray comma would be worse
/// than losing a preference.
pub fn load(data_dir: &Path) -> Settings {
    let file = path(data_dir);
    if !file.exists() {
        return Settings::default();
    }
    match std::fs::read_to_string(&file).map(|s| serde_json::from_str::<Settings>(&s)) {
        Ok(Ok(mut settings)) => {
            // A stored zero (or a count from a machine with more cores) would
            // size the rayon pool to nothing, so the floor is enforced on read
            // rather than trusted from disk.
            if settings.max_cores == 0 {
                settings.max_cores = default_max_cores();
            }
            settings
        }
        Ok(Err(e)) => {
            tracing::warn!("settings.json is malformed, using defaults: {}", e);
            Settings::default()
        }
        Err(e) => {
            tracing::warn!("could not read settings.json, using defaults: {}", e);
            Settings::default()
        }
    }
}

pub fn save(data_dir: &Path, settings: &Settings) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let json = serde_json::to_string_pretty(settings)?;
    std::fs::write(path(data_dir), json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join("capstone-tests")
            .join(format!("{}-{}", tag, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_empty_data_dir_gives_the_documented_defaults() {
        let settings = load(&scratch("settings-empty"));
        assert_eq!(settings.output_location, OutputLocation::Specific);
        assert_eq!(settings.output_dir, None);
        assert!(!settings.mirror_input_structure);
        assert_eq!(settings.format, OutputFormat::Aiff);
        assert_eq!(settings.mp3_rate, Mp3Rate::Cbr320);
        assert_eq!(settings.max_cores, default_max_cores());
        assert_eq!(settings.active_template_id, crate::templates::FESTIVAL_ID);
    }

    #[test]
    fn settings_survive_a_round_trip() {
        let dir = scratch("settings-roundtrip");
        let written = Settings {
            format: OutputFormat::Flac,
            output_dir: Some("/out".to_string()),
            mirror_input_structure: true,
            max_cores: 2,
            ..Settings::default()
        };
        save(&dir, &written).unwrap();

        let read = load(&dir);
        assert_eq!(read.format, OutputFormat::Flac);
        assert_eq!(read.output_dir.as_deref(), Some("/out"));
        assert!(read.mirror_input_structure);
        assert_eq!(read.max_cores, 2);
    }

    /// The stored file is camelCase on both sides of the bridge. A snake_case
    /// key here would deserialize to the default and the Personalize pane would
    /// quietly forget the setting.
    #[test]
    fn the_stored_file_is_camel_case() {
        let dir = scratch("settings-casing");
        let written = Settings {
            mirror_input_structure: true,
            ..Settings::default()
        };
        save(&dir, &written).unwrap();

        let raw = std::fs::read_to_string(path(&dir)).unwrap();
        assert!(raw.contains("mirrorInputStructure"));
        assert!(raw.contains("activeTemplateId"));
        assert!(!raw.contains("mirror_input_structure"));
    }

    /// A zero would size the rayon pool to nothing, so it is corrected on read
    /// rather than trusted from disk.
    #[test]
    fn a_zero_core_count_on_disk_is_corrected() {
        let dir = scratch("settings-zero-cores");
        std::fs::write(path(&dir), r#"{"maxCores":0}"#).unwrap();
        assert_eq!(load(&dir).max_cores, default_max_cores());
    }

    #[test]
    fn a_malformed_file_does_not_stop_the_app_launching() {
        let dir = scratch("settings-malformed");
        std::fs::write(path(&dir), "not json").unwrap();
        assert_eq!(load(&dir).format, OutputFormat::Aiff);
    }
}
