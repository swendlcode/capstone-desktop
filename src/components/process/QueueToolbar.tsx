import { Button } from '../ui/Button';
import { Trash } from '../ui/icons';
import type { QueuedFile } from '../../types';

interface QueueToolbarProps {
  files: QueuedFile[];
  onClear: () => void;
  disabled?: boolean;
}

/**
 * The strip above the table: how many files are queued, how far along they are,
 * and the clear-queue action.
 *
 * The counts are derived from the rows rather than kept alongside them. A
 * second copy of "how many are done" is a second thing that can disagree with
 * the table it sits above, and the backend never sends a count.
 */
export function QueueToolbar({ files, onClear, disabled }: QueueToolbarProps) {
  const done = files.filter((f) => f.status === 'done').length;
  const failed = files.filter((f) => f.status === 'error').length;

  return (
    <div className="flex shrink-0 items-center justify-between gap-4 px-4 py-3">
      <div className="flex items-baseline gap-3 text-[10px] uppercase tracking-[0.14em]">
        <span className="text-gray-400">
          <span className="mono text-capstone-white">{files.length}</span> files
        </span>
        {done > 0 && (
          <span className="text-gray-500">
            <span className="mono text-capstone-sky">{done}</span> done
          </span>
        )}
        {failed > 0 && (
          <span className="text-gray-500">
            <span className="mono text-[#ff6b6b]">{failed}</span> failed
          </span>
        )}
      </div>

      <Button
        variant="ghost"
        size="sm"
        onClick={onClear}
        disabled={disabled || files.length === 0}
        icon={<Trash size={14} color="currentColor" variant="Linear" />}
      >
        Clear
      </Button>
    </div>
  );
}
