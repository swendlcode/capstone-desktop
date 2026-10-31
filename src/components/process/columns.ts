/**
 * The one definition of the table's column widths.
 *
 * Header and rows are separate elements — the header has to stay put while the
 * body scrolls, and the body is virtualized, so they cannot be one `<table>`.
 * That makes a single shared grid template the only thing keeping the labels
 * over their own figures.
 *
 * cover | Name | Clips | Dynamics | Volume | Pitch | Status | remove
 *
 * The cover column is a fixed 36px, holding a 28px tile. Fixed and not `auto`,
 * because an auto column would be one width for a row with artwork and another
 * for a row without, and the names below would no longer line up.
 *
 * Volume is the widest of the four measurements because after a run it holds a
 * before and an after: `-18.2 → -12.5`.
 */
export const QUEUE_GRID = '36px minmax(0,1fr) 72px 92px 136px 104px 112px 28px';

/** Row height in px. Fixed, because the virtualizer measures nothing. */
export const ROW_HEIGHT = 40;
