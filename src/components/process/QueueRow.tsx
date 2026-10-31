import { memo } from 'react';
import { Trash } from '../ui/icons';
import { MetricCell } from './MetricCell';
import { StatusCell } from './StatusCell';
import { QUEUE_GRID, ROW_HEIGHT } from './columns';
import { fmtCents, fmtClips, fmtDb, fmtDuration, fmtLufs } from './format';
import type { QueuedFile } from '../../types';

interface QueueRowProps {
  file: QueuedFile;
  selected: boolean;
  onSelect: (id: string, additive: boolean) => void;
  onRemove: (id: string) => void;
}

/**
 * One queued file: name, the four measurements, status, and a remove action
 * that appears on hover.
 *
 * Memoized because the table is driven by `capstone://file-updated`, which
 * fires once per row per status change. During a run of several hundred files
 * that is several hundred renders of the list, and only one row has actually
 * changed in each.
 */
export const QueueRow = memo(function QueueRow({
  file,
  selected,
  onSelect,
  onRemove,
}: QueueRowProps) {
  const a = file.analysis;
  const r = file.result;

  // Before → after, from the two sources the contract gives us: `analysis` is
  // what was measured, `result` is what the run did. Where the run reports
  // nothing for a column, the column keeps showing the measurement.
  const clips = a ? fmtClips(a.clipCount) : null;
  const clipsAfter = r ? fmtClips(Math.max(0, a ? a.clipCount - r.clipsRepaired : 0)) : null;

  const volume = a ? fmtLufs(a.lufs) : null;

  const pitch = a ? fmtCents(a.tuningCents) : null;
  // A reported correction means the file was brought to pitch, so the cell ends
  // on zero. The contract carries the amount applied, not the residual.
  const pitchAfter = r && r.pitchCorrectedCents !== 0 ? fmtCents(0) : null;

  return (
    <div
      role="row"
      aria-selected={selected}
      onMouseDown={(e) => onSelect(file.id, e.metaKey || e.ctrlKey || e.shiftKey)}
      style={{ gridTemplateColumns: QUEUE_GRID, height: ROW_HEIGHT }}
      className={`group grid items-center gap-3 border-b border-gray-800 px-4 transition-colors ${
        selected ? 'bg-capstone-sky/10' : 'hover:bg-gray-900'
      }`}
    >
      <div className="flex min-w-0 items-baseline gap-2" title={file.path}>
        <span className="truncate text-[13px] text-capstone-white">{file.filename}</span>
        {a && (
          <span className="mono shrink-0 text-[10px] text-gray-600">
            {fmtDuration(a.durationSec)}
          </span>
        )}
      </div>

      <MetricCell value={r ? clipsAfter : clips} before={r ? clips : null} />
      <MetricCell value={a ? fmtDb(a.crestDb) : null} />
      <MetricCell
        value={r ? fmtLufs(r.lufsAfter) : volume}
        before={r ? fmtLufs(r.lufsBefore) : null}
      />
      <MetricCell value={pitchAfter ?? pitch} before={pitchAfter ? pitch : null} />

      <StatusCell status={file.status} error={file.error} />

      <button
        onClick={(e) => {
          // The row is selectable, and a remove that also selected the row it
          // just deleted leaves the selection pointing at nothing.
          e.stopPropagation();
          onRemove(file.id);
        }}
        onMouseDown={(e) => e.stopPropagation()}
        aria-label={`Remove ${file.filename}`}
        title="Remove"
        className="flex h-6 w-6 items-center justify-center rounded text-gray-600 opacity-0 transition-opacity hover:bg-gray-700 hover:text-capstone-white focus-visible:opacity-100 group-hover:opacity-100"
      >
        <Trash size={14} color="currentColor" variant="Linear" />
      </button>
    </div>
  );
});
