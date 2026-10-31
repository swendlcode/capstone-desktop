//! Turning a per-sample required gain into a smooth envelope that is provably
//! never above it.
//!
//! Kept out of `limiter.rs` so that file stays inside the module size the house
//! rules set. The brickwall guarantee is a property of these four functions
//! together, so the proofs live with them.

use std::collections::VecDeque;

use super::character::Character;

/// Sliding-window minimum of the required gain over the look-ahead, then the
/// log gain smoothed by a triangular window of the same length.
///
/// Provably overshoot-free. For any sample within `La` of a peak, the window
/// minimum is at most the gain that peak requires; so is every value the
/// smoothing window averages, because that window is shorter than `La` and ends
/// at or before the sample. Any non-negative unity-sum average of values each
/// at most `r` is also at most `r`.
///
/// A triangular window rather than the Hann one: the proof needs only
/// non-negative weights summing to one, and a triangle is a box filter applied
/// twice, which is O(1) per sample instead of O(La). At a 12 ms look-ahead and
/// 48 kHz that is 576 multiplies saved on every sample of every file.
pub(super) fn attack(required: &[f64], lookahead: usize) -> Vec<f64> {
    let floor: Vec<f64> = sliding_min(required, lookahead)
        .into_iter()
        .map(|gain| gain.max(1e-12).ln())
        .collect();
    let width = (lookahead / 2).max(1);
    box_filter(&box_filter(&floor, width), width)
}

/// Minimum of `x[n ..= n + win - 1]`, monotonic deque, O(1) amortised. Windows
/// that would run past the end simply stop there.
fn sliding_min(x: &[f64], win: usize) -> Vec<f64> {
    let len = x.len();
    let mut out = vec![0.0; len];
    let mut queue: VecDeque<usize> = VecDeque::new();
    for index in 0..len {
        while queue.back().is_some_and(|back| x[*back] >= x[index]) {
            queue.pop_back();
        }
        queue.push_back(index);
        if index + 1 >= win {
            let start = index + 1 - win;
            while queue.front().is_some_and(|front| *front < start) {
                queue.pop_front();
            }
            out[start] = x[*queue.front().expect("the current index is still queued")];
        }
    }
    let tail = len.saturating_sub(win.saturating_sub(1));
    let mut running = f64::INFINITY;
    for index in (tail..len).rev() {
        running = running.min(x[index]);
        out[index] = running;
    }
    out
}

/// Causal moving average, unity sum. The first few outputs average over fewer
/// samples, which keeps the weights summing to one where there is no history
/// yet - and so keeps the overshoot proof intact at the head of the buffer.
fn box_filter(x: &[f64], width: usize) -> Vec<f64> {
    if width <= 1 {
        return x.to_vec();
    }
    let mut out = vec![0.0; x.len()];
    let mut sum = 0.0;
    for (index, slot) in out.iter_mut().enumerate() {
        sum += x[index];
        if index >= width {
            sum -= x[index - width];
        }
        *slot = sum / (index + 1).min(width) as f64;
    }
    out
}

/// Upward smoothing of the log gain only, so the release can give gain back
/// more slowly than the attack envelope allows but never faster.
///
/// Program-dependent release is a `min` of parallel poles: fast at `tau/8`,
/// slow at `tau*4`, and for the glued settings a third at `tau*20`. A short
/// isolated peak recovers quickly because the slow pole barely moved; sustained
/// loudness recovers slowly because it has wound down. A `min` of releases can
/// never return more gain than the detector asked for, so brickwall safety
/// survives by construction.
pub(super) fn release(envelope: &mut [f64], settings: &Character, fs: f64) {
    let tau = settings.release_ms / 1000.0;
    let poles: Vec<f64> = match settings.release_stages {
        0 | 1 => vec![pole(tau, fs)],
        2 => vec![pole(tau / 8.0, fs), pole(tau * 4.0, fs)],
        _ => vec![
            pole(tau / 8.0, fs),
            pole(tau * 4.0, fs),
            pole(tau * 20.0, fs),
        ],
    };
    let mut held = vec![envelope.first().copied().unwrap_or(0.0); poles.len()];
    for value in envelope.iter_mut() {
        let target = *value;
        let mut lowest = f64::INFINITY;
        for (state, coefficient) in held.iter_mut().zip(&poles) {
            *state = if target > *state {
                *state + (target - *state) * coefficient
            } else {
                target
            };
            lowest = lowest.min(*state);
        }
        *value = lowest;
    }
}

fn pole(tau: f64, fs: f64) -> f64 {
    1.0 - (-1.0 / (tau.max(1e-6) * fs)).exp()
}
