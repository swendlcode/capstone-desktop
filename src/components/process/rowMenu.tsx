import { Copy, ExportSquare, FolderOpen, Play, Refresh2, Trash } from '../ui/icons';
import type { ContextMenuEntry } from '../ui/ContextMenu';
import type { QueuedFile } from '../../types';

export interface RowMenuActions {
  onPlay: () => void;
  onReveal: () => void;
  onCopyPath: () => void;
  onReanalyze: () => void;
  onOpenOutput: () => void;
  onRemove: () => void;
}

/**
 * The right-click menu for one queue row.
 *
 * Every row shows the same seven entries in the same order. What a row cannot
 * do right now is greyed out, never dropped: a menu that changes shape per row
 * has to be re-read every time it opens, and the user loses the muscle memory
 * for where Remove sits.
 *
 * Re-analyze is the one entry with no other affordance in the app. An errored
 * row could previously only be re-read by removing it and adding it back.
 */
export function rowMenu(file: QueuedFile, actions: RowMenuActions): ContextMenuEntry[] {
  // Mid-read or mid-write: a second analysis would race the one in flight, and
  // removing the row the runner is holding leaves the run writing to nothing.
  const reading = file.status === 'analyzing';
  const writing = file.status === 'processing';
  const output = file.result?.outputPath ?? null;

  return [
    {
      label: 'Play file',
      icon: <Play size={15} color="currentColor" variant="Bold" />,
      onSelect: actions.onPlay,
    },
    {
      label: 'Reveal in Finder',
      icon: <FolderOpen size={15} color="currentColor" variant="Linear" />,
      onSelect: actions.onReveal,
    },
    {
      label: 'Copy path',
      icon: <Copy size={15} color="currentColor" variant="Linear" />,
      onSelect: actions.onCopyPath,
    },
    { separator: true },
    {
      label: 'Re-analyze',
      icon: <Refresh2 size={15} color="currentColor" variant="Linear" />,
      disabled: reading || writing,
      onSelect: actions.onReanalyze,
    },
    {
      label: 'Open output',
      icon: <ExportSquare size={15} color="currentColor" variant="Linear" />,
      disabled: output === null,
      onSelect: actions.onOpenOutput,
    },
    { separator: true },
    {
      label: 'Remove from queue',
      icon: <Trash size={15} color="currentColor" variant="Linear" />,
      danger: true,
      disabled: writing,
      onSelect: actions.onRemove,
    },
  ];
}
