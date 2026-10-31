// Transport shim for `@tauri-apps/api/event`, aliased the same way as
// `./tauri-core` (see vite.config.ts).
//
// It is a pass-through: the alias pair is kept so that a later transport change
// — a browser bridge, a test double, a logging tap on every event — lands in
// one file instead of every `listen()` call site.
import { event } from '@tauri-apps/api';

export type { Event, EventCallback, UnlistenFn } from '@tauri-apps/api/event';

export const listen = event.listen;
export const once = event.once;
export const emit = event.emit;
export const emitTo = event.emitTo;
