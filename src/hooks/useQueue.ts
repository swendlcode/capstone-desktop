import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { capstoneService } from '../services/capstoneService';
import { useQueueStore } from '../stores/queueStore';
import {
  EVENT_FILE_UPDATED,
  EVENT_QUEUE_FINISHED,
  EVENT_QUEUE_PROGRESS,
} from '../types';
import type { QueuedFile, QueueFinished, QueueProgress } from '../types';

export const queueQueryKeys = {
  all: ['queue'] as const,
};

/** The queued files, as the backend holds them. */
export function useQueue() {
  return useQuery({
    queryKey: queueQueryKeys.all,
    queryFn: () => capstoneService.getQueue(),
  });
}

/**
 * Every mutation that touches the queue, each invalidating the list on success
 * so the table reflects what the backend actually did rather than what the UI
 * assumed.
 *
 * `start` and `cancel` are the exception: `start_processing` returns the moment
 * the run is spawned, so there is nothing new to read yet — the status changes
 * arrive as events instead.
 */
export function useQueueActions() {
  const queryClient = useQueryClient();
  const markStarted = useQueueStore((s) => s.markStarted);
  const clearSelection = useQueueStore((s) => s.clearSelection);
  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: queueQueryKeys.all });

  /**
   * Adds the rows `add_paths` returned to the table already on screen, rather
   * than refetching the whole queue.
   *
   * Not just to save a round trip on a drop of three hundred. The backend
   * starts analysing the moment it has queued a row, so a refetch is a
   * snapshot racing live `capstone://file-updated` events: a row that finished
   * -- or failed on a file that cannot be decoded at all, which is immediate --
   * inside that window had its result patched into the cache and then
   * overwritten by the older snapshot, leaving it stuck on Queued with no
   * event left to come.
   */
  const append = (added: QueuedFile[]) => {
    if (added.length === 0) return;
    queryClient.setQueryData<QueuedFile[]>(queueQueryKeys.all, (prev) => {
      if (!prev) return added;
      const known = new Set(prev.map((f) => f.id));
      return [...prev, ...added.filter((f) => !known.has(f.id))];
    });
  };

  const addPaths = useMutation({
    mutationFn: (paths: string[]) => capstoneService.addPaths(paths),
    onSuccess: append,
  });

  const analyze = useMutation({
    mutationFn: (id: string) => capstoneService.analyzeFile(id),
    // No invalidate: `analyze_file` emits the row twice over
    // `capstone://file-updated`, and the second one carries the measurements.
    onSuccess: () => undefined,
  });

  const remove = useMutation({
    mutationFn: (id: string) => capstoneService.removeFromQueue(id),
    onSuccess: () => {
      clearSelection();
      invalidate();
    },
  });

  const clear = useMutation({
    mutationFn: () => capstoneService.clearQueue(),
    onSuccess: () => {
      clearSelection();
      invalidate();
    },
  });

  const start = useMutation({
    mutationFn: () => capstoneService.startProcessing(),
    // Raise the flag here rather than waiting for the first progress event, so
    // the button does not stay clickable through the gap.
    onMutate: markStarted,
  });

  const cancel = useMutation({
    mutationFn: () => capstoneService.cancelProcessing(),
    onSuccess: invalidate,
  });

  /** Opens the native picker and queues whatever came back. */
  const addViaPicker = useMutation({
    mutationFn: async () => {
      const paths = await capstoneService.pickFiles();
      if (paths.length === 0) return [];
      return capstoneService.addPaths(paths);
    },
    onSuccess: append,
  });

  return { addPaths, addViaPicker, analyze, remove, clear, start, cancel };
}

/**
 * The single listener for the backend's three queue events. Mounted once, from
 * App — per-component listeners plus a blanket `invalidateQueries` on every
 * file is how a long run turns into a re-query storm.
 *
 * `file-updated` therefore patches the cached array in place instead of
 * invalidating it: the event already carries the whole row, so a refetch would
 * ask the backend for something we were just handed, once per file.
 */
export function useQueueSync() {
  const queryClient = useQueryClient();
  const setProgress = useQueueStore((s) => s.setProgress);
  const markFinished = useQueueStore((s) => s.markFinished);

  useEffect(() => {
    const unsubs: Array<Promise<() => void>> = [];

    unsubs.push(
      listen<QueuedFile>(EVENT_FILE_UPDATED, (e) => {
        const file = e.payload;
        queryClient.setQueryData<QueuedFile[]>(queueQueryKeys.all, (prev) => {
          if (!prev) return prev;
          const index = prev.findIndex((f) => f.id === file.id);
          // A row we do not know about is one the user has just removed: an
          // analysis already past its own lookup goes on to emit for it. Adding
          // it back here is how a deleted row reappears seconds later.
          if (index === -1) return prev;
          const next = prev.slice();
          next[index] = file;
          return next;
        });
      }),
    );

    unsubs.push(
      listen<QueueProgress>(EVENT_QUEUE_PROGRESS, (e) => setProgress(e.payload)),
    );

    unsubs.push(
      listen<QueueFinished>(EVENT_QUEUE_FINISHED, (e) => {
        markFinished(e.payload);
        // One reconciling refetch at the end, in case an event was dropped
        // while the window was hidden.
        queryClient.invalidateQueries({ queryKey: queueQueryKeys.all });
      }),
    );

    return () => {
      Promise.all(unsubs).then((fns) => fns.forEach((fn) => fn()));
    };
  }, [queryClient, setProgress, markFinished]);
}
