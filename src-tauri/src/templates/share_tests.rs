use super::*;
use crate::templates;

/// A shared file with every field at a sane value, as text, so the tests edit
/// the JSON the way a person with a text editor would.
fn file_with(patch: &[(&str, &str)]) -> String {
    let base = to_json(&templates::club(), "Hotter and glued, for a club system.").unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&base).unwrap();
    for (key, raw) in patch {
        value[*key] = serde_json::from_str(raw).unwrap();
    }
    value.to_string()
}

#[test]
fn a_template_survives_a_round_trip() {
    let source = templates::vinyl_rip();
    let json = to_json(&source, "Vinyl rips: corrects a deck off speed.").unwrap();
    let (back, description) = from_json(&json, &[]).unwrap();

    assert_eq!(description, "Vinyl rips: corrects a deck off speed.");
    assert_eq!(back.name, source.name);
    assert_eq!(back.target_lufs, source.target_lufs);
    assert_eq!(back.limiter_character, source.limiter_character);
    assert_eq!(back.pitch_mode, source.pitch_mode);
    assert!(back.correct_pitch);
    assert_eq!(back.bands.len(), source.bands.len());
    for (got, want) in back.bands.iter().zip(source.bands.iter()) {
        assert_eq!(got.low_hz, want.low_hz);
        assert_eq!(got.high_hz, want.high_hz);
        assert_eq!(got.ratio, want.ratio);
    }
}

/// An imported template is the user's, whatever the file claimed: a stock id or
/// `isStock: true` would be dropped on the next launch by `templates::load`.
#[test]
fn an_import_is_always_a_user_template() {
    let (imported, _) = from_json(&file_with(&[]), &[]).unwrap();
    assert!(!imported.is_stock);
    assert!(templates::stock().iter().all(|s| s.id != imported.id));
}

/// The headroom is the limiter's ceiling. A file asking for none of it would
/// put the ceiling at or above full scale, which is the one setting that can
/// make the output worse than the input.
#[test]
fn an_impossible_ceiling_is_clamped_not_honoured() {
    let (imported, _) = from_json(&file_with(&[("headroomDb", "-40")]), &[]).unwrap();
    assert_eq!(imported.headroom_db, 0.0);

    let (imported, _) = from_json(&file_with(&[("headroomDb", "999")]), &[]).unwrap();
    assert_eq!(imported.headroom_db, 6.0);
}

/// `makeup_db` is raw gain on a band, so an unbounded one is a loud accident
/// rather than a louder template.
#[test]
fn band_gain_is_clamped() {
    let bands = r#"[
      {"lowHz":20,"highHz":120,"thresholdDb":-16,"ratio":2.5,"attackMs":20,"releaseMs":200,"makeupDb":400},
      {"lowHz":120,"highHz":20000,"thresholdDb":-18,"ratio":900,"attackMs":0,"releaseMs":0,"makeupDb":-400}
    ]"#;
    let (imported, _) = from_json(&file_with(&[("bands", bands)]), &[]).unwrap();

    assert_eq!(imported.bands[0].makeup_db, 12.0);
    assert_eq!(imported.bands[1].makeup_db, -12.0);
    assert_eq!(imported.bands[1].ratio, 20.0);
    // A zero attack or release is a divide waiting to happen in the envelope.
    assert_eq!(imported.bands[0].attack_ms, 20.0);
    assert_eq!(imported.bands[1].attack_ms, 0.1);
    assert_eq!(imported.bands[1].release_ms, 10.0);
}

#[test]
fn every_numeric_field_lands_inside_the_tabs_own_range() {
    let hot = file_with(&[
        ("targetLufs", "6"),
        ("limiterCharacter", "250"),
        ("repairAboveLufs", "40"),
        ("clipThresholdPct", "1000"),
        ("pitchToleranceCents", "-90"),
    ]);
    let (imported, _) = from_json(&hot, &[]).unwrap();

    assert_eq!(imported.target_lufs, -6.0);
    assert_eq!(imported.limiter_character, 10);
    assert_eq!(imported.repair_above_lufs, 0.0);
    assert_eq!(imported.clip_threshold_pct, 100.0);
    assert_eq!(imported.pitch_tolerance_cents, 1.0);
}

/// `multiband` reads each `high_hz` as the next band's `low_hz` without
/// checking, so a file with a gap has to come back tiled or the output has a
/// notch at every edge and nothing in the UI explains it.
#[test]
fn a_band_list_with_a_gap_comes_back_tiled() {
    let bands = r#"[
      {"lowHz":20,"highHz":9000,"thresholdDb":-16,"ratio":2,"attackMs":10,"releaseMs":100,"makeupDb":0},
      {"lowHz":400,"highHz":300,"thresholdDb":-18,"ratio":2,"attackMs":10,"releaseMs":100,"makeupDb":0},
      {"lowHz":7000,"highHz":1000000,"thresholdDb":-20,"ratio":2,"attackMs":10,"releaseMs":100,"makeupDb":0}
    ]"#;
    let (imported, _) = from_json(&file_with(&[("bands", bands)]), &[]).unwrap();

    assert_eq!(imported.bands.len(), 3);
    assert_eq!(imported.bands[0].low_hz, 20.0);
    assert_eq!(imported.bands.last().unwrap().high_hz, 20_000.0);
    for pair in imported.bands.windows(2) {
        assert_eq!(pair[0].high_hz, pair[1].low_hz);
        assert!(pair[0].high_hz > pair[0].low_hz);
    }
}

#[test]
fn a_file_that_is_not_a_template_is_refused_with_a_sentence() {
    for raw in ["{}", "[]", "not json at all", r#"{"formatVersion":1}"#] {
        let err = from_json(raw, &[]).unwrap_err().to_string();
        assert!(!err.is_empty(), "{} produced an empty message", raw);
    }
}

#[test]
fn a_future_format_version_is_refused_rather_than_guessed_at() {
    let err = from_json(&file_with(&[("formatVersion", "99")]), &[])
        .unwrap_err()
        .to_string();
    assert!(err.contains("99"), "the message should name the version: {}", err);
}

/// An f32 field cannot hold 1e300, so it arrives as infinity. Clamping that
/// would be guessing at what the writer meant; `f64::clamp` panics on NaN from
/// the same route.
#[test]
fn a_value_too_large_to_be_a_number_is_refused_rather_than_clamped() {
    let bands = r#"[
      {"lowHz":20,"highHz":20000,"thresholdDb":-1e300,"ratio":2,"attackMs":10,"releaseMs":100,"makeupDb":0}
    ]"#;
    assert!(from_json(&file_with(&[("bands", bands)]), &[]).is_err());
}

#[test]
fn a_file_too_big_to_be_a_template_is_refused_before_it_is_parsed() {
    let padding = " ".repeat(MAX_FILE_BYTES + 1);
    assert!(from_json(&padding, &[]).is_err());
}

#[test]
fn a_colliding_name_gets_a_suffix_rather_than_overwriting() {
    let taken = vec!["Warehouse".to_string(), "Warehouse 2".to_string()];
    assert_eq!(unique_name("Warehouse", &taken), "Warehouse 3");
    assert_eq!(unique_name("  Warehouse  ", &taken), "Warehouse 3");
    assert_eq!(unique_name("Basement", &taken), "Basement");
    assert_eq!(unique_name("   ", &taken), "Imported");

    let (imported, _) = from_json(&file_with(&[("name", r#""Warehouse""#)]), &taken).unwrap();
    assert_eq!(imported.name, "Warehouse 3");
}

#[test]
fn a_name_long_enough_to_be_a_payload_is_truncated() {
    let long = "x".repeat(4_000);
    assert_eq!(unique_name(&long, &[]).chars().count(), MAX_NAME_CHARS);
}

/// `load` runs this over everything it reads, so a hand-edited templates.json
/// cannot carry a setting no control in the app could produce.
#[test]
fn clamping_a_stored_template_keeps_its_identity() {
    let stored = Template {
        id: "mine".to_string(),
        name: "Warehouse".to_string(),
        is_stock: false,
        headroom_db: -12.0,
        target_lufs: 0.0,
        ..templates::festival()
    };
    let safe = clamped(stored);

    assert_eq!(safe.id, "mine");
    assert_eq!(safe.name, "Warehouse");
    assert_eq!(safe.headroom_db, 0.0);
    assert_eq!(safe.target_lufs, -6.0);
}

#[test]
fn a_stored_template_that_cannot_be_repaired_keeps_its_name_and_takes_the_defaults() {
    let stored = Template {
        id: "mine".to_string(),
        name: "Broken".to_string(),
        is_stock: false,
        bands: Vec::new(),
        ..templates::festival()
    };
    let safe = clamped(stored);

    assert_eq!(safe.name, "Broken");
    assert!(!safe.is_stock);
    assert_eq!(safe.bands.len(), 4);
}
