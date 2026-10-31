import { create } from 'zustand';
import type { QueueFinished, QueueProgress } from '../types';

/**
 * Client-side state for a processing run. The queue itself is server state and
 * lives in react-query (see hooks/useQueue.ts) — nothing here duplicates a
 * `QueuedFile`.
 *
 * None of it is persisted: a run that was interrupted by a quit is over, and
 * restoring a stale "3 of 40" bar on launch would misreport the backend.
 */
interface QueueStore {
  isProcessing: boolean;
  progress: QueueProgress | null;
  /** Summary of the last finished run, for the footer. Cleared on the next start. */
  lastRun: QueueFinished | null;
  /** Rows the user has selected in the table, for remove-from-queue. */
  selectedIds: Set<string>;
  setProgress: (progress: QueueProgress) => void;
  markStarted: () => void;
  markFinished: (summary: QueueFinished) => void;
  toggleSelected: (id: string) => void;
  selectOnly: (id: string) => void;
  clearSelection: () => void;
}

export const useQueueStore = create<QueueStore>((set) => ({
  isProcessing: false,
  progress: null,
  lastRun: null,
  selectedIds: new Set(),
  // A progress event is the backend telling us a run is live, so it also
  // raises the flag: `start_processing` returns before the first file is read,
  // and a cancel that lost the race would otherwise leave the flag stuck down.
  setProgress: (progress) => set({ progress, isProcessing: true }),
  markStarted: () => set({ isProcessing: true, progress: null, lastRun: null }),
  markFinished: (lastRun) => set({ isProcessing: false, progress: null, lastRun }),
  toggleSelected: (id) =>
    set((s) => {
      const next = new Set(s.selectedIds);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { selectedIds: next };
    }),
  selectOnly: (id) => set({ selectedIds: new Set([id]) }),
  clearSelection: () => set({ selectedIds: new Set() }),
}));
