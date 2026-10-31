export type Warmth = 'none' | 'gentle' | 'tube';

/** snake_case in JSON, matching the Rust enum's own rename. */
export type PitchMode = 'keep_tempo' | 'change_tempo';

/** One band of the multiband compressor. f32 on the Rust side. */
export interface BandSetting {
  lowHz: number;
  highHz: number;
  thresholdDb: number;
  ratio: number;
  attackMs: number;
  releaseMs: number;
  makeupDb: number;
}

/**
 * A named set of processing settings. Stock templates arrive with
 * `isStock: true` and are read-only: the UI duplicates one to make it editable
 * rather than editing in place, so the shipped defaults are always recoverable.
 */
export interface Template {
  id: string;
  name: string;
  isStock: boolean;
  targetLufs: number;
  /** 0–10. How hard the limiter leans on the peaks. */
  limiterCharacter: number;
  repairClips: boolean;
  /** Only repair when the track is louder than this, in LUFS. */
  repairAboveLufs: number;
  clipThresholdPct: number;
  headroomDb: number;
  warmth: Warmth;
  correctPitch: boolean;
  pitchToleranceCents: number;
  pitchMode: PitchMode;
  bands: BandSetting[];
}
