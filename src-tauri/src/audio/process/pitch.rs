//! Pitch correction, for vinyl rips and older recordings that sit off concert
//! pitch for the whole file.

use rubato::audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use signalsmith_stretch::Stretch;

use crate::audio::DecodedAudio;
use crate::error::{CapstoneError, Result};
use crate::models::PitchMode;

/// Half a semitone. Tuning is measured by folding partials onto the
/// equal-tempered grid, and that fold is ambiguous beyond this point: a track
/// 60 cents sharp is indistinguishable from one 40 cents flat. Correcting in
/// the shorter direction is also the musically right answer, because a 60 cent
/// error is a different key.
const MAX_CENTS: f64 = 50.0;

/// Below this there is nothing to do.
const MIN_CENTS: f64 = 0.01;

/// Corrects the buffer by `-cents` and reports how much it moved.
///
/// Two modes, and they are genuinely different operations:
///
/// - `KeepTempo` shifts pitch and leaves the running time alone, through
///   signalsmith-stretch. Use its `exact()` call, which preserves length to the
///   sample. Do not hand-roll a phase vocoder: identity phase locking and
///   transient phase reset are mandatory rather than optional, and without both
///   a kick turns into a thud. This library has them already.
/// - `ChangeTempo` resamples, the way nudging a turntable does, through rubato.
///   Pitch and tempo move together and the file gets longer or shorter - by up
///   to 2.9% at 50 cents, which is ten seconds on a six-minute track. A DJ with
///   saved cue points will care, so the UI has to say so, and any BPM metadata
///   written has to be the corrected figure.
///
/// Note rubato 5.0 is a breaking redesign around the audioadapter traits, so
/// 4.x examples will not compile, and the import is
/// `rubato::audioadapter_buffers::direct` rather than the
/// `audioadapter::interleaved` that reads naturally. `process_all()` resets,
/// runs the clip and trims the resampler's own delay in one call, which is
/// exactly this offline case.
///
/// The figure returned is the shift applied, signed, so a file measured 14
/// cents sharp reports -14.
///
/// The caller only reaches this when the measured deviation exceeds
/// `pitch_tolerance_cents`, so a correction of less than that is never
/// attempted: nudging a file that was already in tune is worse than leaving it.
pub fn correct(audio: &mut DecodedAudio, cents: f64, mode: PitchMode) -> Result<f64> {
    if audio.is_empty() || !cents.is_finite() {
        return Ok(0.0);
    }
    let shift = -cents.clamp(-MAX_CENTS, MAX_CENTS);
    if shift.abs() < MIN_CENTS {
        return Ok(0.0);
    }
    match mode {
        PitchMode::KeepTempo => keep_tempo(audio, shift),
        PitchMode::ChangeTempo => change_tempo(audio, shift),
    }
}

/// Shifts pitch and puts the running time back, in two exact steps.
///
/// Not `set_transpose_factor_semitones`. That is the obvious call and it is
/// what this used to do, but the factor it actually applies is not the factor
/// asked for, and not off by a constant either. Measured on a 440 Hz tone,
/// with two estimators that share no code - a Hann-windowed FFT peak refined
/// parabolically, and the mean spacing of interpolated zero crossings, which
/// agreed to within a tenth of a cent:
///
/// ```text
///   asked   -10    -20    -30    -40    -50  cents
///   left  +4.08  +0.60  +1.41  +3.77  -0.04  cents of the detune still there
/// ```
///
/// Four cents left of a ten-cent correction is a 40% error, and the whole
/// point of correcting a vinyl rip is that it then plays in tune against
/// something else.
///
/// Resampling, by contrast, lands on the ratio exactly - `change_tempo`
/// measures -0.03 cents at every one of those shifts, which is the
/// measurement's own bias and not an error at all. So the pitch is moved by
/// resampling and the duration that costs is then stretched back out, with the
/// transpose factor left alone. Composed, the residual is the same -0.03 at
/// every shift.
///
/// This is what a phase vocoder pitch shifter does internally anyway. Doing it
/// here only means the resampling step is rubato's, at a ratio it honours.
fn keep_tempo(audio: &mut DecodedAudio, shift: f64) -> Result<f64> {
    let frames = audio.frame_count();
    let channels = audio.channels.len().max(1) as u32;
    let rate = audio.sample_rate;

    let mut stretch = Stretch::preset_default(channels, rate);
    // `exact` needs twice its own input latency before it will do anything,
    // measured at 8, 44.1 and 96 kHz in mono and stereo. Asked here rather
    // than discovered from a `false` return, because by the time the stretch
    // runs the buffer has already been resampled and the original length is
    // gone. Three times, not two, because the resample moves the length by up
    // to 2.9% first - and the margin costs nothing, since a buffer this short
    // never reaches the chain's pitch branch at all: the tuning estimator
    // wants two and a half seconds before it will report any deviation.
    if frames < 3 * stretch.input_latency() {
        // Leaving the file at the pitch it arrived at beats refusing to
        // process it: the row reports no correction and every other stage
        // still runs.
        tracing::warn!(
            frames,
            "pitch shift declined this buffer, leaving it as found"
        );
        return Ok(0.0);
    }

    change_tempo(audio, shift)?;

    let interleaved = audio.to_interleaved();
    // The planar copy is released before the output buffer is allocated. A
    // six-minute stereo track is 60 MB of f32 and several of these workers run
    // at once, so holding the resampled buffer, its interleaved copy and the
    // stretched output all at once is 180 MB that nothing needs.
    *audio = DecodedAudio::new(Vec::new(), rate);
    let mut stretched = vec![0.0f32; frames * channels as usize];
    if !stretch.exact(&interleaved, &mut stretched) {
        // The length check above should have caught this. If it ever does
        // happen the buffer is already resampled, so its running time is
        // wrong by up to 2.9%, and trimming it back would cut ten seconds off
        // a six-minute track. A row that says it failed beats a file that is
        // quietly the wrong length.
        return Err(CapstoneError::Other(format!(
            "pitch shift could not hold the length of a {frames} frame buffer"
        )));
    }
    *audio = DecodedAudio::from_interleaved(&stretched, channels as u16, rate);
    Ok(shift)
}

fn change_tempo(audio: &mut DecodedAudio, shift: f64) -> Result<f64> {
    let channels = audio.channels.len();
    let frames = audio.frame_count();
    let rate = audio.sample_rate;
    // The pitch multiplier wanted. Reading the input with step `p` lowers pitch
    // by `p` and lengthens the file by `1/p`, so the resampling ratio - output
    // frames per input frame - is the reciprocal. The output sample rate does
    // not change; only the content stretches.
    let multiplier = 2f64.powf(shift / 1200.0);
    let parameters = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: Some(0.95),
        oversampling_factor: 256,
        interpolation: SincInterpolationType::Cubic,
        window: WindowFunction::BlackmanHarris2,
    };
    let mut resampler = Async::<f32>::new_sinc(
        1.0 / multiplier,
        1.1,
        &parameters,
        1_024,
        channels,
        FixedAsync::Input,
    )
    .map_err(|e| CapstoneError::Other(format!("could not build resampler: {e}")))?;
    let resampled = {
        let adapter = SequentialSliceOfVecs::new(&audio.channels, channels, frames)
            .map_err(|e| CapstoneError::Other(format!("resampler input: {e}")))?;
        resampler
            .process_all(&adapter, frames, None)
            .map_err(|e| CapstoneError::Other(format!("could not resample: {e}")))?
    };
    *audio = DecodedAudio::from_interleaved(&resampled.take_data(), channels as u16, rate);
    Ok(shift)
}

#[cfg(test)]
mod tests {
    use super::super::testkit;
    use super::*;

    /// The largest correction the fold allows, which is also the worst case for
    /// both modes.
    const QUARTER_TONE: f64 = 50.0;

    /// Half a semitone down lands on 427.49 Hz from 440, and the file gets 2.9%
    /// longer, because that is what a turntable does. Ten seconds on a
    /// six-minute track, which is why the UI has to say the duration changes.
    #[test]
    fn change_tempo_moves_pitch_and_length_together() {
        let fs = 44_100.0;
        let frames = 44_100;
        let mut audio = DecodedAudio::new(vec![testkit::sine(440.0, fs, frames, 0.5)], 44_100);
        let applied = correct(&mut audio, QUARTER_TONE, PitchMode::ChangeTempo).expect("resampled");
        assert!((applied + QUARTER_TONE).abs() < 1e-9, "reported {applied}");
        let measured = testkit::peak_frequency(&audio.channels[0], fs);
        assert!(
            (measured - 427.49).abs() < 1.0,
            "landed on {measured:.2} Hz"
        );
        let ratio = audio.frame_count() as f64 / frames as f64;
        assert!((ratio - 1.0293).abs() < 0.005, "length ratio {ratio:.4}");
    }

    /// The same shift with the tempo held: pitch moves, length does not.
    #[test]
    fn keep_tempo_moves_pitch_only() {
        let fs = 44_100.0;
        let frames = 44_100;
        let mut audio = DecodedAudio::new(vec![testkit::sine(440.0, fs, frames, 0.5)], 44_100);
        let applied = correct(&mut audio, QUARTER_TONE, PitchMode::KeepTempo).expect("shifted");
        assert!((applied + QUARTER_TONE).abs() < 1e-9, "reported {applied}");
        assert_eq!(audio.frame_count(), frames, "length changed");
        let measured = testkit::peak_frequency(&audio.channels[0], fs);
        assert!(
            (measured - 427.49).abs() < 2.0,
            "landed on {measured:.2} Hz"
        );
    }

    /// Both modes have to survive a real stereo buffer without collapsing or
    /// swapping the channels.
    #[test]
    fn stereo_stays_stereo() {
        for mode in [PitchMode::KeepTempo, PitchMode::ChangeTempo] {
            let fs = 44_100.0;
            let left = testkit::sine(440.0, fs, 44_100, 0.4);
            let right = testkit::sine(1_760.0, fs, 44_100, 0.4);
            let mut audio = DecodedAudio::new(vec![left, right], 44_100);
            correct(&mut audio, 30.0, mode).expect("corrected");
            assert_eq!(audio.channels.len(), 2);
            let low = testkit::peak_frequency(&audio.channels[0], fs);
            let high = testkit::peak_frequency(&audio.channels[1], fs);
            assert!(
                low < high / 2.0,
                "{mode:?}: {low:.1} Hz against {high:.1} Hz"
            );
        }
    }

    /// Beyond half a semitone the fold is ambiguous, so the correction is bound
    /// rather than extrapolated.
    #[test]
    fn clamps_to_half_a_semitone() {
        let mut audio =
            DecodedAudio::new(vec![testkit::sine(440.0, 44_100.0, 44_100, 0.5)], 44_100);
        let applied = correct(&mut audio, 400.0, PitchMode::ChangeTempo).expect("corrected");
        assert!((applied + 50.0).abs() < 1e-9, "reported {applied}");
    }

    #[test]
    fn a_file_already_in_tune_is_untouched() {
        let original = testkit::sine(440.0, 44_100.0, 4_096, 0.5);
        let mut audio = DecodedAudio::new(vec![original.clone()], 44_100);
        assert_eq!(
            correct(&mut audio, 0.0, PitchMode::KeepTempo).expect("ok"),
            0.0
        );
        assert_eq!(audio.channels[0], original);
    }

    #[test]
    fn odd_input_does_not_panic() {
        for mode in [PitchMode::KeepTempo, PitchMode::ChangeTempo] {
            let cases = vec![
                DecodedAudio::new(vec![vec![]], 44_100),
                DecodedAudio::new(vec![vec![0.0; 100]], 44_100),
                DecodedAudio::new(vec![vec![0.5; 4_096]], 96_000),
                DecodedAudio::new(vec![vec![0.5; 2_048], vec![-0.5; 2_048]], 8_000),
                DecodedAudio::new(Vec::new(), 44_100),
            ];
            for mut audio in cases {
                let applied = correct(&mut audio, 25.0, mode).expect("no hard failure");
                assert!(applied.is_finite());
                assert!(audio.channels.iter().flatten().all(|s| s.is_finite()));
            }
        }
    }
}
