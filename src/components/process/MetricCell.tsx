interface MetricCellProps {
  /** What analysis measured. Null until it has run. */
  value: string | null;
  /**
   * The measurement a finished run replaced, if any. Set it and the cell shows
   * the change — `-18.2 → -12.5` — rather than only the figure it ended on,
   * which on its own says nothing about whether the file needed the work.
   */
  before?: string | null;
}

/**
 * One analysis figure, in DM Mono and right-aligned so the column reads as a
 * column.
 *
 * The unit lives in the header label, not here: repeating `dB` down forty rows
 * costs the width that the before → after pair needs.
 */
export function MetricCell({ value, before }: MetricCellProps) {
  if (value === null) {
    // Quiet placeholder, not a zero: a row that has not been measured yet must
    // not read as a row with nothing wrong with it.
    return <span className="mono block text-right text-xs text-gray-600">–</span>;
  }

  if (before != null && before !== value) {
    return (
      <span className="mono flex items-baseline justify-end gap-1 text-xs">
        <span className="text-gray-500">{before}</span>
        <span className="text-gray-600">→</span>
        <span className="text-capstone-sky">{value}</span>
      </span>
    );
  }

  return (
    <span className="mono block truncate text-right text-xs text-capstone-white">{value}</span>
  );
}
