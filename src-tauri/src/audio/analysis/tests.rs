//! Every measure in `analyze` is checked against a signal whose answer is known
//! from arithmetic rather than from a previous run of this code.

use std::f64::consts::TAU;

use super::*;

const RATE: u32 = 44_100;

fn mono(samples: Vec<f32>) -> DecodedAudio {
    DecodedAudio::new(vec![samples], RATE)
}

fn sine(frequency: f64, amplitude: f64, seconds: f64) -> Vec<f32> {
    let count = (seconds * RATE as f64) as usize;
    (0..count)
        .map(|n| (amplitude * (TAU * frequency * n as f64 / RATE as f64).sin()) as f32)
        .collect()
}

/// A 1 kHz sine whose RMS is -20 dBFS has to read -20 LUFS: K-weighting is
/// calibrated so that at 1 kHz the two scales coincide (+0.01 dB). Mono on
/// purpose -- the same sine in two channels reads 3 dB louder, because the
/// measure sums the channel energies, and that surprise belongs in a test of
/// its own rather than hidden inside this one.
#[test]
fn integrated_loudness_of_a_minus_twenty_lufs_sine() {
    let amplitude = 2f64.sqrt() / 10.0; // RMS = amplitude / sqrt(2) = 0.1
    let audio = mono(sine(1000.0, amplitude, 5.0));

    let (lufs, _) = loudness(&audio).unwrap();

    assert!(
        (lufs + 20.0).abs() < 0.5,
        "expected about -20 LUFS, got {lufs}"
    );
}

/// Summing channel energies is the standard's behaviour, not a bug, and the
/// gain stage depends on it: the same material in two channels is 3.01 dB
/// louder than in one.
#[test]
fn a_second_identical_channel_adds_three_db() {
    let samples = sine(1000.0, 0.5, 5.0);
    let one = mono(samples.clone());
    let two = DecodedAudio::new(vec![samples.clone(), samples], RATE);

    let (mono_lufs, _) = loudness(&one).unwrap();
    let (stereo_lufs, _) = loudness(&two).unwrap();

    assert!(
        (stereo_lufs - mono_lufs - 3.01).abs() < 0.1,
        "mono {mono_lufs}, stereo {stereo_lufs}"
    );
}

/// A full-scale sine at a quarter of the sample rate, phased so no sample
/// lands on a peak: the samples only reach 0.707 (-3.01 dBFS) while the
/// waveform they describe reaches full scale. A sample-peak meter reports the
/// -3 dB; a true-peak meter has to find what is between the samples, because
/// that is what a DAC's reconstruction filter and any lossy transcode will
/// produce.
#[test]
fn true_peak_sees_between_the_samples() {
    let count = 2 * RATE as usize;
    let samples: Vec<f32> = (0..count)
        .map(|n| (TAU * (n as f64 + 0.5) / 4.0).sin() as f32)
        .collect();
    let audio = mono(samples);

    let sample_peak_db = linear_to_db(audio.peak() as f64);
    let (_, true_peak_db) = loudness(&audio).unwrap();

    assert!(
        (sample_peak_db + 3.01).abs() < 0.01,
        "sample peak should be -3.01 dBFS, got {sample_peak_db}"
    );
    assert!(
        true_peak_db > sample_peak_db + 2.0 && true_peak_db < 0.5,
        "true peak should sit near 0 dBTP, got {true_peak_db}"
    );
}

/// A 1 kHz sine driven 3.5 dB into a hard ceiling flattens both halves of every
/// cycle, so one second of it is 1000 cycles and 2000 clipped runs. The count is
/// runs, not samples: each of these runs is about 12 samples long.
#[test]
fn clipping_is_counted_in_runs_not_samples() {
    let clipped: Vec<f32> = sine(1000.0, 1.5, 1.0)
        .into_iter()
        .map(|s| s.clamp(-1.0, 1.0))
        .collect();

    let runs = clip_count(&mono(clipped));

    assert!(
        (1990..=2000).contains(&runs),
        "expected ~2000 runs, got {runs}"
    );
}

/// The other half of the same test, and the one that matters more: a loud but
/// unclipped master must read zero. A waveform spends real time near full scale
/// without ever being flattened there.
#[test]
fn an_unclipped_sine_has_no_clips() {
    assert_eq!(clip_count(&mono(sine(1000.0, 0.99, 1.0))), 0);
    assert_eq!(clip_count(&mono(sine(60.0, 1.0, 1.0))), 0);
}

/// Peak over RMS for any sine is sqrt(2), which is 3.01 dB. This is the floor
/// of the Dynamics column: nothing musical is flatter than a sine wave.
#[test]
fn crest_of_a_sine_is_three_db() {
    let crest = crest_db(&mono(sine(1000.0, 0.8, 2.0)));
    assert!((crest - 3.01).abs() < 0.05, "expected 3.01 dB, got {crest}");
}

/// A note 30 cents sharp of A440, with a harmonic on top so the estimator has
/// more than one partial to work from. The harmonic is an exact octave, so it
/// is 30 cents sharp of its own semitone too and votes the same way.
#[test]
fn tuning_finds_thirty_cents_sharp() {
    let fundamental = 440.0 * 2f64.powf(30.0 / 1200.0);
    let mut samples = sine(fundamental, 0.5, 5.0);
    for (slot, harmonic) in samples.iter_mut().zip(sine(fundamental * 2.0, 0.12, 5.0)) {
        *slot += harmonic;
    }

    let cents = tuning_cents(&mono(samples));

    assert!(
        (cents - 30.0).abs() < 5.0,
        "expected about +30 cents, got {cents}"
    );
}

/// Flat the other way, to catch a sign error that a single-sided test would not.
#[test]
fn tuning_reports_flat_as_negative() {
    let detuned = 440.0 * 2f64.powf(-25.0 / 1200.0);
    let cents = tuning_cents(&mono(sine(detuned, 0.5, 5.0)));
    assert!(
        (cents + 25.0).abs() < 5.0,
        "expected about -25 cents, got {cents}"
    );
}

/// Noise is the case the feature has to decline rather than answer: every frame
/// is spectrally flat, nothing is a partial, and the row shows no pitch at all.
#[test]
fn tuning_declines_on_percussive_material() {
    let mut state = 0x2545_f491_4f6c_dd1du64;
    let samples: Vec<f32> = (0..5 * RATE as usize)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            ((state >> 40) as f32 / 8_388_608.0) - 0.5
        })
        .collect();

    assert_eq!(tuning_cents(&mono(samples)), 0.0);
}

/// Too short to average over. 50 ms is one analysis window, which is not enough
/// frames to tell a mastering drift from one note.
#[test]
fn tuning_declines_on_a_short_file() {
    assert_eq!(tuning_cents(&mono(sine(447.0, 0.5, 0.05))), 0.0);
}

/// A file under one 400 ms gating block has no gated loudness at all, and the
/// meter answers -inf. The RMS fallback has to put a usable number in the
/// Volume column instead.
#[test]
fn a_file_too_short_to_gate_still_reports_a_level() {
    let audio = mono(sine(1000.0, 2f64.sqrt() / 10.0, 0.2));
    let analysis = analyze(&audio).unwrap();

    assert!(
        (analysis.lufs + 20.0).abs() < 1.0,
        "expected about -20, got {}",
        analysis.lufs
    );
}

/// The whole struct, once, so a field wired to the wrong measure shows up here
/// rather than in the table.
#[test]
fn analyze_fills_every_column() {
    let audio = DecodedAudio::new(vec![sine(1000.0, 0.5, 2.0), sine(1000.0, 0.5, 2.0)], RATE);
    let analysis = analyze(&audio).unwrap();

    assert_eq!(analysis.sample_rate, RATE);
    assert_eq!(analysis.channels, 2);
    assert!((analysis.duration_sec - 2.0).abs() < 1e-6);
    assert_eq!(analysis.clip_count, 0);
    assert!((analysis.crest_db - 3.01).abs() < 0.05);
    assert!(analysis.true_peak_db < 0.0 && analysis.true_peak_db > -7.0);
}

#[test]
fn an_empty_buffer_is_an_error_not_a_row_of_zeroes() {
    assert!(analyze(&DecodedAudio::new(vec![vec![]], RATE)).is_err());
}
