//! Stock templates and template persistence.
//!
//! Stock templates are code, not data: they are rebuilt on every launch and
//! merged over whatever is on disk. That way a shipped tweak to Festival
//! reaches existing installs, and a user who edited a stock template by hand
//! in the JSON file cannot leave themselves with a setting the app no longer
//! understands. Anything the user made carries `is_stock: false` and is loaded
//! back untouched.

use std::path::{Path, PathBuf};

use crate::error::Result;
use crate::models::{BandSetting, PitchMode, Template, Warmth};

const FILE_NAME: &str = "templates.json";

pub const FESTIVAL_ID: &str = "stock-festival";

impl Default for BandSetting {
    fn default() -> Self {
        Self {
            low_hz: 20.0,
            high_hz: 20_000.0,
            threshold_db: -18.0,
            ratio: 2.0,
            attack_ms: 10.0,
            release_ms: 120.0,
            makeup_db: 0.0,
        }
    }
}

/// The defaults from the product contract. `Default` rather than a free
/// function because `Template` is deserialized with a container-level serde
/// default: a field added in a later release fills in from here instead of
/// failing the whole file.
impl Default for Template {
    fn default() -> Self {
        Self {
            id: FESTIVAL_ID.to_string(),
            name: "Festival".to_string(),
            is_stock: true,
            target_lufs: -12.5,
            limiter_character: 4,
            repair_clips: true,
            repair_above_lufs: -14.0,
            clip_threshold_pct: 99.7,
            headroom_db: 1.5,
            warmth: Warmth::Gentle,
            correct_pitch: false,
            pitch_tolerance_cents: 10.0,
            pitch_mode: PitchMode::KeepTempo,
            bands: default_bands(),
        }
    }
}

/// A four-way split: sub/low, body, presence, air. Crossovers meet exactly, so
/// `multiband` can treat `high_hz` as the next band's `low_hz` and never has to
/// reason about a gap or an overlap.
pub fn default_bands() -> Vec<BandSetting> {
    vec![
        BandSetting {
            low_hz: 20.0,
            high_hz: 120.0,
            threshold_db: -16.0,
            ratio: 2.5,
            attack_ms: 20.0,
            release_ms: 200.0,
            makeup_db: 0.0,
        },
        BandSetting {
            low_hz: 120.0,
            high_hz: 800.0,
            threshold_db: -18.0,
            ratio: 2.0,
            attack_ms: 12.0,
            release_ms: 150.0,
            makeup_db: 0.0,
        },
        BandSetting {
            low_hz: 800.0,
            high_hz: 5_000.0,
            threshold_db: -20.0,
            ratio: 1.8,
            attack_ms: 6.0,
            release_ms: 100.0,
            makeup_db: 0.0,
        },
        BandSetting {
            low_hz: 5_000.0,
            high_hz: 20_000.0,
            threshold_db: -22.0,
            ratio: 1.6,
            attack_ms: 3.0,
            release_ms: 80.0,
            makeup_db: 0.0,
        },
    ]
}

pub fn festival() -> Template {
    Template::default()
}

/// Every template that ships with the app.
pub fn stock() -> Vec<Template> {
    vec![festival()]
}

pub fn path(data_dir: &Path) -> PathBuf {
    data_dir.join(FILE_NAME)
}

/// Never fails, for the same reason settings load does not: the stock
/// templates alone are a working app.
pub fn load(data_dir: &Path) -> Vec<Template> {
    let file = path(data_dir);
    let stored: Vec<Template> = if file.exists() {
        match std::fs::read_to_string(&file).map(|s| serde_json::from_str::<Vec<Template>>(&s)) {
            Ok(Ok(list)) => list,
            Ok(Err(e)) => {
                tracing::warn!("templates.json is malformed, using stock only: {}", e);
                Vec::new()
            }
            Err(e) => {
                tracing::warn!("could not read templates.json, using stock only: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    let mut out = stock();
    let stock_ids: Vec<String> = out.iter().map(|t| t.id.clone()).collect();
    // Stock first and user templates after, dropping any stored copy of a
    // stock id. A stale stock template on disk shadowing the shipped one is how
    // a fixed default would fail to reach anybody who had already launched.
    out.extend(
        stored
            .into_iter()
            .filter(|t| !t.is_stock && !stock_ids.contains(&t.id)),
    );
    out
}

/// Writes only what the user owns. Stock templates are code and would be
/// re-merged on load anyway, so storing them would just be a second source of
/// truth to drift.
pub fn save(data_dir: &Path, templates: &[Template]) -> Result<()> {
    std::fs::create_dir_all(data_dir)?;
    let user: Vec<&Template> = templates.iter().filter(|t| !t.is_stock).collect();
    let json = serde_json::to_string_pretty(&user)?;
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

    fn user_template(id: &str, name: &str) -> Template {
        Template {
            id: id.to_string(),
            name: name.to_string(),
            is_stock: false,
            ..Template::default()
        }
    }

    #[test]
    fn festival_carries_the_contract_defaults() {
        let t = festival();
        assert_eq!(t.id, FESTIVAL_ID);
        assert_eq!(t.name, "Festival");
        assert!(t.is_stock);
        assert_eq!(t.target_lufs, -12.5);
        assert_eq!(t.limiter_character, 4);
        assert!(t.repair_clips);
        assert_eq!(t.repair_above_lufs, -14.0);
        assert_eq!(t.clip_threshold_pct, 99.7);
        assert_eq!(t.headroom_db, 1.5);
        assert_eq!(t.warmth, Warmth::Gentle);
        assert!(!t.correct_pitch);
        assert_eq!(t.pitch_tolerance_cents, 10.0);
        assert_eq!(t.pitch_mode, PitchMode::KeepTempo);
        assert_eq!(t.bands.len(), 4);
    }

    /// Crossovers meet exactly, which is what lets `multiband` treat each
    /// band's `high_hz` as the next one's `low_hz` with no gap to reason about.
    #[test]
    fn the_default_bands_tile_the_spectrum() {
        let bands = default_bands();
        for pair in bands.windows(2) {
            assert_eq!(pair[0].high_hz, pair[1].low_hz);
        }
    }

    #[test]
    fn an_empty_data_dir_still_gives_a_working_app() {
        let dir = scratch("templates-empty");
        let loaded = load(&dir);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, FESTIVAL_ID);
    }

    #[test]
    fn user_templates_survive_a_round_trip_and_stock_is_not_stored() {
        let dir = scratch("templates-roundtrip");
        let mine = user_template("mine", "Warehouse");
        save(&dir, &[festival(), mine.clone()]).unwrap();

        // Only the user's own template is on disk: stock is code, and storing
        // it would be a second source of truth to drift.
        let raw = std::fs::read_to_string(path(&dir)).unwrap();
        assert!(raw.contains("Warehouse"));
        assert!(!raw.contains("Festival"));

        let loaded = load(&dir);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].id, FESTIVAL_ID);
        assert_eq!(loaded[1].name, "Warehouse");
        assert!(!loaded[1].is_stock);
    }

    /// A stale copy of a stock template on disk shadowing the shipped one is how
    /// a fixed default would fail to reach anybody who had already launched.
    #[test]
    fn a_stored_copy_of_a_stock_id_is_dropped() {
        let dir = scratch("templates-shadow");
        let stale = Template {
            name: "Festival (old)".to_string(),
            is_stock: false,
            target_lufs: -6.0,
            ..festival()
        };
        std::fs::write(path(&dir), serde_json::to_string(&vec![stale]).unwrap()).unwrap();

        let loaded = load(&dir);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].target_lufs, -12.5);
    }

    #[test]
    fn a_malformed_file_does_not_stop_the_app_launching() {
        let dir = scratch("templates-malformed");
        std::fs::write(path(&dir), "{ not json at all").unwrap();
        let loaded = load(&dir);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, FESTIVAL_ID);
    }

    /// The container-level serde default is what lets a file written by an older
    /// build survive a release that adds a field.
    #[test]
    fn a_template_missing_newer_fields_fills_in_from_defaults() {
        let dir = scratch("templates-partial");
        std::fs::write(
            path(&dir),
            r#"[{"id":"mine","name":"Warehouse","isStock":false}]"#,
        )
        .unwrap();

        let loaded = load(&dir);
        assert_eq!(loaded.len(), 2);
        let mine = &loaded[1];
        assert_eq!(mine.name, "Warehouse");
        assert_eq!(mine.target_lufs, -12.5);
        assert_eq!(mine.bands.len(), 4);
    }
}
