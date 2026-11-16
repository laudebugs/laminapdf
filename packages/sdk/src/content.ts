// What a page already draws, for an editor to select and change: the type of `doc.content`
// (ADR 0022). Reads need a document opened while the extract or text module was loaded; edits go
// through `apply` as one `editContent` op. Only a type: `Document` builds the object, which keeps
// the package's JavaScript small.

import type { ContentEdit, ContentTextRun, Edited, ObjectRef, PageObject, TextRunRef } from "./types.js";

/**
 * The objects the document's pages draw, and edits to them ({@link Document.content}). Pages are
 * 0-based indexes; geometry is in page space: points from the top-left of the page as shown, y
 * down. Editing existing content is best effort: an object that clips what follows it, or that
 * would draw differently in its new place in the stacking order, is refused with
 * `content-not-editable` and a reason, and changes nothing.
 *
 * @since 0.5.0
 */
export interface DocumentContent {
  /**
   * The objects page `index` draws, in painting order: paths, text objects, images, forms and
   * shadings, each with its name for an edit, its box and, when it can't be moved, why.
   *
   * @throws {@link LaminaPdfError} `extract-module-not-loaded`, `page-index-out-of-range`; reading
   *   errors.
   *
   * @since 0.5.0
   */
  objects(index: number): PageObject[];

  /**
   * The runs of text page `index` shows, in the order the page draws them: each with its name for
   * {@link DocumentContent.editText}, its text, box, font, size and line and, when it can't be
   * edited, why. Text inside a form isn't listed.
   *
   * @throws {@link LaminaPdfError} as {@link DocumentContent.objects}.
   *
   * @since 0.5.0
   */
  textRuns(index: number): ContentTextRun[];

  /**
   * Makes `run` on page `index` show `text` (`editText`), in the font it is set in: every
   * character must be one the font can draw (for an embedded subset, one the document already
   * shows in it; a space it can't draw becomes a word-sized gap). The text after the run in the
   * same text object moves by the width difference; text another object places on the line stays
   * put, and the lines below stay where they were. Empty text deletes the run. To change part of a run, name its glyphs: `start` and
   * `end` within the run's. The old text stays in the file's earlier revisions until a full save.
   *
   * @throws {@link LaminaPdfError} `missing-glyph` for a character the font has no code for,
   *   `content-not-editable` with the reason for a read-only run, `content-object-stale`; as
   *   {@link Document.apply}.
   *
   * @since 0.5.0
   */
  editText(index: number, run: TextRunRef, text: string): Edited;

  /**
   * The objects of page `index` at `at`, topmost first: a point `[x, y]` (an object within 2
   * points of it counts), or a rectangle `[x0, y0, x1, y1]` (every object whose box meets it).
   *
   * @throws {@link LaminaPdfError} as {@link DocumentContent.objects}.
   *
   * @since 0.5.0
   */
  hitTest(index: number, at: readonly [number, number] | readonly [number, number, number, number]): ObjectRef[];

  /**
   * Makes `edits` on page `index` in one transaction (`editContent`), in order. Each names its
   * object as {@link DocumentContent.objects} gave it before the edit; a name that no longer
   * matches fails with `content-object-stale`. Only the objects named are written again: the
   * rest of the page keeps its bytes, and a content stream another page also draws stays as it
   * was there. What a deletion removes stays in the file's earlier revisions until a full save
   * (`stillInEarlierRevisions`).
   *
   * @throws {@link LaminaPdfError} `content-object-stale`, `content-not-editable`,
   *   `unknown-blob`; as {@link Document.apply}.
   *
   * @since 0.5.0
   */
  edit(index: number, edits: readonly ContentEdit[]): Edited;
}
