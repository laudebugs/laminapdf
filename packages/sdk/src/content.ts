// What a page already draws, for an editor to select and change: the type of `doc.content`
// (ADR 0022). Reads need a document opened while the extract or text module was loaded; edits go
// through `apply` as one `editContent` op. Only a type: `Document` builds the object, which keeps
// the package's JavaScript small.

import type { ContentEdit, Edited, ObjectRef, PageObject } from "./types.js";

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
