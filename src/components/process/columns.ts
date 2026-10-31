/**
 * The one definition of the table's column widths.
 *
 * Header and rows are separate elements — the header has to stay put while the
 * body scrolls, and the body is virtualized, so they cannot be one `<table>`.
 * That makes a single shared grid template the only thing keeping the labels
 * over their own figures.
 *
 * Name | Clips | Dynamics | Volume | Pitch | Status | remove
 *
 * Volume is the widest of the four measurements because after a run it holds a
 * before and an after: `-18.2 → -12.5`.
 */
export const QUEUE_GRID = 'minmax(0,1fr) 72px 92px 136px 104px 112px 28px';

/** Row height in px. Fixed, because the virtualizer measures nothing. */
export const ROW_HEIGHT = 40;
