import { useCallback, useEffect } from 'react';
import { DropOverlay, DropTarget } from '../components/process/DropTarget';
import { ProcessFooter } from '../components/process/ProcessFooter';
import { QueueTable } from '../components/process/QueueTable';
import { QueueToolbar } from '../components/process/QueueToolbar';
import { useFileDrop } from '../components/process/useFileDrop';
import { useQueue, useQueueActions } from '../hooks/useQueue';
import { useQueueStore } from '../stores/queueStore';
import type { FileStatus } from '../types';

/**
 * What a run will pick up. The runner skips `done` and retries everything
 * else, so this mirrors its own filter rather than guessing from the UI.
 */
const READY: FileStatus[] = ['pending', 'analyzing', 'analyzed', 'error'];

/**
 * Process Audio Files — the first tab, and the one the app opens on.
 *
 * The queue is server state and lives in react-query; this page keeps no copy
 * of it. Row updates arrive as `capstone://file-updated` through the listener
 * App mounts, so nothing here polls and nothing refetches per file.
 *
 * Analysis is not driven from here. `add_paths` starts it on the rows it
 * queued, so the measurements fill in whichever tab the user is looking at,
 * and the backend's own pool is the only thing deciding how many files are
 * read at once. A second scheduler on this side read every dropped file twice.
 */
export function ProcessPage() {
  const { data: files = [], isLoading } = useQueue();
  const { addPaths, addViaPicker, analyze, remove, clear, start, cancel } = useQueueActions();

  const isProcessing = useQueueStore((s) => s.isProcessing);
  const progress = useQueueStore((s) => s.progress);
  const lastRun = useQueueStore((s) => s.lastRun);
  const selectedIds = useQueueStore((s) => s.selectedIds);
  const selectOnly = useQueueStore((s) => s.selectOnly);
  const toggleSelected = useQueueStore((s) => s.toggleSelected);
  const clearSelection = useQueueStore((s) => s.clearSelection);

  const onDrop = useCallback((paths: string[]) => addPaths.mutate(paths), [addPaths]);
  const isOver = useFileDrop(onDrop);

  const handleSelect = useCallback(
    (id: string, additive: boolean) => {
      if (additive) toggleSelected(id);
      else selectOnly(id);
    },
    [selectOnly, toggleSelected],
  );

  const handleRemove = useCallback((id: string) => remove.mutate(id), [remove]);

  // Re-analyze from the row menu. Nothing is patched in here: `analyze_file`
  // emits the row over `capstone://file-updated` twice, and the second one
  // carries the measurements.
  const handleReanalyze = useCallback((id: string) => analyze.mutate(id), [analyze]);

  // Delete removes the selection, Escape drops it. Nothing on this tab takes
  // typed input, but the listener is guarded on the target anyway so it stays
  // correct if a filter field is ever added.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      if (target && (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA')) return;
      if (e.key === 'Escape') {
        clearSelection();
        return;
      }
      if (e.key !== 'Backspace' && e.key !== 'Delete') return;
      if (isProcessing || selectedIds.size === 0) return;
      e.preventDefault();
      selectedIds.forEach((id) => remove.mutate(id));
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [clearSelection, isProcessing, remove, selectedIds]);

  const readyCount = files.filter((f) => READY.includes(f.status)).length;
  const busy = addPaths.isPending || addViaPicker.isPending || start.isPending;
  const pickFiles = () => addViaPicker.mutate();

  return (
    <div className="relative flex h-full min-h-0 flex-col">
      {files.length === 0 ? (
        // The empty state is the drop target itself, not a message with a zone
        // underneath it.
        isLoading ? (
          <div className="min-h-0 flex-1" />
        ) : (
          <DropTarget isOver={isOver} disabled={busy} onAddFiles={pickFiles} />
        )
      ) : (
        <>
          <QueueToolbar files={files} onClear={() => clear.mutate()} disabled={isProcessing} />
          <QueueTable
            files={files}
            selectedIds={selectedIds}
            onSelect={handleSelect}
            onRemove={handleRemove}
            onReanalyze={handleReanalyze}
          />
        </>
      )}

      {isOver && files.length > 0 && <DropOverlay />}

      <ProcessFooter
        readyCount={readyCount}
        isProcessing={isProcessing}
        progress={progress}
        lastRun={lastRun}
        onAddFiles={pickFiles}
        onStart={() => start.mutate()}
        onCancel={() => cancel.mutate()}
        busy={busy}
      />
    </div>
  );
}
