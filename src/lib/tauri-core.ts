// Transport shim for `@tauri-apps/api/core`.
//
// Vite aliases `@tauri-apps/api/core` to this module (see vite.config.ts), so
// every `import { invoke } from '@tauri-apps/api/core'` in the app resolves
// here. The alias is invisible in the importing file, which is the one thing to
// remember about it: grepping for "tauri-core" finds nothing.
//
// Capstone has no browser-access fallback, so `invoke` is a straight
// pass-through. The module still has to exist, because the alias redirects
// PLUGIN imports too — a Tauri plugin that pulls a symbol from
// `@tauri-apps/api/core` gets it from here, and anything this file does not
// re-export resolves to `undefined` at runtime with no build error.
import { core } from '@tauri-apps/api';

/** True inside the Tauri webview, false when the bundle is opened in a browser. */
export const isTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

// @tauri-apps/plugin-updater extends Resource for its Update handle.
export const Resource = core.Resource;

// Nothing bundled today uses Channel, but plugins that stream progress do, and
// finding out the hard way costs a white screen rather than a compile error.
export const Channel = core.Channel;

export function invoke<T = unknown>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return core.invoke<T>(cmd, args);
}

export function convertFileSrc(filePath: string, protocol = 'asset'): string {
  return core.convertFileSrc(filePath, protocol);
}
