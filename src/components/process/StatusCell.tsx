import type { FileStatus } from '../../types';

interface StatusCellProps {
  status: FileStatus;
  /** The backend's own message, shown on hover rather than in the cell. */
  error?: string | null;
  /**
   * Something that happened which is not a failure: the output was skipped
   * because one was already there, or renamed to avoid overwriting one.
   */
  note?: string | null;
  /**
   * Whether a file was actually written. A skipped row finishes as `done`
   * with no result, and labelling that "Done" would say the opposite of what
   * happened — the user would go looking for a master that is not there.
   */
  wrote?: boolean;
}

const LABELS: Record<FileStatus, string> = {
  pending: 'Queued',
  analyzing: 'Reading',
  analyzed: 'Ready',
  processing: 'Working',
  done: 'Done',
  error: 'Error',
};

// The accent carries live and finished states and nothing else, so a glance
// down the column separates what the app is doing from what is merely queued.
// `error` is the palette's own danger token, which exists because a failed file
// that reads grey like a queued one is a file the user never notices.
const TONES: Record<FileStatus, string> = {
  pending: 'text-gray-500',
  analyzing: 'text-gray-300',
  analyzed: 'text-gray-200',
  processing: 'text-capstone-sky',
  done: 'text-capstone-sky',
  error: 'text-capstone-red',
};

const DOTS: Record<FileStatus, string> = {
  pending: 'bg-gray-600',
  analyzing: 'bg-gray-400 animate-pulse',
  analyzed: 'bg-gray-400',
  processing: 'bg-capstone-sky animate-pulse',
  done: 'bg-capstone-sky',
  error: 'bg-capstone-red',
};

export function StatusCell({ status, error, note, wrote = true }: StatusCellProps) {
  // Finished, but nothing was written: the collision policy left the existing
  // file alone. Grey rather than the accent, because the accent down this
  // column means "this one was mastered".
  const skipped = status === 'done' && !wrote;

  return (
    <span
      className={`flex items-center gap-2 text-[11px] uppercase tracking-wide ${
        skipped ? 'text-gray-400' : TONES[status]
      }`}
      title={(status === 'error' ? error : note) ?? note ?? undefined}
    >
      <span
        className={`h-1.5 w-1.5 shrink-0 rounded-full ${
          skipped ? 'bg-gray-500' : DOTS[status]
        }`}
      />
      <span className="truncate">{skipped ? 'Skipped' : LABELS[status]}</span>
    </span>
  );
}
