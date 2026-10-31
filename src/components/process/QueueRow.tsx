import { memo } from 'react';
import { ContextMenu, useContextMenu } from '../ui/ContextMenu';
import { Trash } from '../ui/icons';
import { CoverCell } from './CoverCell';
import { MetricCell } from './MetricCell';
import { StatusCell } from './StatusCell';
import { QUEUE_GRID, ROW_HEIGHT } from './columns';
import { fmtCents, fmtClips, fmtDb, fmtDuration, fmtLufs } from './format';
import { rowMenu } from './rowMenu';
import { useRowActions } from './useRowActions';
import type { QueuedFile } from '../../types';

interface QueueRowProps {
  file: QueuedFile;
  selected: boolean;
  onSelect: (id: string, additive: boolean) => void;
  onRemove: (id: string) => void;
  onReanalyze: (id: string) => void;
}

/**
 * One queued file: cover, name, the four measurements, status, and a remove
 * action that appears on hover. Right-clicking it opens the row menu.
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
  onReanalyze,
}: QueueRowProps) {
  const a = file.analysis;
  const r = file.result;
  const menu = useContextMenu();
  const shell = useRowActions();

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
      onMouseDown={(e) => {
        // Left button only. A right-click selects through the handler below,
        // and letting both fire ran the selection twice for one click.
        if (e.button !== 0) return;
        onSelect(file.id, e.metaKey || e.ctrlKey || e.shiftKey);
      }}
      onContextMenu={(e) => {
        // The menu acts on this row, so this row has to be the selected one
        // before it opens.
        onSelect(file.id, false);
        menu.open(e);
      }}
      style={{ gridTemplateColumns: QUEUE_GRID, height: ROW_HEIGHT }}
      className={`group grid items-center gap-3 border-b border-gray-800 px-4 transition-colors ${
        selected ? 'bg-capstone-sky/10' : 'hover:bg-gray-900'
      }`}
    >
      <CoverCell path={file.path} />

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

      {menu.at && (
        <ContextMenu
          x={menu.at.x}
          y={menu.at.y}
          onClose={menu.close}
          items={rowMenu(file, {
            onPlay: () => shell.open.mutate(file.path),
            onReveal: () => shell.reveal.mutate(file.path),
            onCopyPath: () => shell.copyPath.mutate(file.path),
            onReanalyze: () => onReanalyze(file.id),
            // Checked here too, not just disabled in the menu: the entry and
            // the handler read the same field, and only one of them is visible.
            onOpenOutput: () => {
              const out = file.result?.outputPath;
              if (out) shell.open.mutate(out);
            },
            onRemove: () => onRemove(file.id),
          })}
        />
      )}
    </div>
  );
});
