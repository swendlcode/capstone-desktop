use super::*;
use crate::models::{PitchMode, Warmth};

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

/// Crossovers meet exactly, which is what lets `multiband` treat each band's
/// `high_hz` as the next one's `low_hz` with no gap to reason about. Asserted
/// for every shipped template, not just Festival: `tile` is what makes it true
/// and a template that stopped using it would be a notch at every band edge.
#[test]
fn every_stock_template_tiles_the_spectrum() {
    for template in stock() {
        let bands = &template.bands;
        assert_eq!(bands.first().unwrap().low_hz, 20.0, "{}", template.name);
        assert_eq!(bands.last().unwrap().high_hz, 20_000.0, "{}", template.name);
        for pair in bands.windows(2) {
            assert_eq!(pair[0].high_hz, pair[1].low_hz, "{}", template.name);
        }
    }
}

#[test]
fn the_shipped_set_is_read_only_and_uniquely_identified() {
    let all = stock();
    assert!(all.iter().all(|t| t.is_stock), "stock templates are read-only");

    let mut ids: Vec<&str> = all.iter().map(|t| t.id.as_str()).collect();
    ids.sort_unstable();
    let count = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), count, "two stock templates share an id");
}

/// Every shipped setting is one a control on the Configure tab could also
/// produce. A stock template outside those ranges would show the user a slider
/// pinned somewhere it cannot reach, and would be silently rewritten the moment
/// they duplicated it.
#[test]
fn every_shipped_setting_is_reachable_from_the_ui() {
    for template in stock() {
        let name = template.name.clone();
        let bounded = share::clamped(template.clone());
        assert_eq!(bounded.target_lufs, template.target_lufs, "{}", name);
        assert_eq!(bounded.limiter_character, template.limiter_character, "{}", name);
        assert_eq!(bounded.repair_above_lufs, template.repair_above_lufs, "{}", name);
        assert_eq!(bounded.clip_threshold_pct, template.clip_threshold_pct, "{}", name);
        assert_eq!(bounded.headroom_db, template.headroom_db, "{}", name);
        assert_eq!(bounded.pitch_tolerance_cents, template.pitch_tolerance_cents, "{}", name);
        for (got, want) in bounded.bands.iter().zip(template.bands.iter()) {
            assert_eq!(
                (got.low_hz, got.high_hz, got.threshold_db, got.ratio, got.attack_ms, got.release_ms, got.makeup_db),
                (want.low_hz, want.high_hz, want.threshold_db, want.ratio, want.attack_ms, want.release_ms, want.makeup_db),
                "{}", name
            );
        }
    }
}

/// The five cover five situations, and the point of a template is not having to
/// touch a slider after picking one. Two templates with the same target and the
/// same ceiling would be the same template with two names.
#[test]
fn the_shipped_templates_are_genuinely_different() {
    let mut pairs: Vec<(String, String)> = stock()
        .iter()
        .map(|t| (format!("{:?}", t.target_lufs), format!("{:?}", t.headroom_db)))
        .collect();
    let count = pairs.len();
    pairs.sort();
    pairs.dedup();
    assert_eq!(pairs.len(), count, "two stock templates land in the same place");
}

/// Vinyl Rip is the one template that corrects pitch, and it resamples: a slow
/// platter stretched pitch and time together, so shifting pitch alone would
/// leave the timing wrong.
#[test]
fn only_the_vinyl_template_corrects_pitch() {
    for template in stock() {
        if template.id == VINYL_ID {
            assert!(template.correct_pitch);
            assert_eq!(template.pitch_mode, PitchMode::ChangeTempo);
        } else {
            assert!(!template.correct_pitch, "{} corrects pitch", template.name);
        }
    }
}

/// Archive turns the multiband off the only way the model allows: 1.0:1 applies
/// no gain reduction whatever the threshold does.
#[test]
fn the_archive_template_applies_no_dynamics() {
    let archive = archive();
    assert!(archive.bands.iter().all(|b| b.ratio == 1.0));
    assert!(archive.bands.iter().all(|b| b.makeup_db == 0.0));
    assert_eq!(archive.warmth, Warmth::None);
    assert!(archive.repair_clips);
}

#[test]
fn an_empty_data_dir_still_gives_a_working_app() {
    let dir = scratch("templates-empty");
    let loaded = load(&dir);
    assert_eq!(loaded.len(), stock().len());
    assert_eq!(loaded[0].id, FESTIVAL_ID);
}

#[test]
fn user_templates_survive_a_round_trip_and_stock_is_not_stored() {
    let dir = scratch("templates-roundtrip");
    let mine = user_template("mine", "Warehouse");
    save(&dir, &[festival(), mine.clone()]).unwrap();

    // Only the user's own template is on disk: stock is code, and storing it
    // would be a second source of truth to drift.
    let raw = std::fs::read_to_string(path(&dir)).unwrap();
    assert!(raw.contains("Warehouse"));
    assert!(!raw.contains("Festival"));

    let loaded = load(&dir);
    assert_eq!(loaded.len(), stock().len() + 1);
    assert_eq!(loaded[0].id, FESTIVAL_ID);
    assert_eq!(loaded.last().unwrap().name, "Warehouse");
    assert!(!loaded.last().unwrap().is_stock);
}

/// A stale copy of a stock template on disk shadowing the shipped one is how a
/// fixed default would fail to reach anybody who had already launched.
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
    assert_eq!(loaded.len(), stock().len());
    assert_eq!(loaded[0].target_lufs, -12.5);
}

#[test]
fn a_malformed_file_does_not_stop_the_app_launching() {
    let dir = scratch("templates-malformed");
    std::fs::write(path(&dir), "{ not json at all").unwrap();
    let loaded = load(&dir);
    assert_eq!(loaded.len(), stock().len());
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
    let mine = loaded.last().unwrap();
    assert_eq!(mine.name, "Warehouse");
    assert_eq!(mine.target_lufs, -12.5);
    assert_eq!(mine.bands.len(), 4);
}

/// A shared file dropped into templates.json by hand reaches the app without
/// passing the importer, so the bound has to be on the load path too.
#[test]
fn a_hand_edited_file_cannot_carry_an_impossible_setting() {
    let dir = scratch("templates-hand-edited");
    std::fs::write(
        path(&dir),
        r#"[{"id":"mine","name":"Hot","isStock":false,"headroomDb":-30,"targetLufs":12}]"#,
    )
    .unwrap();

    let mine = load(&dir).pop().unwrap();
    assert_eq!(mine.name, "Hot");
    assert_eq!(mine.headroom_db, 0.0);
    assert_eq!(mine.target_lufs, -6.0);
}
