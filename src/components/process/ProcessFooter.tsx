import { Button } from '../ui/Button';
import { Add, Flash, FolderOpen, Stop } from '../ui/icons';
import { useOpenOutputFolder } from './useOutputFolder';
import type { QueueFinished, QueueProgress } from '../../types';

interface ProcessFooterProps {
  /** Rows the runner would pick up: anything not already `done`. */
  readyCount: number;
  isProcessing: boolean;
  progress: QueueProgress | null;
  lastRun: QueueFinished | null;
  onAddFiles: () => void;
  onStart: () => void;
  onCancel: () => void;
  busy?: boolean;
}

/**
 * Open Output Folder on the left, Add Files and Start Processing on the right.
 *
 * Start turns into a progress bar with a Cancel beside it rather than opening a
 * modal: the queue is the thing the user is watching, and a dialog over it
 * hides the per-row status that says which file is being worked on.
 *
 * Start and Cancel are a bolt and a stop block, both solid, rather than the
 * media triangle this button used to carry: it starts a batch job, it does not
 * play audio, and a transport glyph promised the wrong thing.
 *
 * The bar is driven by `capstone://queue-progress` only. `start_processing`
 * returns the moment the run is spawned, so there is a gap before the first
 * event in which there is no progress to show — hence the indeterminate
 * "Starting" state rather than a bar sitting at zero.
 */
export function ProcessFooter({
  readyCount,
  isProcessing,
  progress,
  lastRun,
  onAddFiles,
  onStart,
  onCancel,
  busy,
}: ProcessFooterProps) {
  const openOutput = useOpenOutputFolder();
  const pct = progress && progress.total > 0 ? (progress.done / progress.total) * 100 : 0;

  return (
    <div className="shrink-0 border-t border-gray-700 bg-gray-900">
      {isProcessing && (
        <div className="h-0.5 w-full bg-gray-800">
          <div
            className={`h-full bg-capstone-sky transition-[width] duration-200 ${
              progress ? '' : 'animate-pulse'
            }`}
            style={{ width: progress ? `${pct}%` : '12%' }}
          />
        </div>
      )}

      <div className="flex items-center justify-between gap-4 px-4 py-3">
        <div className="flex min-w-0 items-center gap-3">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => openOutput.mutate()}
            disabled={openOutput.isPending}
            icon={<FolderOpen size={15} color="currentColor" variant="Linear" />}
          >
            Open Output Folder
          </Button>
          {!isProcessing && lastRun && (
            <span className="mono truncate text-[11px] text-gray-500">
              {lastRun.processed} processed
              {lastRun.failed > 0 ? ` · ${lastRun.failed} failed` : ''}
            </span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {isProcessing ? (
            <>
              <span className="mono text-xs text-gray-400">
                {progress ? `${progress.done} / ${progress.total}` : 'Starting'}
              </span>
              <Button
                variant="secondary"
                size="sm"
                onClick={onCancel}
                icon={<Stop size={15} color="currentColor" variant="Bold" />}
              >
                Cancel
              </Button>
            </>
          ) : (
            <>
              <Button
                variant="secondary"
                size="sm"
                onClick={onAddFiles}
                disabled={busy}
                icon={<Add size={16} color="currentColor" variant="Linear" />}
              >
                Add Files...
              </Button>
              <Button
                variant="primary"
                size="sm"
                onClick={onStart}
                disabled={readyCount === 0 || busy}
                icon={<Flash size={15} color="currentColor" variant="Bold" />}
              >
                Start Processing
              </Button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
