/**
 * What analysis measured about one file, before anything is changed.
 *
 * Mirrors `Analysis` in the Rust models. Field names are camelCase on this side
 * and snake_case on that side; the bridge between them is
 * `#[serde(rename_all = "camelCase")]` on the Rust struct, so renaming a field
 * here without renaming it there fails silently as an `undefined` at runtime.
 */
export interface Analysis {
  durationSec: number;
  sampleRate: number;
  channels: number;
  /** Integrated loudness, EBU R128. Negative; around -8 for modern masters. */
  lufs: number;
  truePeakDb: number;
  /** Consecutive-sample runs sitting at or over the ceiling. */
  clipCount: number;
  /** Peak-to-RMS. This is the figure the Dynamics column shows. */
  crestDb: number;
  /** Signed deviation from concert pitch. Negative means flat. */
  tuningCents: number;
}
