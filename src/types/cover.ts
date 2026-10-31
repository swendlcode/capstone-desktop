/**
 * A file's embedded artwork, as a `data:` URI ready for an `<img src>`.
 *
 * `null` means the file carries no picture, or carries one we could not read —
 * the two are not distinguished on purpose, because the UI does the same thing
 * either way: draw the placeholder.
 *
 * Already scaled down by the backend (64px on the long edge). It is small
 * enough to hold in state for every row of the queue, which a full-size cover
 * is not.
 */
export type CoverArt = string | null;
