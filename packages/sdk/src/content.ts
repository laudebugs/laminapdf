// What a page draws, for an editor to select and change: the type of `doc.content` (ADR 0022;
// E6 plan §2.1). Reads of a page's own content need a document opened while the extract or text
// module was loaded; the objects Lamina PDF adds work in every module. Edits go through `apply`
// as one op each. Only a type: `Document` builds the object, which keeps the package's JavaScript
// small.

import type {
  AddedObject,
  AnnotFont,
  ContentChanges,
  ContentEdit,
  ContentTextRun,
  Edited,
  NewBates,
  NewContentObject,
  NewHeaderFooter,
  NewText,
  NewWatermark,
  ObjectRef,
  PageMark,
  PageMarkInfo,
  PageObject,
  TextEditReport,
  TextRunRef,
  TextStyle,
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
   * Pass `fonts` to let a character the run's font lacks come from another font: a full copy of
   * the run's own font (its bytes from {@link Document.addBlob}) adds a new subset of it; any
   * other font (`"helvetica"`, `"times"`, `"courier"` or a blob ID) substitutes. Nothing is
   * substituted that `fonts` doesn't name, and the returned `textEdits` say which characters
   * were drawn in which font; {@link DocumentContent.planEditText} says so before the edit.
   *
   * @throws {@link LaminaPdfError} `missing-glyph` for a character no font can draw,
   *   `content-not-editable` with the reason for a read-only run, `content-object-stale`; as
   *   {@link Document.apply}.
   *
   * @since 0.5.0
   */
  editText(index: number, run: TextRunRef, text: string, fonts?: readonly AnnotFont[]): Edited;

  /**
   * What {@link DocumentContent.editText} would do, without doing it: how faithful the result
   * would be (`fidelity`), the characters the run's font can't draw with the font that would
   * draw each (`substitutions`) and how many points wider the line would grow (`grewBy`). Show
   * it before committing the edit, so a font is never changed silently.
   *
   * @throws {@link LaminaPdfError} as {@link DocumentContent.editText}, but nothing is changed.
   *
   * @since 0.7.0
   */
  planEditText(index: number, run: TextRunRef, text: string, fonts?: readonly AnnotFont[]): TextEditReport;

  /**
   * Changes how `runs` on page `index` look (`setTextStyle`): font (`"helvetica"`, `"times"`,
   * `"courier"` or a blob ID, embedded as a new subset), `size` in points, `color`, `bold`,
   * `italic` (the face of a standard font), `underline`, `baseline` (`"normal"`, `"super"`,
   * `"sub"`), `align` and `lineSpacing` (a multiple of the glyphs' size) for the lines of the
   * runs' text object. The text stays as it is; the glyphs after a run on its line move by the
   * width difference. Each field is optional and changes only what it names.
   *
   * @throws {@link LaminaPdfError} `content-not-editable` with the reason, `content-object-stale`,
   *   `missing-glyph` when the new font lacks a character, `invalid-op` for a style that
   *   names nothing or can't be done (taking an underline off, bold on a font with no bold
   *   face to take); as {@link Document.apply}.
   *
   * @since 0.7.0
   */
  setTextStyle(index: number, runs: readonly TextRunRef[], style: TextStyle): Edited;

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
   *   object: { image: { blob } },
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
   * Adds text to page `index` as page content (`addText`), laid out in `rect`, a box in page
   * space: shaped and wrapped at the box's width, set in the first of `fonts` and, for a
   * character it lacks, the fonts after it (a symbol font, a font of another script), each
   * embedded as a subset that copies and searches as typed. Standard fonts need no bytes; add a
   * font's bytes with {@link Document.addBlob} first and name them by the ID it returns. The
   * text is a paragraph in a tagged file, or an artifact for `tag: "artifact"`. Change or remove
   * it with {@link DocumentContent.update} (`text`, `matrix`) and {@link DocumentContent.remove}.
   *
   * ```ts
   * doc.content.addText(0, {
   *   id: "note",
   *   rect: [72, 72, 288, 120],
   *   runs: [{ text: "Reviewed " }, { text: "twice", bold: true }],
   *   fonts: ["helvetica"],
   *   modified: "D:20261118120000Z",
   * });
   * ```
   *
   * @throws {@link LaminaPdfError} `content-id-in-use`, `content-glyph-missing` (a character in
   *   none of the fonts), `content-font-not-embeddable` (a font whose licence forbids embedding),
   *   `content-font-invalid`, `appearance-needs-text` (a font's bytes in a module without text
   *   layout, or a character Helvetica, Times and Courier lack), `unknown-blob`, `invalid-op`; as
   *   {@link Document.apply}.
   *
   * @since 0.7.0
   */
  addText(index: number, text: NewText): Edited;

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

  /**
   * Puts a watermark on `pages` (`addWatermark`): text, or an image added with
   * {@link Document.addBlob}, centred on each page and drawn in front of its content, turned by
   * `angle` and faded by `opacity`. It is part of the page, printed and drawn by every reader, and
   * an artifact in a tagged file. With `hideable` a reader's layers panel can switch it off.
   * Change it with {@link DocumentContent.updatePageMark} and take it off with
   * {@link DocumentContent.deletePageMark}, by `id`, which is the same on every page.
   *
   * ```ts
   * doc.content.addWatermark([0, 1, 2], {
   *   id: "draft",
   *   mark: { text: { text: "DRAFT", size: 96, color: [0.8, 0, 0] } },
   *   opacity: 0.3,
   *   angle: 45,
   *   hideable: true,
   *   modified: "D:20261118120000Z",
   * });
   * ```
   *
   * @throws {@link LaminaPdfError} `content-id-in-use`, `content-glyph-missing`,
   *   `content-font-not-embeddable`, `content-font-invalid`, `content-object-invalid` (an image
   *   that isn't one), `appearance-needs-text`, `unknown-blob`, `invalid-op`,
   *   `page-index-out-of-range`; as {@link Document.apply}.
   *
   * @since 0.7.0
   */
  addWatermark(pages: readonly number[], watermark: NewWatermark): Edited;

  /**
   * Puts a background on `pages` (`addBackground`): text, or an image that covers the page, drawn
   * behind each page's content. Otherwise as {@link DocumentContent.addWatermark}.
   *
   * @throws {@link LaminaPdfError} as {@link DocumentContent.addWatermark}.
   *
   * @since 0.7.0
   */
  addBackground(pages: readonly number[], background: NewWatermark): Edited;

  /**
   * Puts a header and footer on `pages` (`setHeaderFooter`): six slots of text with the fields
   * `{page}`, `{pages}`, `{label}`, `{date}` and `{file}`, filled when the edit is applied.
   *
   * ```ts
   * doc.content.setHeaderFooter([0, 1, 2], {
   *   id: "folio",
   *   slots: [null, null, null, null, "Page {page} of {pages}", null],
   *   size: 9,
   *   modified: "D:20261118120000Z",
   * });
   * ```
   *
   * @throws {@link LaminaPdfError} as {@link DocumentContent.addWatermark}.
   *
   * @since 0.7.0
   */
  setHeaderFooter(pages: readonly number[], headerFooter: NewHeaderFooter): Edited;

  /**
   * Numbers `pages` with a Bates number (`applyBates`): `bates.start` on the first page named,
   * one more on each after it, in the order given. Number many files by calling it on each with
   * the `start` the last one reached.
   *
   * @throws {@link LaminaPdfError} as {@link DocumentContent.addWatermark}.
   *
   * @since 0.7.0
   */
  applyBates(pages: readonly number[], bates: NewBates): Edited;

  /**
   * Replaces the mark `id` on `pages` with `mark`, of the same kind (`updatePageMark`), before or
   * after a save. It is drawn again where the old one was drawn.
   *
   * @throws {@link LaminaPdfError} `unknown-page-mark`, `invalid-op` (a mark of another kind); as
   *   {@link DocumentContent.addWatermark}.
   *
   * @since 0.7.0
   */
  updatePageMark(pages: readonly number[], id: string, mark: PageMark, modified: string): Edited;

  /**
   * Takes the mark `id` off `pages` (`deletePageMark`). It stays in the file's earlier revisions
   * until a full save (`stillInEarlierRevisions`).
   *
   * @throws {@link LaminaPdfError} `unknown-page-mark`, `page-index-out-of-range`; as
   *   {@link Document.apply}.
   *
   * @since 0.7.0
   */
  deletePageMark(pages: readonly number[], id: string): Edited;

  /**
   * The page marks on page `index`, in the order they paint, backgrounds first: each with its ID,
   * what it is and whether a reader's layers panel can hide it. Works in every module.
   *
   * @throws {@link LaminaPdfError} `page-index-out-of-range`; reading errors.
   *
   * @since 0.7.0
   */
  pageMarks(index: number): PageMarkInfo[];
}
