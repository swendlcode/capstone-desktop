//! Claim 4: no output sample and no inter-sample peak exceeds the ceiling.
//!
//! Both halves matter and they are different measurements. A sample-peak check
//! passes on a file that overshoots 0.8 dB between two samples, which becomes
//! audible distortion on the next lossy transcode or on a cheap DAC's
//! reconstruction filter. The inter-sample figure comes from `ebur128`'s
//! true-peak meter, which is conformance tested and is not the detector the
//! limiter uses for its own sidechain.

use capstone_lib::audio::DecodedAudio;
use capstone_lib::models::{OutputFormat, Template};
use capstone_lib::templates;

use crate::support::{self, fixtures, measure};

/// 16-bit output is dithered, which can push a sample one LSB - about
/// 0.0003 dB at this level - past where the chain left it. Anything beyond
/// this is the limiter, not the dither.
const DITHER_SLACK_DB: f64 = 0.05;

/// The inter-sample figure is reconstructed by a 4x polyphase filter, so it
/// carries a little more measurement noise than the sample peak does.
const TRUE_PEAK_SLACK_DB: f64 = 0.1;

/// Every material profile the limiter has to behave on: a quiet wide-dynamic
/// source needing a lot of gain, ordinary programme, something already at the
/// rail, and something clipped past it.
fn material() -> Vec<(&'static str, DecodedAudio)> {
    let programme = fixtures::programme(12.0, 44_100, -20.0);
    vec![
        ("quiet programme", fixtures::programme(12.0, 44_100, -34.0)),
        ("programme", programme.clone()),
        (
            "limited master",
            fixtures::limited_master(12.0, 44_100, -8.0),
        ),
        ("clipped", fixtures::clipped(&programme, 14.0)),
    ]
}

#[test]
fn nothing_leaves_above_the_ceiling() {
    // Every headroom the character dial is likely to be paired with, and the
    // extremes of the dial itself: 0 is the slowest and most transparent
    // setting, 10 the fastest and most obvious, and they take different paths
    // through the release stage.
    for headroom_db in [0.3f64, 1.5, 3.0] {
        for character in [0u8, 4, 10] {
            let template = Template {
                headroom_db,
                limiter_character: character,
                ..templates::festival()
            };
            let ceiling = support::ceiling_db(&template);

            for (name, signal) in material() {
                let dir = fixtures::scratch("limiter");
                let (path, _) = fixtures::materialise(signal, &dir, "source.wav");
                let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
                let run = support::pipeline(&path, &template, &settings);

                let label = format!("{name}, headroom {headroom_db} dB, character {character}");

                // The chain's own output first, before any encoder could be
                // blamed for it.
                let chain_sample = measure::sample_peak_dbfs(&run.processed);
                assert!(
                    chain_sample <= ceiling + 1e-3,
                    "{label}: the chain left a sample at {chain_sample:.4} dBFS \
                     over a {ceiling:.1} dB ceiling"
                );
                let chain_true = measure::true_peak_dbtp(&run.processed);
                assert!(
                    chain_true <= ceiling + TRUE_PEAK_SLACK_DB,
                    "{label}: the chain left an inter-sample peak at \
                     {chain_true:.4} dBTP over a {ceiling:.1} dB ceiling"
                );

                // And then the file as it actually lands on disk.
                let written_sample = measure::sample_peak_dbfs(&run.decoded);
                assert!(
                    written_sample <= ceiling + DITHER_SLACK_DB,
                    "{label}: the written file has a sample at \
                     {written_sample:.4} dBFS over a {ceiling:.1} dB ceiling"
                );
                let written_true = measure::true_peak_dbtp(&run.decoded);
                assert!(
                    written_true <= ceiling + TRUE_PEAK_SLACK_DB,
                    "{label}: the written file peaks at {written_true:.4} dBTP \
                     over a {ceiling:.1} dB ceiling"
                );
            }
        }
    }
}

/// The ceiling has to be a ceiling, not a target, and the target has to be hit
/// whether the limiter engages or not.
///
/// Dense material never reaches the ceiling after its gain move - at a
/// -12.5 LUFS target and a -1.5 dB ceiling the limiter only engages past 11 dB
/// of peak-to-loudness, and a modern master sits at six to nine - so it must
/// land on target to the measurement's own precision. Transient-heavy material
/// does engage the limiter, which removes loudness as well as peaks, and has to
/// land on target anyway.
///
/// Both halves matter because the second one is where the bug was: a single
/// gain move followed by a limiter lands under the target by however much the
/// limiter took, and 0.84 LU of that is well outside what the product claims.
#[test]
fn both_idle_and_working_land_on_the_target() {
    let template = templates::festival();
    let cases: Vec<(&str, DecodedAudio, f64)> = vec![
        // Dense: the limiter stays idle, so the gain move is the whole story.
        ("dense", fixtures::limited_master(12.0, 44_100, -16.0), 0.1),
        // Transients and a lot of gain to give: the limiter engages hard.
        ("transient", fixtures::programme(12.0, 44_100, -34.0), 0.5),
    ];

    for (name, signal, tolerance) in cases {
        let dir = fixtures::scratch("limiter-target");
        let (path, _) = fixtures::materialise(signal, &dir, "source.wav");
        let settings = support::settings(OutputFormat::Wav, &dir.join("out"));
        let run = support::pipeline(&path, &template, &settings);

        let after = measure::integrated_lufs(&run.decoded);
        assert!(
            (after - template.target_lufs).abs() <= tolerance,
            "{name}: landed on {after:.3} LUFS against a {:.1} target, \
             outside {tolerance} LU",
            template.target_lufs
        );
    }
}
