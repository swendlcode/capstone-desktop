//! Float to integer, with dither where it matters.
//!
//! Rounding to 16 bits without dither is an audible mistake, and it is audible
//! in exactly the material a DJ library is full of: a long reverb tail or a
//! quiet intro turns into quantisation distortion that tracks the signal, which
//! the ear hears as a gritty edge rather than as noise. TPDF dither trades that
//! for a steady, signal-independent noise floor about 4.8 dB above the
//! undithered one, which is the right trade every time.
//!
//! 24-bit output is not dithered. Its own noise floor sits near -144 dBFS, far
//! below the noise already on any real recording, so dither there would add
//! noise to solve nothing.

/// Peak full-scale value per word length. Deliberately the positive maximum
/// rather than 2^(n-1): scaling by 32768 and rounding lets a sample at exactly
/// -1.0 or +1.0 land one step outside the range, and a wrap there is a loud
/// click rather than a rounding error.
const I16_SCALE: f32 = 32_767.0;
const I24_SCALE: f32 = 8_388_607.0;

/// Interleaved f32 to interleaved i16, TPDF dithered.
pub fn to_i16(samples: &[f32]) -> Vec<i16> {
    let mut rng = Tpdf::new();
    samples
        .iter()
        .map(|s| {
            let scaled = s * I16_SCALE + rng.next();
            scaled.round().clamp(-32_768.0, 32_767.0) as i16
        })
        .collect()
}

/// Interleaved f32 to interleaved 24-bit values carried in i32, which is the
/// shape both hound and aifc want for a 24-bit write.
pub fn to_i24(samples: &[f32]) -> Vec<i32> {
    samples
        .iter()
        .map(|s| (s * I24_SCALE).round().clamp(-8_388_608.0, 8_388_607.0) as i32)
        .collect()
}

/// Triangular-PDF noise, one LSB either side of zero.
///
/// The generator is seeded with a constant, so the same input produces the same
/// output bytes every run. That is what makes the round-trip and idempotence
/// tests meaningful, and nobody can hear the difference between one dither
/// sequence and another.
struct Tpdf {
    state: u64,
}

impl Tpdf {
    fn new() -> Self {
        Self {
            state: 0x2545_F491_4F6C_DD1D,
        }
    }

    /// Two independent uniform draws, subtracted: the sum of two rectangular
    /// distributions is triangular, which is what decorrelates the error from
    /// the signal. One draw (RPDF) leaves the noise floor modulating with the
    /// programme, which is the fault dither exists to remove.
    fn next(&mut self) -> f32 {
        self.uniform() - self.uniform()
    }

    fn uniform(&mut self) -> f32 {
        // xorshift64*: three shifts and a multiply, with a period long enough
        // that a whole album never repeats it.
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        let scrambled = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (scrambled >> 40) as f32 / 16_777_216.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overs_clamp_instead_of_wrapping() {
        // A sample over full scale must pin to the rail. Wrapping turns the
        // loudest moment in the file into a click.
        assert_eq!(to_i16(&[4.0, -4.0]), vec![32_767, -32_768]);
    }

    #[test]
    fn dither_stays_within_one_lsb() {
        // Silence in, dither only: every sample must land on 0 or +/-1, or the
        // noise is louder than it should be.
        let out = to_i16(&vec![0.0; 4096]);
        assert!(out.iter().all(|s| s.abs() <= 1), "dither exceeded 1 LSB");
        assert!(
            out.iter().any(|s| *s != 0),
            "dither produced nothing at all"
        );
    }

    #[test]
    fn dither_is_reproducible() {
        let input = vec![0.25f32; 256];
        assert_eq!(to_i16(&input), to_i16(&input));
    }

    #[test]
    fn twenty_four_bit_is_exact() {
        assert_eq!(to_i24(&[0.0, 1.0, -1.0]), vec![0, 8_388_607, -8_388_607]);
    }
}
