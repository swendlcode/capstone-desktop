export type OutputFormat = 'aiff' | 'alac' | 'flac' | 'mp3' | 'wav';

export type Mp3Rate = 'cbr320' | 'vbr';

/** snake_case in JSON, matching the Rust enum's own rename. */
export type OutputLocation = 'specific' | 'same_as_source';

/**
 * Application preferences — the Personalize tab edits this whole object and
 * sends it back through `update_settings`, which returns the stored version.
 */
export interface Settings {
  outputLocation: OutputLocation;
  /** Null while `outputLocation` is `same_as_source`. */
  outputDir: string | null;
  mirrorInputStructure: boolean;
  format: OutputFormat;
  mp3Rate: Mp3Rate;
  /** Worker count for the processing pool; defaults to every core. */
  maxCores: number;
  activeTemplateId: string;
}
