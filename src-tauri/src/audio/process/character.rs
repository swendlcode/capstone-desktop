//! "Limiter Character", 0 to 10, default 4.
//!
//! One dial over four correlated parameters. A table rather than a fitted
//! exponent because the control the UI sends is an integer, so eleven entries
//! are exact, auditable and re-tunable by ear by editing eleven numbers.
//!
//! The spec fixes the even steps; the odd ones are the interpolation between
//! their neighbours, which is why they look like nothing in particular.
//!
//! What the two ends sound like, so it is possible to tell whether this is
//! working at all. Low: fast and transparent, flattens transients so a kick
//! loses its click, and preserves average level best. High: slow, rides the
//! level, "glued", and pumps on sparse material.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Character {
    pub lookahead_ms: f64,
    /// Release time constant. The program-dependent stages are derived from it
    /// rather than stored, so the ratios between them cannot drift.
    pub release_ms: f64,
    /// Soft-knee width in dB. Quadratic, infinite ratio.
    pub knee_db: f64,
    /// 1 means a single release pole; 2 and 3 are the parallel
    /// program-dependent releases.
    pub release_stages: u8,
}

// One row per dial position, left as a table on purpose: the whole value of a
// lookup over a fitted curve is that somebody re-tuning it by ear can read all
// eleven settings at once.
#[rustfmt::skip]
const TABLE: [Character; 11] = [
    Character { lookahead_ms: 1.0, release_ms: 1.0, knee_db: 0.0, release_stages: 1 },
    Character { lookahead_ms: 1.25, release_ms: 10.5, knee_db: 0.25, release_stages: 1 },
    Character { lookahead_ms: 1.5, release_ms: 20.0, knee_db: 0.5, release_stages: 1 },
    Character { lookahead_ms: 2.0, release_ms: 50.0, knee_db: 1.0, release_stages: 2 },
    Character { lookahead_ms: 2.5, release_ms: 80.0, knee_db: 1.5, release_stages: 2 },
    Character { lookahead_ms: 3.25, release_ms: 130.0, knee_db: 2.0, release_stages: 2 },
    Character { lookahead_ms: 4.0, release_ms: 180.0, knee_db: 2.5, release_stages: 2 },
    Character { lookahead_ms: 5.5, release_ms: 290.0, knee_db: 3.25, release_stages: 2 },
    Character { lookahead_ms: 7.0, release_ms: 400.0, knee_db: 4.0, release_stages: 3 },
    Character { lookahead_ms: 9.5, release_ms: 650.0, knee_db: 5.0, release_stages: 3 },
    Character { lookahead_ms: 12.0, release_ms: 900.0, knee_db: 6.0, release_stages: 3 },
];

/// Out-of-range values clamp rather than panic: this number arrives over IPC
/// from a template the user can edit by hand on disk.
pub fn lookup(character: u8) -> Character {
    TABLE[character.min(10) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_the_documented_one() {
        let four = lookup(4);
        assert_eq!(four.lookahead_ms, 2.5);
        assert_eq!(four.release_ms, 80.0);
        assert_eq!(four.knee_db, 1.5);
        assert_eq!(four.release_stages, 2);
    }

    /// The dial has to be monotonic in all four parameters or it stops being a
    /// perceptual axis and becomes a bag of presets.
    #[test]
    fn every_parameter_rises_with_the_dial() {
        for pair in TABLE.windows(2) {
            assert!(pair[1].lookahead_ms > pair[0].lookahead_ms);
            assert!(pair[1].release_ms > pair[0].release_ms);
            assert!(pair[1].knee_db > pair[0].knee_db);
            assert!(pair[1].release_stages >= pair[0].release_stages);
        }
    }

    #[test]
    fn out_of_range_clamps() {
        assert_eq!(lookup(200), lookup(10));
    }
}
