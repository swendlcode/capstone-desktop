import { useRef } from 'react';
import { useVirtualizer } from '@tanstack/react-virtual';
import { QueueRow } from './QueueRow';
import { QUEUE_GRID, ROW_HEIGHT } from './columns';
import type { QueuedFile } from '../../types';

interface QueueTableProps {
  files: QueuedFile[];
  selectedIds: Set<string>;
  onSelect: (id: string, additive: boolean) => void;
  onRemove: (id: string) => void;
}

/** Units live up here so the cells below can be nothing but figures. */
const HEADERS: Array<{ label: string; unit?: string; right?: boolean }> = [
  { label: 'Name' },
  { label: 'Clips', right: true },
  { label: 'Dynamics', unit: 'dB', right: true },
  { label: 'Volume', unit: 'LUFS', right: true },
  { label: 'Pitch', unit: 'cents', right: true },
  { label: 'Status' },
];

/**
 * The queue, as a fixed header over a virtualized body.
 *
 * It is not a `<table>`: the header has to stay put while the body scrolls, and
 * the body only renders the rows in view, so the two are separate elements held
 * in line by one shared grid template (see columns.ts).
 *
 * Virtualized unconditionally rather than past some row count. A dropped folder
 * is routinely thousands of files, and a list that switches strategy at a
 * threshold is a list whose slow path is the one nobody tests.
 */
export function QueueTable({ files, selectedIds, onSelect, onRemove }: QueueTableProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: files.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 12,
  });

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div
        role="row"
        style={{ gridTemplateColumns: QUEUE_GRID }}
        className="grid shrink-0 items-center gap-3 border-b border-gray-700 bg-gray-900 px-4 py-2"
      >
        {HEADERS.map((h) => (
          <span
            key={h.label}
            className={`flex items-baseline gap-1 overflow-hidden text-[10px] uppercase tracking-[0.14em] text-gray-500 ${
              h.right ? 'justify-end' : ''
            }`}
          >
            <span className="truncate">{h.label}</span>
            {h.unit && <span className="mono text-[9px] normal-case text-gray-600">{h.unit}</span>}
          </span>
        ))}
        <span />
      </div>

      <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
        <div style={{ height: virtualizer.getTotalSize(), position: 'relative' }}>
          {virtualizer.getVirtualItems().map((item) => {
            const file = files[item.index];
            return (
              <div
                key={file.id}
                style={{
                  position: 'absolute',
                  top: 0,
                  left: 0,
                  width: '100%',
                  transform: `translateY(${item.start}px)`,
                }}
              >
                <QueueRow
                  file={file}
                  selected={selectedIds.has(file.id)}
                  onSelect={onSelect}
                  onRemove={onRemove}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
