// What a page draws, for an editor to select and change: the type of `doc.content` (ADR 0022;
// E6 plan §2.1). Reads of a page's own content need a document opened while the extract or text
// module was loaded; the objects Lamina PDF adds work in every module. Edits go through `apply`
// as one op each. Only a type: `Document` builds the object, which keeps the package's JavaScript
// small.

import type {
  AddedObject,
  ContentChanges,
  ContentEdit,
  ContentTextRun,
  Edited,
  NewContentObject,
  ObjectRef,
  PageObject,
  TextRunRef,
} from "./types.js";

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

  /**
   * Adds an image, a shape or a page of another PDF to page `index` as page content
   * (`addContentObject`): part of the page, printed and drawn by every reader, not a comment.
   * `object.matrix` places the object's space on the page: an image or a page fills the unit
   * square, so `[w, 0, 0, h, x, y]` draws it `w` by `h` points with its top-left corner at
   * `(x, y)`; a shape's path is in page space, so the identity draws it where its points are.
   * Add the bytes an image or a page names with {@link Document.addBlob} first.
   *
   * ```ts
   * const blob = doc.addBlob(logoPng);
   * doc.content.add(0, {
   *   id: "logo",
   *   object: { type: "image", blob },
   *   matrix: [144, 0, 0, 44, 72, 40],
   *   alt: { text: "Northwind" },
   *   modified: "D:20261118120000Z",
   * });
   * ```
   *
   * @throws {@link LaminaPdfError} `content-id-in-use`, `content-object-invalid`,
   *   `content-needs-alt-text` (a picture without `alt` in a PDF/UA-1 file), `unknown-blob`,
   *   `invalid-op`; as {@link Document.apply}.
   *
   * @since 0.7.0
   */
  add(index: number, object: NewContentObject): Edited;

  /**
   * Changes the object `id` that Lamina PDF added to page `index` (`updateContentObject`), before
   * or after a save: its matrix, opacity, an image's crop, a shape's fill and stroke, new bytes,
   * its alternate text or its place among the objects on its layer. Fields left out keep their
   * values; `null` removes a crop, a fill or a stroke.
   *
   * @throws {@link LaminaPdfError} `unknown-content-object`, `invalid-op` (a change for another
   *   kind of object); as {@link DocumentContent.add}.
   *
   * @since 0.7.0
   */
  update(index: number, id: string, changes: ContentChanges): Edited;

  /**
   * Takes the object `id` that Lamina PDF added to page `index` off the page
   * (`deleteContentObject`). It stays in the file's earlier revisions until a full save
   * (`stillInEarlierRevisions`).
   *
   * @throws {@link LaminaPdfError} `unknown-content-object`; as {@link Document.apply}.
   *
   * @since 0.7.0
   */
  remove(index: number, id: string): Edited;

  /**
   * The objects Lamina PDF added to page `index`, in the order they paint, those behind the
   * page's content first: each with its ID, what it is, its matrix, opacity, layer, alternate text
   * and its box in page space. Works in every module.
   *
   * @throws {@link LaminaPdfError} `page-index-out-of-range`; reading errors.
   *
   * @since 0.7.0
   */
  added(index: number): AddedObject[];
}
