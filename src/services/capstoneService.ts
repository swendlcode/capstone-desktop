// The only module in the frontend that calls `invoke`. Components reach the
// backend through a hook, which reaches it through here; nothing shortcuts
// straight to IPC.
//
// Note that `@tauri-apps/api/core` is aliased in vite.config.ts to
// src/lib/tauri-core.ts — the import below reads as the real package but
// resolves to the local shim.
import { invoke } from '@tauri-apps/api/core';
import type {
  Analysis,
  CoverArt,
  ImportedTemplate,
  QueuedFile,
  Settings,
  Template,
} from '../types';

/**
 * One function per registered Tauri command, named after it.
 *
 * Every argument object is camelCase, matching the
 * `#[serde(rename_all = "camelCase")]` on the Rust side. A snake_case key here
 * does not fail the build: the command receives `None` and the call either
 * errors deep in Rust or quietly does nothing.
 */
export const capstoneService = {
  // ── Queue ────────────────────────────────────────────────────────────────

  /** Expands any folders among `paths` and keeps only audio files. */
  addPaths(paths: string[]): Promise<QueuedFile[]> {
    return invoke('add_paths', { paths });
  },

  analyzeFile(id: string): Promise<Analysis> {
    return invoke('analyze_file', { id });
  },

  getQueue(): Promise<QueuedFile[]> {
    return invoke('get_queue');
  },

  removeFromQueue(id: string): Promise<void> {
    return invoke('remove_from_queue', { id });
  },

  clearQueue(): Promise<void> {
    return invoke('clear_queue');
  },

  /**
   * Returns as soon as the run is spawned, not when it finishes. Progress
   * arrives as `capstone://queue-progress` and the end as
   * `capstone://queue-finished`.
   */
  startProcessing(): Promise<void> {
    return invoke('start_processing');
  },

  cancelProcessing(): Promise<void> {
    return invoke('cancel_processing');
  },

  // ── Cover art ────────────────────────────────────────────────────────────

  /**
   * The file's embedded artwork as a `data:` URI, already scaled down, or null
   * when it has none.
   *
   * Keyed on `path` and not on a queue id, because the backend caches on the
   * path: the same file added twice is read once.
   */
  getCoverArt(path: string): Promise<CoverArt> {
    return invoke('get_cover_art', { path });
  },

  // ── Templates ────────────────────────────────────────────────────────────

  getTemplates(): Promise<Template[]> {
    return invoke('get_templates');
  },

  /** Returns the stored template, which may differ from what was sent. */
  saveTemplate(template: Template): Promise<Template> {
    return invoke('save_template', { template });
  },

  /** The only way to get an editable copy of a stock template. */
  duplicateTemplate(id: string, name: string): Promise<Template> {
    return invoke('duplicate_template', { id, name });
  },

  deleteTemplate(id: string): Promise<void> {
    return invoke('delete_template', { id });
  },

  /**
   * Writes the template to a file the user picks, through a native save
   * dialog. Resolves to the path written, or null when the user cancels.
   */
  exportTemplate(id: string): Promise<string | null> {
    return invoke('export_template', { id });
  },

  /**
   * Opens a shared template file and stores it as one of the user's own.
   * Null when the user cancels; rejects with a sentence when the file is not
   * a template Capstone can read.
   */
  importTemplate(): Promise<ImportedTemplate | null> {
    return invoke('import_template');
  },

  // ── Settings ─────────────────────────────────────────────────────────────

  getSettings(): Promise<Settings> {
    return invoke('get_settings');
  },

  updateSettings(settings: Settings): Promise<Settings> {
    return invoke('update_settings', { settings });
  },

  // ── Shell ────────────────────────────────────────────────────────────────

  /** Reveals the configured output folder in Finder / Explorer. */
  openOutputFolder(): Promise<void> {
    return invoke('open_output_folder');
  },

  /** Native file picker. Empty array when the user cancels. */
  pickFiles(): Promise<string[]> {
    return invoke('pick_files');
  },

  /** Native folder picker. Null when the user cancels. */
  pickFolder(): Promise<string | null> {
    return invoke('pick_folder');
  },
};
