//! All DSP lives under here. Nothing in `commands` or `jobs` reaches into a
//! sample buffer; they hand a `DecodedAudio` to one of these functions.

pub mod analyze;
pub mod decode;
pub mod encode;
pub mod process;

/// Deinterleaved audio: one Vec per channel, every one the same length.
///
/// Planar is the shape the processing chain wants, since a per-channel filter
/// is then a straight slice walk with no stride arithmetic, and it is the shape
/// most of the encoders want too. Only ebur128 and signalsmith-stretch need
/// interleaved, which is why the conversions live here instead of being
/// open-coded at each of those two boundaries.
#[derive(Debug, Clone)]
pub struct DecodedAudio {
    pub channels: Vec<Vec<f32>>,
    pub sample_rate: u32,
}

impl DecodedAudio {
    pub fn new(channels: Vec<Vec<f32>>, sample_rate: u32) -> Self {
        Self {
            channels,
            sample_rate,
        }
    }

    pub fn channel_count(&self) -> u16 {
        self.channels.len() as u16
    }

    /// Frames, not samples. Takes the shortest channel so a decoder that
    /// delivered a ragged last packet cannot walk a caller off the end of a
    /// shorter channel.
    pub fn frame_count(&self) -> usize {
        self.channels.iter().map(|c| c.len()).min().unwrap_or(0)
    }

    pub fn duration_sec(&self) -> f64 {
        if self.sample_rate == 0 {
            return 0.0;
        }
        self.frame_count() as f64 / self.sample_rate as f64
    }

    pub fn is_empty(&self) -> bool {
        self.frame_count() == 0
    }

    pub fn to_interleaved(&self) -> Vec<f32> {
        let frames = self.frame_count();
        let channel_count = self.channels.len();
        let mut out = Vec::with_capacity(frames * channel_count);
        for frame in 0..frames {
            for channel in &self.channels {
                out.push(channel[frame]);
            }
        }
        out
    }

    pub fn from_interleaved(samples: &[f32], channel_count: u16, sample_rate: u32) -> Self {
        let channel_count = channel_count.max(1) as usize;
        let frames = samples.len() / channel_count;
        let mut channels = vec![Vec::with_capacity(frames); channel_count];
        for frame in 0..frames {
            for (index, channel) in channels.iter_mut().enumerate() {
                channel.push(samples[frame * channel_count + index]);
            }
        }
        Self::new(channels, sample_rate)
    }

    /// Largest absolute sample, linear. The honest peak of the buffer, which is
    /// not the same as a true peak: `analyze` asks ebur128 for that, because an
    /// inter-sample peak does not exist in these samples at all.
    pub fn peak(&self) -> f32 {
        self.channels
            .iter()
            .flat_map(|c| c.iter())
            .fold(0.0f32, |acc, s| acc.max(s.abs()))
    }

    pub fn apply_gain(&mut self, linear: f32) {
        for channel in self.channels.iter_mut() {
            for sample in channel.iter_mut() {
                *sample *= linear;
            }
        }
    }
}

pub fn db_to_linear(db: f64) -> f64 {
    10f64.powf(db / 20.0)
}

pub fn linear_to_db(linear: f64) -> f64 {
    if linear <= 0.0 {
        // Silence has no decibel value. -200 is far below anything audible and
        // keeps callers from having to special-case an infinity in a report.
        return -200.0;
    }
    20.0 * linear.log10()
}
