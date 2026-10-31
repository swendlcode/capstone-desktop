/**
 * Backend → frontend event names, emitted on the app handle in Rust.
 *
 * They live here as constants rather than inline strings because a typo in a
 * `listen()` call is not a type error and not a runtime error either: the
 * listener simply never fires, which looks exactly like a backend that forgot
 * to emit.
 */
export const EVENT_FILE_UPDATED = 'capstone://file-updated' as const;
export const EVENT_QUEUE_PROGRESS = 'capstone://queue-progress' as const;
export const EVENT_QUEUE_FINISHED = 'capstone://queue-finished' as const;

export interface QueueProgress {
  done: number;
  total: number;
}

export interface QueueFinished {
  processed: number;
  failed: number;
}
