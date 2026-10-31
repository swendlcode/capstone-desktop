//! Sample-rate conversion on the way out.
//!
//! This is a sample-rate change only, which is not the same job as the pitch
//! path in `audio::process::pitch`: that one resamples to *move* the pitch and
//! the running time. Here the content must come out sounding identical and only
//! the rate changes, so the ratio is whatever the output format needs and
//! nothing about the audio is meant to be audible as different.
//!
//! Reached in two cases: an output rate that differs from the source's, and an
//! MP3 target whose rate is not one of the nine MPEG Layer III rates.

use rubato::audioadapter_buffers::direct::InterleavedSlice;
use rubato::{
    Async, FixedAsync, Resampler, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

use crate::audio::DecodedAudio;
use crate::error::{CapstoneError, Result};

/// Converts to `target_rate`, or hands the audio straight back when it is
/// already there. The caller gets an owned buffer either way, so the no-op case
/// costs one clone rather than a branch at every call site.
pub fn to_rate(audio: &DecodedAudio, target_rate: u32) -> Result<DecodedAudio> {
    if audio.sample_rate == target_rate || audio.is_empty() {
        return Ok(audio.clone());
    }
    if target_rate == 0 || audio.sample_rate == 0 {
        return Err(CapstoneError::Unsupported(
            "cannot resample to or from a zero sample rate".to_string(),
        ));
    }

    let channels = audio.channels.len();
    let frames = audio.frame_count();
    let ratio = f64::from(target_rate) / f64::from(audio.sample_rate);

    // Offline batch quality, not real-time: a 256-tap sinc with 256x
    // oversampled interpolation is far more than a 44.1 to 48 conversion needs
    // and costs milliseconds on a track.
    let parameters = SincInterpolationParameters {
        sinc_len: 256,
        f_cutoff: None,
        oversampling_factor: 256,
        interpolation: SincInterpolationType::Cubic,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = Async::<f32>::new_sinc(
        ratio,
        // No ratio changes mid-clip, so the relative headroom is a formality.
        1.1,
        &parameters,
        1024,
        channels,
        FixedAsync::Input,
    )
    .map_err(|e| CapstoneError::Encode(format!("resampler: {}", e)))?;

    let interleaved = audio.to_interleaved();
    let input = InterleavedSlice::new(&interleaved[..], channels, frames)
        .map_err(|e| CapstoneError::Encode(format!("resampler input: {}", e)))?;

    // `process_all` resets, runs the whole clip and trims the resampler's own
    // delay, which is the reason to prefer it here: doing those three by hand
    // is where an off-by-latency bug comes from, and a shifted output would
    // move every clip position the earlier stages measured.
    let output = resampler
        .process_all(&input, frames, None)
        .map_err(|e| CapstoneError::Encode(format!("resampling failed: {}", e)))?;

    Ok(DecodedAudio::from_interleaved(
        &output.take_data(),
        channels as u16,
        target_rate,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(rate: u32, hz: f64, secs: f64) -> DecodedAudio {
        let frames = (f64::from(rate) * secs) as usize;
        let channel: Vec<f32> = (0..frames)
            .map(|i| {
                let t = i as f64 / f64::from(rate);
                (0.5 * (std::f64::consts::TAU * hz * t).sin()) as f32
            })
            .collect();
        DecodedAudio::new(vec![channel.clone(), channel], rate)
    }

    #[test]
    fn same_rate_is_untouched() {
        let audio = sine(44_100, 1_000.0, 0.1);
        let out = to_rate(&audio, 44_100).unwrap();
        assert_eq!(out.channels, audio.channels);
    }

    #[test]
    fn frame_count_follows_the_ratio() {
        let audio = sine(44_100, 1_000.0, 0.5);
        let out = to_rate(&audio, 48_000).unwrap();
        assert_eq!(out.sample_rate, 48_000);
        assert_eq!(out.channel_count(), 2);
        let expected = 24_000f64;
        let drift = (out.frame_count() as f64 - expected).abs();
        assert!(drift < expected * 0.01, "got {} frames", out.frame_count());
    }

    #[test]
    fn level_survives_the_conversion() {
        // A resample that is working leaves a steady sine at its own amplitude.
        // Anything materially louder means the interpolation is ringing.
        let audio = sine(48_000, 1_000.0, 0.3);
        let out = to_rate(&audio, 44_100).unwrap();
        assert!((out.peak() - 0.5).abs() < 0.02, "peak {}", out.peak());
    }
}
