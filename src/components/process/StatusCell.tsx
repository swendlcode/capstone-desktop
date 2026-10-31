import type { FileStatus } from '../../types';

interface StatusCellProps {
  status: FileStatus;
  /** The backend's own message, shown on hover rather than in the cell. */
  error?: string | null;
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

export function StatusCell({ status, error }: StatusCellProps) {
  return (
    <span
      className={`flex items-center gap-2 text-[11px] uppercase tracking-wide ${TONES[status]}`}
      title={status === 'error' && error ? error : undefined}
    >
      <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${DOTS[status]}`} />
      <span className="truncate">{LABELS[status]}</span>
    </span>
  );
}
