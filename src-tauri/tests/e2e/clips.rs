//! Claim 2: the clip count on a hard-clipped file drops to zero, the repaired
//! file's true peak stays under the template ceiling, and the regions that
//! were flat are not flat any more.
//!
//! The last part is the one that matters. A repair that only turned the file
//! down would satisfy a count-only test, so the position of every flat top is
//! recorded before the repair and each one is re-measured afterwards.

use capstone_lib::audio::{analyze, process};
use capstone_lib::models::{OutputFormat, Template};
use capstone_lib::templates;

use crate::support::{self, fixtures, measure};

/// 110 Hz for 10 s is 1,100 cycles, so 2,200 flat tops per channel: a number
/// that comes out of arithmetic rather than out of a detector.
const CLIP_HZ: f64 = 110.0;
const CLIP_SECONDS: f64 = 10.0;
const OVERDRIVE_DB: f64 = 1.5;

#[test]
fn a_known_number_of_flat_tops_is_found_and_cleared() {
    let dir = fixtures::scratch("clips-known");
    let (signal, expected) = fixtures::clipped_sine(CLIP_HZ, CLIP_SECONDS, 44_100, OVERDRIVE_DB, 2);
    let (path, before) = fixtures::materialise(signal, &dir, "clipped-sine.wav");

    let channels = before.channels.len() as u64;
    let independent = measure::clip_runs(&before);
    assert_eq!(
        independent.len() as u64,
        expected * channels,
        "independent detector found {} runs, arithmetic says {}",
        independent.len(),
        expected * channels
    );
    assert_eq!(
        analyze::clip_count(&before),
        expected * channels,
        "the Clips column disagrees with arithmetic"
    );

    // Repair on its own, because the full chain ends with a limiter that pulls
    // everything well under the clip ceiling: a zero after that would prove
    // nothing about the repair.
    let template = templates::festival();
    let mut repaired = before.clone();
    let fixed = process::clip_repair::repair(&mut repaired, &template);
    assert_eq!(
        fixed,
        expected * channels,
        "the repair acted on {fixed} of {} flat tops",
        expected * channels
    );
    assert_eq!(
        measure::clip_runs(&repaired).len(),
        0,
        "flat tops survived the repair"
    );
    assert_eq!(analyze::clip_count(&repaired), 0);

    // Each former flat top now carries a curve rather than a plateau.
    let mut still_flat = 0;
    for run in &independent {
        if measure::spread(&repaired, run.channel, run.start, run.end) <= measure::FLATNESS_LIMIT {
            still_flat += 1;
        }
    }
    assert_eq!(
        still_flat, 0,
        "{still_flat} repaired regions are still flat"
    );

    // The reconstruction is meant to reach back above full scale. That is what
    // the template's headroom makes room for, and the recovered peak is what
    // says the curve put back is the right size: 1.5 dB in, 1.5 dB out.
    let recovered = measure::sample_peak_dbfs(&repaired);
    assert!(
        (recovered - OVERDRIVE_DB).abs() < 0.3,
        "recovered {recovered:.3} dBFS from a {OVERDRIVE_DB} dB clip"
    );

    // And the written file is under the ceiling once the chain has run.
    let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
    let run = support::pipeline(&path, &template, &settings);
    assert_eq!(run.result.clips_repaired, expected * channels);
    let ceiling = support::ceiling_db(&template);
    let written = measure::true_peak_dbtp(&run.decoded);
    assert!(
        written <= ceiling + 0.1,
        "the written file peaks at {written:.3} dBTP over a {ceiling:.1} dB ceiling"
    );
    assert_eq!(
        analyze::clip_count(&run.decoded),
        0,
        "the written file still reports clipping"
    );
}

/// The same claim on real programme material, where the flat tops are whatever
/// shape the mix happens to produce rather than a sine's.
#[test]
fn a_clipped_programme_file_comes_back_clean_and_under_the_ceiling() {
    let dir = fixtures::scratch("clips-programme");
    let source = fixtures::programme(30.0, 44_100, -20.0);
    // +14 dB on a -20 LUFS source puts it at -6 LUFS, comfortably over the
    // template's -14 repair gate, and well into the rail.
    let (path, before) = fixtures::materialise(
        fixtures::clipped(&source, 14.0),
        &dir,
        "clipped-programme.wav",
    );

    let runs_before = measure::clip_runs(&before);
    assert!(
        runs_before.len() > 100,
        "only {} flat tops: the fixture is not really clipped",
        runs_before.len()
    );
    assert!(analyze::clip_count(&before) > 0);

    let template = templates::festival();
    let mut repaired = before.clone();
    let fixed = process::clip_repair::repair(&mut repaired, &template);
    assert!(fixed > 0, "the repair found nothing to do");

    let left_over = measure::clip_runs(&repaired);
    assert!(
        left_over.is_empty(),
        "{} of {} flat tops survived the repair",
        left_over.len(),
        runs_before.len()
    );
    assert_eq!(analyze::clip_count(&repaired), 0);

    let still_flat = runs_before
        .iter()
        .filter(|run| {
            measure::spread(&repaired, run.channel, run.start, run.end) <= measure::FLATNESS_LIMIT
        })
        .count();
    assert_eq!(
        still_flat,
        0,
        "{still_flat} of {} repaired regions are still flat",
        runs_before.len()
    );

    let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
    let run = support::pipeline(&path, &template, &settings);
    assert!(run.result.clips_repaired > 0);
    let ceiling = support::ceiling_db(&template);
    let written = measure::true_peak_dbtp(&run.decoded);
    assert!(
        written <= ceiling + 0.1,
        "the written file peaks at {written:.3} dBTP over a {ceiling:.1} dB ceiling"
    );
    assert_eq!(analyze::clip_count(&run.decoded), 0);
}

/// The gate the template puts on the repair: a file quieter than
/// `repair_above_lufs` was mastered with those flat tops and undoing them is
/// not Capstone's call.
///
/// Isolated by running one fixture down one pipeline twice, with templates that
/// differ in nothing but the gate. A fixture contrived to be quiet would test
/// the fixture; two templates over one file can only test the gate.
#[test]
fn the_repair_gate_is_the_only_thing_that_decides() {
    let dir = fixtures::scratch("clips-gate");
    let (signal, expected) = fixtures::clipped_sine(CLIP_HZ, 6.0, 44_100, OVERDRIVE_DB, 1);
    let (path, before) = fixtures::materialise(signal, &dir, "clipped.wav");

    let loudness = measure::integrated_lufs(&before);
    assert_eq!(analyze::clip_count(&before), expected);

    let open = Template {
        repair_above_lufs: loudness - 1.0,
        ..templates::festival()
    };
    let shut = Template {
        repair_above_lufs: loudness + 1.0,
        ..templates::festival()
    };

    let opened = support::pipeline(
        &path,
        &open,
        &support::settings(OutputFormat::Wav, &dir.join("a")),
    );
    assert_eq!(
        opened.result.clips_repaired, expected,
        "with the gate below the file's {loudness:.2} LUFS, every flat top should be repaired"
    );

    let kept = support::pipeline(
        &path,
        &shut,
        &support::settings(OutputFormat::Wav, &dir.join("b")),
    );
    assert_eq!(
        kept.result.clips_repaired, 0,
        "with the gate above the file's {loudness:.2} LUFS, nothing should be touched"
    );
}
