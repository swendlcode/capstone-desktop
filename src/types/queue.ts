import type { Analysis } from './analysis';

/** Lowercase in JSON: the Rust enum carries `rename_all = "lowercase"`. */
export type FileStatus =
  | 'pending'
  | 'analyzing'
  | 'analyzed'
  | 'processing'
  | 'done'
  | 'error';

/** What processing actually did, as opposed to what analysis predicted. */
export interface ProcessResult {
  outputPath: string;
  lufsBefore: number;
  lufsAfter: number;
  /**
   * True peak of the finished file, dBTP. Loudness is an average, so
   * `lufsAfter` alone cannot say whether the master is over its ceiling.
   */
  truePeakAfterDb: number;
  /** The limiter ceiling it was produced against, dBTP. */
  ceilingDb: number;
  clipsRepaired: number;
  gainAppliedDb: number;
  pitchCorrectedCents: number;
  /** Whether the output carries the source's artist, title and the rest. */
  keptTags?: boolean;
  /**
   * Where the embedded cover came from — the artwork folder, beside the file,
   * or the source's own tags — or absent when the output has none.
   */
  artworkFrom?: string | null;
}

export interface QueuedFile {
  id: string;
  path: string;
  filename: string;
  /** Null until analysis has run; the middle table columns read from here. */
  analysis: Analysis | null;
  status: FileStatus;
  /** Null until the file has been written out. */
  result: ProcessResult | null;
  error: string | null;
  /**
   * Something that happened which is not a failure: the output was skipped
   * because one already existed, or renamed to avoid overwriting one. Absent
   * on an ordinary row.
   */
  note?: string | null;
  /**
   * The folder this file was dropped in, or null when it was picked
   * individually. Only "Mirror structure of input folders" reads it — it is
   * what tells the backend how much of the path below the drop to reproduce.
   */
  sourceRoot: string | null;
}
