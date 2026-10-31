/**
 * Formatters for the four analysis columns.
 *
 * Every figure here is rendered in DM Mono, so each one is formatted to a fixed
 * number of decimals even when the decimal is a zero: a column of `-18.2` over
 * `-9` jitters its decimal point, and the whole reason these are monospaced is
 * that the user scans them vertically.
 */

/** Consecutive-sample runs at or over the ceiling, as a plain count. */
export function fmtClips(count: number): string {
  return String(Math.max(0, Math.round(count)));
}

/** Crest, headroom, gain — anything in dB. The unit is a separate element. */
export function fmtDb(db: number): string {
  if (!Number.isFinite(db)) return '--';
  return db.toFixed(1);
}

/** Integrated loudness. Always negative in practice, so the sign is its own. */
export function fmtLufs(lufs: number): string {
  if (!Number.isFinite(lufs)) return '--';
  return lufs.toFixed(1);
}

/**
 * Pitch deviation, signed: the sign is the information. A `+` is written
 * explicitly because `12` and `-12` read as opposite problems and a bare `12`
 * looks like an absolute amount.
 */
export function fmtCents(cents: number): string {
  if (!Number.isFinite(cents)) return '--';
  const rounded = Math.round(cents * 10) / 10;
  if (Math.abs(rounded) < 0.05) return '0';
  return `${rounded > 0 ? '+' : ''}${rounded.toFixed(1)}`;
}

/** Duration for the row's tooltip, not for a column. */
export function fmtDuration(seconds: number): string {
  if (!Number.isFinite(seconds) || seconds < 0) return '--';
  const total = Math.round(seconds);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, '0')}`;
}
