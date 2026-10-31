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
  clipsRepaired: number;
  gainAppliedDb: number;
  pitchCorrectedCents: number;
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
   * The folder this file was dropped in, or null when it was picked
   * individually. Only "Mirror structure of input folders" reads it — it is
   * what tells the backend how much of the path below the drop to reproduce.
   */
  sourceRoot: string | null;
}
