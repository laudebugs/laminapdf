// Document: open, read, edit with ops (transactions, undo), save. A thin layer over
// WasmDocument (ADR 0028 F12): every call is synchronous and throws LaminaPdfError. Documents,
// pages and save jobs are handles (internal/handle.ts): a page or save job is freed with its
// document.
import type { WasmDocument, WasmPage, WasmSaveJob } from "@laminapdf/wasm/base";
import type { WasmDocument as ExtractWasmDocument, PageTextJs, WasmExtractJob } from "@laminapdf/wasm/extract";
import { isLaminaPdfError, type LaminaPdfWarning, packageError } from "./errors.js";
import { type ExtractFormat, type ExtractOptions, type ExtractResult, parseExtract } from "./extract.js";
import { ExtractJob, type ExtractJobOptions } from "./extract-job.js";
import { type BaseGlue, extractsText, readingModule } from "./init.js";
import { Handle, watch } from "./internal/handle.js";
import type { Instance } from "./internal/instance.js";
import type { Reflow } from "./reflow.js";
import { type RangeOptions, type RangeSource, type RangeState, settle, wholeBytes } from "./source/range.js";
import { PageText, type TextOptions } from "./text.js";
import {
  type AddedSource,
  type Carry,
  type CropMargins,
  cropPagesOp,
  type DeletePagesOptions,
  deletePagesOp,
  duplicatePagesOp,
  type InsertBlankPagesOptions,
  insertBlankPagesOp,
  insertPagesFromOp,
  type MergeOptions,
  type MergeSource,
  type MetadataEditOptions,
  type MetadataRemoveOptions,
  mergeDocumentsOp,
  movePagesOp,
  type OrientationMethod,
  type PageBoxes,
  type PageOrientation,
  type PagePosition,
  type PageSize,
  type ResizePagesOptions,
  type RotatePagesOptions,
  type Rotation,
  removeMetadataOp,
  removeXmpPropertyOp,
  replacePagesOp,
  replaceXmpPacketOp,
  resizePagesOp,
  reversePagesOp,
  rotatePagesOp,
  setInitialViewOp,
  setMetadataOp,
  setOrientationOp,
  setPageBoxesOp,
  setPageLabelsOp,
  setXmpPropertyOp,
  Transaction,
  type XmpEditOptions,
} from "./transaction.js";
import type {
  AddAnnot,
  AddLink,
  AnnotCommon,
  ArticleThread,
  DocumentInfo,
  DocumentMetadata,
  DocumentProperties,
  Edited,
  ExplicitDestination,
  ExtractedImages,
  ImageOptions,
  InitialView,
  InitialViewPatch,
  MarkupKind,
  MetadataKey,
  MetadataPatch,
  NamedDestination,
  NoteIcon,
  Op,
  OpenOptions,
  OpLog,
  OutlineNode,
  OutlineTarget,
  PageAnnotation,
  PageInfo,
  PageLabelRange,
  PageLink,
  PlannedSave,
  Progress,
  Reverted,
  SaveOptions,
  SaveReport,
  StructureNode,
  Target,
  TargetView,
  Violation,
  XmpText,
  XmpValue,
} from "./types.js";

function isThenable(value: unknown): boolean {
  return (
    (typeof value === "object" || typeof value === "function") &&
    value !== null &&
    typeof (value as { then?: unknown }).then === "function"
  );
}

/** How many warnings one cursor call fetches. */
const WARNING_BATCH = 256;

function bytesOf(input: Uint8Array | ArrayBuffer): Uint8Array {
  return input instanceof Uint8Array ? input : new Uint8Array(input);
}

/**
 * One file {@link Document.merge} brings in: its bytes, its password when it is encrypted, and
 * the name of the bookmark that holds its bookmarks (the file's own title when left out).
 *
 * @since 0.3.0
 */
export interface MergeInput {
  readonly bytes: Uint8Array | ArrayBuffer;
  readonly password?: string;
  readonly title?: string;
}

/**
 * Where {@link Document.split} cuts a document: every `count` pages, at the lists of page IDs in
 * `ranges`, or where the bookmarks at depth `bookmarks` (1: the top level) begin.
 *
 * @since 0.3.0
 */
export type SplitBy =
  | { readonly count: number }
  | { readonly ranges: readonly (readonly string[])[] }
  | { readonly bookmarks: number };

/**
 * The open options to try, in order: as given, then, when the password's NFKC form differs, with
 * that form (tried only after `wrong-password`). R5/R6 passwords are prepared with SASLprep,
 * whose normalisation the WebAssembly modules leave out, while R2–R4 passwords must reach
 * the core unchanged.
 */
function passwordAttempts<O extends OpenOptions>(options: O): O[] {
  const password = options.password;
  const nfkc = password?.normalize("NFKC");
  return password === undefined || nfkc === undefined || nfkc === password
    ? [options]
    : [options, { ...options, password: nfkc }];
}

/** Whether a failed attempt should be followed by the next one. */
function tryNext(e: unknown, attempt: number, attempts: number): boolean {
  return attempt + 1 < attempts && isLaminaPdfError(e, "wrong-password");
}

/**
 * A page of a {@link Document}, held by its ID: moving pages doesn't retarget it.
 * Freed with its document; every call after that throws `disposed`.
 *
 * @since 0.2.0
 */
export class PageRef extends Handle<BaseGlue> {
  readonly #page: WasmPage;

  /** @internal */
  constructor(page: WasmPage, doc: Document, instance: Instance<BaseGlue>) {
    super("PageRef", instance, doc);
    this.#page = page;
  }

  /**
   * The page's stable ID (`p12.0`).
   *
   * @since 0.2.0
   */
  get id(): string {
    return this.call(() => this.#page.id);
  }

  /**
   * The page's current 0-based index. @throws {@link LaminaPdfError} `unknown-page` when it is gone.
   *
   * @since 0.2.0
   */
  get index(): number {
    return this.call(() => this.#page.index());
  }

  /**
   * The page's facts now. @throws {@link LaminaPdfError} `unknown-page` when it is gone.
   *
   * @since 0.2.0
   */
  get info(): PageInfo {
    return this.call(() => this.#page.info());
  }

  /**
   * The page's text: lines in reading order, runs, and every glyph's quad in page space.
   * Needs a document opened while the extract or text module was loaded.
   *
   * @example
   * ```ts
   * await init({ extract: true });
   * const doc = Document.open(bytes);
   * const text = doc.page(0).text().text();
   * ```
   * @param options - `skipInvisible` leaves out invisible text (OCR layers).
   * @throws {@link LaminaPdfError} `extract-module-not-loaded` for a document opened in the base
   *   module; `unknown-page`; reading errors (`data-pending` for a range document: use `ensure`).
   *
   * @since 0.2.0
   */
  text(options?: TextOptions): PageText {
    this.assertLive();
    const page = this.#page as WasmPage & { text?: (options: TextOptions | undefined) => PageTextJs };
    if (typeof page.text !== "function") {
      throw packageError(
        "extract-module-not-loaded",
        "page.text() needs a document opened while the extract or text module was loaded",
      );
    }
    const extract = page.text.bind(page);
    return new PageText(this.call(() => extract(options)));
  }

  protected release(): void {
    this.#page.free();
  }
}

/**
 * A save in progress, stepped by the caller with a budget of work units. Freed with
 * its document; every call after that throws `disposed`.
 *
 * @since 0.2.0
 */
export class SaveJob extends Handle<BaseGlue> {
  readonly #job: WasmSaveJob;

  /** @internal */
  constructor(job: WasmSaveJob, doc: Document, instance: Instance<BaseGlue>) {
    super("SaveJob", instance, doc);
    this.#job = job;
  }

  /**
   * Runs at most `budget` work units.
   *
   * @returns `{ state: "progress", … }`, or `{ state: "done", bytes, report }` at the end.
   * @throws {@link LaminaPdfError} `cancelled`, `document-changed`, `job-finished`, or the save's errors.
   *
   * @since 0.2.0
   */
  step(
    budget: number,
  ):
    | ({ readonly state: "progress" } & Progress)
    | { readonly state: "done"; readonly bytes: Uint8Array<ArrayBuffer>; readonly report: SaveReport } {
    return this.call(() => this.#job.step(budget));
  }

  /**
   * Stops the job: its next step fails with `cancelled`.
   *
   * @since 0.2.0
   */
  cancel(): void {
    this.call(() => this.#job.cancel());
  }

  protected release(): void {
    this.#job.free();
  }
}

/**
 * An open PDF: read it, edit it with ops (atomic transactions with undo), save it. It holds
 * WebAssembly memory: free it with `using` or {@link Document.free} (an alias).
 *
 * @example
 * ```ts
 * await init();
 * using doc = Document.open(bytes);
 * doc.transact((tx) => tx.setInfo({ title: "Final" }));
 * const updated = doc.save({ mode: "incremental" });
 * ```
 *
 * @since 0.2.0
 */
export class Document extends Handle<BaseGlue> {
  readonly #doc: WasmDocument;
  /** Set for a document opened with {@link Document.openRange}: where more bytes come from. */
  readonly #range: RangeState | undefined;
  /** Whether a `transact` callback of this document is running. */
  #transacting = false;

  /**
   * @internal Wraps a raw document (from `open`, `openRange` or `Builder.finish`) of `instance`.
   */
  constructor(doc: WasmDocument, instance: Instance<BaseGlue>, range?: RangeState) {
    super("Document", instance);
    this.#doc = doc;
    this.#range = range;
    watch(this, "Document");
  }

  /**
   * Opens a document whose bytes are fetched range by range ({@link httpRangeSource},
   * {@link callbackSource}): only what the open needs is fetched (the end of the file, the xref,
   * the catalog). Calls on the document stay synchronous; one that needs bytes not fetched yet
   * throws `data-pending` with `ranges`, so wrap later reads in {@link Document.ensure}.
   *
   * @param source - the file (or a promise of it, as `httpRangeSource` returns).
   * @param options - open options (password, strict, limits) and fetch options.
   * @returns the document, once open has everything it needs.
   * @throws {@link LaminaPdfError} as {@link Document.open}; `data-pending` when fetching made no
   *   progress or `maxRounds` ran out; `source-budget-exceeded`; the source's own errors
   *   (`fetch-failed`, `source-changed`); the signal's reason when aborted.
   *
   * @since 0.2.0
   */
  static async openRange(
    source: RangeSource | Promise<RangeSource>,
    options: OpenOptions & RangeOptions = {},
  ): Promise<Document> {
    // The source first: a rejected source promise must not go unhandled behind not-initialized.
    const src = await source;
    const { signal, blockSize, readahead, maxRounds, budget, ...open } = options;
    // A server that ignored Range from the start sent the whole file: open those bytes as they
    // are, rather than copying them into a sparse source as well.
    const whole = wholeBytes(src);
    if (whole !== undefined) return Document.open(whole, open);
    const instance = readingModule();
    const sparse = instance.call((wasm) => new wasm.WasmSparseSource(src.size, budget));
    const state: RangeState = {
      instance,
      generation: instance.generation,
      sparse,
      source: src,
      blockSize: blockSize ?? src.blockSize ?? 4096,
      readahead: readahead ?? 0,
      maxRounds: maxRounds ?? 1024,
    };
    const attempts = passwordAttempts(open);
    try {
      for (let i = 0; ; i++) {
        try {
          return await settle(
            state,
            () =>
              new Document(
                instance.call((wasm) => {
                  instance.check(state.generation, "Document");
                  return wasm.WasmDocument.openSource(sparse, attempts[i]);
                }),
                instance,
                state,
              ),
            signal,
          );
        } catch (e) {
          if (!tryNext(e, i, attempts.length)) throw e;
        }
      }
    } catch (e) {
      // Nothing holds the fetched bytes now: free them at once rather than at collection, unless
      // their instance was replaced (a trap, or reset() during a fetch).
      if (instance.owns(state.generation)) sparse.free();
      throw e;
    }
  }

  /**
   * Runs a synchronous call on this document, fetching what it needs first: for a document
   * from {@link Document.openRange}, a `data-pending` makes it fetch the ranges asked for and
   * call `f` again. For other documents it just returns `f()`.
   *
   * @example
   * ```ts
   * const doc = await Document.openRange(httpRangeSource(url));
   * const pages = await doc.ensure(() => doc.pages);
   * ```
   * @param f - the call (it may run several times; keep it free of side effects). An async
   *   function works too: its `data-pending` rejections are caught the same way.
   * @param options - `signal` aborts this call's fetches (the open's signal applies to the open
   *   only).
   * @returns what `f` returns.
   * @throws what `f` throws, and as {@link Document.openRange}.
   *
   * @since 0.2.0
   */
  async ensure<T>(f: () => T | Promise<T>, options: { readonly signal?: AbortSignal } = {}): Promise<T> {
    this.assertLive();
    return this.#range === undefined ? f() : settle(this.#range, f, options.signal);
  }

  /**
   * Opens a document from its bytes (the bytes are copied into the WASM memory).
   *
   * The password is tried as given. When that fails with `wrong-password` and its NFKC form
   * differs, the NFKC form is tried too: R5/R6 passwords are prepared with SASLprep, whose
   * normalisation the WebAssembly modules leave out, while R2–R4 passwords must reach
   * the core unchanged.
   *
   * @param bytes - the PDF file.
   * @param options - password, strict mode, limits.
   * @returns the document.
   * @throws {@link LaminaPdfError} `not-pdf`, `password-required`, `wrong-password`, `limit-exceeded`,
   *   `strict`, …; `unknown-option` / `invalid-argument` for bad options; `not-initialized`.
   *
   * @since 0.2.0
   */
  static open(bytes: Uint8Array | ArrayBuffer, options?: OpenOptions): Document {
    const instance = readingModule();
    const data = bytesOf(bytes);
    const attempts = passwordAttempts(options ?? {});
    for (let i = 0; ; i++) {
      try {
        return new Document(
          instance.call((wasm) => wasm.WasmDocument.openBytes(data, attempts[i])),
          instance,
        );
      } catch (e) {
        if (!tryNext(e, i, attempts.length)) throw e;
      }
    }
  }

  /**
   * A new document of `inputs` one after another: each file's pages with their annotations, form
   * fields, named destinations, layers and page labels, and its bookmarks as `options.outline`
   * says (by default under a bookmark named after the file: its `title`, else the file's own
   * title). An encrypted file opens with its `password`. Saving it writes a whole file with no
   * password.
   *
   * @throws {@link LaminaPdfError} `wrong-password`, `password-required`, and the errors of a file
   *   that doesn't open.
   *
   * @since 0.3.0
   */
  static merge(inputs: readonly MergeInput[], options?: MergeOptions): Document {
    const instance = readingModule();
    const raw = (nfkc: boolean) =>
      inputs.map((input) => ({
        bytes: bytesOf(input.bytes),
        ...(input.password === undefined ? {} : { password: nfkc ? input.password.normalize("NFKC") : input.password }),
        ...(input.title === undefined ? {} : { title: input.title }),
      }));
    // As `open` does: the passwords as given, then in their NFKC form after `wrong-password`.
    const retry = inputs.some((i) => i.password !== undefined && i.password.normalize("NFKC") !== i.password);
    const attempts = retry ? [false, true] : [false];
    for (let i = 0; ; i++) {
      try {
        const list = raw(attempts[i] ?? false);
        return new Document(
          instance.call((wasm) => wasm.WasmDocument.merge(list, options)),
          instance,
        );
      } catch (e) {
        if (!tryNext(e, i, attempts.length)) throw e;
      }
    }
  }

  /**
   * Whether `page(i).text()` works on this document: it was opened or built while the extract or
   * the text module was loaded. A document opened before the module loaded stays in the base
   * module; save and reopen it to move it.
   *
   * @since 0.2.0
   */
  get canExtractText(): boolean {
    this.assertLive();
    return extractsText(this.#doc);
  }

  /**
   * The number of pages. @throws {@link LaminaPdfError} reading errors.
   *
   * @since 0.2.0
   */
  get pageCount(): number {
    return this.call(() => this.#doc.pageCount());
  }

  /**
   * Every page's facts, in order (one page-tree walk). @throws {@link LaminaPdfError} reading errors.
   *
   * @since 0.2.0
   */
  get pages(): readonly PageInfo[] {
    return this.call(() => this.#doc.pageInfos());
  }

  /**
   * Page `index`'s facts.
   *
   * @throws {@link LaminaPdfError} `page-index-out-of-range`.
   *
   * @since 0.2.0
   */
  pageInfo(index: number): PageInfo {
    return this.call(() => this.#doc.pageInfo(index));
  }

  /**
   * A handle on page `index` that follows the page by its ID.
   *
   * @throws {@link LaminaPdfError} `page-index-out-of-range`.
   *
   * @since 0.2.0
   */
  page(index: number): PageRef {
    return new PageRef(
      this.call(() => this.#doc.page(index)),
      this,
      this.instance,
    );
  }

  /**
   * A handle on the page with `id` (`p12.0`).
   *
   * @throws {@link LaminaPdfError} `invalid-argument`, `unknown-page`.
   *
   * @since 0.2.0
   */
  pageById(id: string): PageRef {
    return new PageRef(
      this.call(() => this.#doc.pageById(id)),
      this,
      this.instance,
    );
  }

  /**
   * The outline (bookmarks): its top-level items in order, each with its children, and where
   * each goes as a page and a view in page space; empty without one. Needs a document opened
   * while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} `extract-module-not-loaded` for a document opened in the base
   *   module; reading errors (`data-pending` for a range document: use `ensure`).
   *
   * @since 0.2.0
   */
  outline(): OutlineNode[] {
    return this.call(() => this.#extracting("doc.outline()").outline());
  }

  /**
   * Where the named destination `name` goes (the names a link or a `#nameddest=` address uses),
   * or `undefined` when the document has none by that name. Needs a document opened while the
   * extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.2.0
   */
  namedDestination(name: string): Target | undefined {
    return this.call(() => this.#extracting("doc.namedDestination()").namedDestination(name));
  }

  /**
   * How the document asks to be shown when it opens: the page and zoom, the panel, the page
   * layout and the window preferences. Needs a document opened while the extract or text module
   * was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.3.0
   */
  initialView(): InitialView {
    return this.call(() => this.#extracting("doc.initialView()").initialView());
  }

  /**
   * The document's article threads in the catalog's order, each with its parts in reading
   * order; empty without any. Needs a document opened while the extract or text module was
   * loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.3.0
   */
  threads(): ArticleThread[] {
    return this.call(() => this.#extracting("doc.threads()").threads());
  }

  /**
   * The images the pages draw, as files: JPEG and JPEG 2000 images as stored, byte for byte, the
   * rest decoded to PNG with their `/Decode`, colour space and mask applied. An image is given
   * once, on the first page that draws it; one that can't be decoded (a JBIG2 image, which the
   * modules don't decode, or an unsupported colour space) is in `skipped`, with why. Needs a
   * document opened while the extract or text module was loaded.
   *
   * @param options - `pages` (0-based) to read; every page by default.
   * @throws {@link LaminaPdfError} `page-index-out-of-range`, and as {@link Document.outline}.
   *
   * @since 0.4.0
   */
  extractImages(options: ImageOptions = {}): ExtractedImages {
    return this.call(() => this.#extracting("doc.extractImages()").extractImages(options));
  }

  /**
   * Every page's visible links as `laminapdf.links/1` JSON: each with its page, its rectangle in
   * page space, its `kind` and where it goes. Needs a document opened while the extract or text
   * module was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.4.0
   */
  linksJson(): string {
    return this.call(() => this.#extracting("doc.linksJson()").linksJson());
  }

  /**
   * Every page's visible links as CSV (RFC 4180), a header row then a row a link. Needs a
   * document opened while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.4.0
   */
  linksCsv(): string {
    return this.call(() => this.#extracting("doc.linksCsv()").linksCsv());
  }

  /**
   * The outline as `laminapdf.outline/1` JSON, its items nested as the outline is. Needs a
   * document opened while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.4.0
   */
  outlineJson(): string {
    return this.call(() => this.#extracting("doc.outlineJson()").outlineJson());
  }

  /**
   * The outline as CSV (RFC 4180), a header row then a row an item in reading order, its depth
   * in `level`. Needs a document opened while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.4.0
   */
  outlineCsv(): string {
    return this.call(() => this.#extracting("doc.outlineCsv()").outlineCsv());
  }

  /**
   * Every named destination, sorted by name, with where each goes. Needs a document opened while
   * the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.3.0
   */
  namedDestinations(): NamedDestination[] {
    return this.call(() => this.#extracting("doc.namedDestinations()").namedDestinations());
  }

  /**
   * A place in the viewer, page `page` (0-based) shown as `view` in page space, as the
   * destination a bookmark or a named destination takes: "set the destination to the current
   * view". Needs a document opened while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} `page-index-out-of-range`; as {@link Document.outline}.
   *
   * @since 0.3.0
   */
  destinationAt(page: number, view: TargetView): ExplicitDestination {
    return this.call(() => this.#extracting("doc.destinationAt()").destinationAt(page, view));
  }

  /**
   * The edit that adds a link on `area` of page `page` (0-based): a box in page space, `[x0, y0,
   * x1, y1]` from the page's top-left, such as one the user drew. The link goes to `target`; set
   * its `contents` (the description screen readers read) before applying it with
   * `{ op: "addLink", args }`. Needs a document opened while the extract or text module was
   * loaded.
   *
   * @throws {@link LaminaPdfError} `invalid-argument`, `page-index-out-of-range`; as
   * {@link Document.outline}.
   *
   * @since 0.3.0
   */
  linkOnArea(page: number, area: readonly [number, number, number, number], target: OutlineTarget): AddLink {
    return this.call(() => this.#extracting("doc.linkOnArea()").linkOnArea(page, [...area], target));
  }

  /**
   * The edit that adds a link on text of page `page`: `lines` are the text's boxes, one per line,
   * as a selection gives them, each `[x0, y0, x1, y1]` in page space. The link takes one area per
   * line, and in a tagged document it joins the structure tree around that text. Needs a document
   * opened while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} `invalid-argument`, `page-index-out-of-range`; as
   * {@link Document.outline}.
   *
   * @since 0.3.0
   */
  linkOnText(
    page: number,
    lines: readonly (readonly [number, number, number, number])[],
    target: OutlineTarget,
  ): AddLink {
    const boxes = lines.map((line) => [...line] as [number, number, number, number]);
    return this.call(() => this.#extracting("doc.linkOnText()").linkOnText(page, boxes, target));
  }

  /**
   * The edit that marks text of page `page` (0-based) with a highlight, underline, strike-out or
   * squiggly line: `lines` are the text's boxes, one per line, as a selection gives them, each
   * `[x0, y0, x1, y1]` in page space. Apply it with `{ op: "addAnnot", args }`. In a tagged
   * document the mark joins the structure tree after its text, and its description (`contents`,
   * what a screen reader reads) is that text; that needs the extract or text module.
   *
   * @throws {@link LaminaPdfError} `invalid-argument`, `invalid-op` (no lines),
   * `page-index-out-of-range`.
   *
   * @since 0.3.0
   */
  markText(page: number, kind: MarkupKind, lines: readonly (readonly [number, number, number, number])[]): AddAnnot {
    const boxes = lines.map((line) => [...line] as [number, number, number, number]);
    return this.call(() => this.#doc.markText(page, kind, boxes));
  }

  /**
   * The edit that puts a sticky note on page `page`: its `icon` (`"note"` by default) with its
   * upper-left corner at `at`, `[x, y]` in page space, and `comment` in the pop-up it opens.
   *
   * @throws {@link LaminaPdfError} `invalid-argument`, `page-index-out-of-range`.
   *
   * @since 0.3.0
   */
  noteAt(page: number, at: readonly [number, number], comment: string, icon?: NoteIcon): AddAnnot {
    return this.call(() => this.#doc.noteAt(page, [...at], comment, icon));
  }

  /**
   * The edit that puts a caret, a mark for where text goes, on page `page`, filling `area`
   * (`[x0, y0, x1, y1]` in page space).
   *
   * @throws {@link LaminaPdfError} `invalid-argument`, `page-index-out-of-range`.
   *
   * @since 0.3.0
   */
  caretAt(page: number, area: readonly [number, number, number, number]): AddAnnot {
    return this.call(() => this.#doc.caretAt(page, [...area]));
  }

  /**
   * Proposes replacing the text in `lines` of page `page` with `replacement`: a strike-out over
   * the text and a caret after it, grouped so they show, move and delete together. The caret
   * holds `replacement`. `common` (author, dates, colour…) goes to both. The two edits are
   * applied at once, as one undo step; the caret's ID is `createdAnnotations[0]`.
   *
   * @throws {@link LaminaPdfError} as {@link Document.markText}; as {@link Document.apply}.
   *
   * @since 0.3.0
   */
  replaceText(
    page: number,
    lines: readonly (readonly [number, number, number, number])[],
    replacement: string,
    common?: AnnotCommon,
  ): Edited {
    this.#notInTransaction("replaceText");
    const boxes = lines.map((line) => [...line] as [number, number, number, number]);
    return this.call(() => this.#doc.replaceText(page, boxes, replacement, common));
  }

  /**
   * What a properties view shows about the document: Info entries and dates, version, pages,
   * size, tagging, encryption and permissions, fonts, signatures (counted, not verified) and
   * whether it holds JavaScript, attachments or a form. It reads every object once, so call it
   * when the view opens. Needs a document opened while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.outline}.
   *
   * @since 0.2.0
   */
  properties(): DocumentProperties {
    return this.call(() => this.#extracting("doc.properties()").properties());
  }

  /**
   * The visible links on page `index` (0-based), in the page's annotation order, each with what
   * following it does. Nothing is run: a link to a `Launch` or `JavaScript` action is reported as
   * `"unsupported"`. Needs a document opened while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} `page-index-out-of-range`; as {@link Document.outline}.
   *
   * @since 0.2.0
   */
  pageLinks(index: number): PageLink[] {
    return this.call(() => this.#extracting("doc.pageLinks()").pageLinks(index));
  }

  /**
   * The annotations on page `index` (0-based), in the order of the page's annotation list, every
   * subtype included. Change one with the `updateAnnot` op (its text, author, flags, or a move by
   * `translate`) and delete one with `deleteAnnot`, which also deletes its pop-up, its replies
   * and the rest of its group. Needs a document opened while the extract or text module was
   * loaded.
   *
   * @throws {@link LaminaPdfError} `page-index-out-of-range`; as {@link Document.outline}.
   *
   * @since 0.3.0
   */
  pageAnnotations(index: number): PageAnnotation[] {
    return this.call(() => this.#extracting("doc.pageAnnotations()").pageAnnotations(index));
  }

  /**
   * The document's text for retrieval and LLMs: Markdown or plain text in reading order, the
   * `laminapdf.extract/1` JSON with a box for every block, or chunks sized for retrieval. Tables
   * drawn with lines come out as tables, and text a reader can't see is left out. Needs a
   * document opened while the extract or text module was loaded.
   *
   * @example
   * ```ts
   * await init({ extract: true });
   * const doc = Document.open(bytes);
   * const markdown = doc.extract({ format: "markdown" });
   * const chunks = doc.extract({ format: "chunks", chunk: { maxTokens: 400 } });
   * ```
   * @returns a string for `"markdown"` and `"text"`, an {@link Extraction} for `"json"`,
   *   {@link Chunk}s for `"chunks"`.
   * @throws {@link LaminaPdfError} `extract-module-not-loaded` for a document opened in the base
   *   module; `page-index-out-of-range`; `unknown-option` (with `didYouMean`) / `invalid-argument`
   *   for bad options; reading errors (`data-pending` for a range document: use `ensure`).
   *
   * @since 0.2.0
   */
  extract<F extends ExtractFormat>(options: ExtractOptions<F>): ExtractResult[F] {
    this.assertLive();
    const doc = this.#doc as WasmDocument & { extract?: (options: ExtractOptions) => string };
    if (typeof doc.extract !== "function") {
      throw packageError(
        "extract-module-not-loaded",
        "doc.extract() needs a document opened while the extract or text module was loaded",
      );
    }
    const extract = doc.extract.bind(doc);
    return parseExtract(
      options.format,
      this.call(() => extract(options)),
    );
  }

  /**
   * An extraction stepped a page at a time, for models the host runs: each step that needs a
   * model's result stops with a `request` (the page and resolution to render), answered with
   * {@link ExtractJob.provide}. {@link extractWithModels} loops one for async models. Needs a
   * document opened while the extract or text module was loaded.
   *
   * @throws {@link LaminaPdfError} as {@link Document.extract}.
   *
   * @since 0.2.0
   */
  extractJob<F extends ExtractFormat>(options: ExtractJobOptions<F>): ExtractJob<F> {
    this.assertLive();
    const doc = this.#doc as WasmDocument & { extractJob?: (options: ExtractJobOptions) => WasmExtractJob };
    if (typeof doc.extractJob !== "function") {
      throw packageError(
        "extract-module-not-loaded",
        "doc.extractJob() needs a document opened while the extract or text module was loaded",
      );
    }
    const extractJob = doc.extractJob.bind(doc);
    return new ExtractJob(
      this.call(() => extractJob(options)),
      options.format,
      this,
      this.instance,
    );
  }

  /**
   * The Info dictionary's text entries (document text). @throws {@link LaminaPdfError} reading errors.
   *
   * @since 0.2.0
   */
  get info(): DocumentInfo {
    return this.call(() => this.#doc.info());
  }

  /**
   * Whether the document says it is tagged (`/MarkInfo /Marked true`); it doesn't check the tree.
   * Needs a document opened while the extract or text module was loaded, as `page.text()` does.
   * @throws {@link LaminaPdfError} `extract-module-not-loaded`; reading errors.
   *
   * @since 0.2.0
   */
  get isTagged(): boolean {
    return this.call(() => this.#extracting("doc.isTagged").isTagged());
  }

  /**
   * The structure tree's top-level elements in reading order, or `undefined` for a document
   * without one. A text run's `mcid` (on {@link PageTextRun}) links it to the element whose
   * `{ type: "content", page, mcid }` kid names it. Needs the extract or text module, as
   * `page.text()` does.
   * @throws {@link LaminaPdfError} `extract-module-not-loaded`; reading errors.
   *
   * @since 0.2.0
   */
  structureTree(): StructureNode[] | undefined {
    return this.call(() => this.#extracting("doc.structureTree()").structureTree());
  }

  /**
   * The document for a reader mode: its blocks in the structure tree's reading order, with
   * headings, list items, quotes, captions and notes, and each figure, formula and table as a
   * crop of its page with its alternate text, never as reflowed text. A document whose tree is
   * missing or can't be trusted (empty, mostly bare spans, too little of the text, or in an order
   * the pages' layout contradicts) comes back with `available: false` and the `reason`. Needs a
   * document opened in the extract module: load it without the text module, since with both
   * loaded documents open in the text module, which leaves reader mode out to keep its size.
   *
   * @example
   * ```ts
   * const reflow = doc.reflow();
   * if (reflow.available) for (const block of reflow.blocks) console.log(block.kind, block.text);
   * ```
   * @throws {@link LaminaPdfError} `extract-module-not-loaded` in the base or text module;
   *   reading errors.
   *
   * @since 0.2.0
   */
  reflow(): Reflow {
    this.assertLive();
    const doc = this.#doc as WasmDocument & { reflow?: () => string };
    if (typeof doc.reflow !== "function") {
      throw packageError(
        "extract-module-not-loaded",
        "doc.reflow() needs a document opened in the extract module: with the text module loaded too, documents open in the text module, which leaves reader mode out",
      );
    }
    const reflow = doc.reflow.bind(doc);
    return JSON.parse(this.call(() => reflow())) as Reflow;
  }

  /** The document as the extract module made it, or `extract-module-not-loaded` naming `what`. */
  #extracting(what: string): ExtractWasmDocument {
    const doc = this.#doc as Partial<ExtractWasmDocument>;
    if (typeof doc.structureTree !== "function" || typeof doc.isTagged !== "function") {
      throw packageError(
        "extract-module-not-loaded",
        `${what} needs a document opened while the extract or text module was loaded`,
      );
    }
    return doc as ExtractWasmDocument;
  }

  /**
   * Every warning so far (build warnings first), each a {@link LaminaPdfWarning}.
   *
   * @since 0.2.0
   */
  get warnings(): readonly LaminaPdfWarning[] {
    return this.call(() => {
      const cursor = this.#doc.warnings();
      try {
        const out: LaminaPdfWarning[] = [];
        for (;;) {
          const batch = cursor.nextBatch(WARNING_BATCH) as LaminaPdfWarning[];
          if (batch.length === 0) return out;
          out.push(...batch);
        }
      } finally {
        cursor.free();
      }
    });
  }

  /**
   * Whether the file is encrypted: it opened with its password, or the empty one. An encrypted
   * document can't be saved incrementally yet (`encrypted-base`).
   *
   * @since 0.2.0
   */
  get isEncrypted(): boolean {
    return this.call(() => this.#doc.isEncrypted());
  }

  /**
   * Whether there are changes no save has written yet.
   *
   * @since 0.2.0
   */
  get hasUnsavedChanges(): boolean {
    return this.call(() => this.#doc.hasUnsavedChanges());
  }

  /**
   * Applies `ops` in order as one transaction: all of them, or none.
   *
   * @returns the pages the edit touched.
   * @throws {@link LaminaPdfError} `unknown-option` / `invalid-argument` for a value that isn't an op;
   *   the first failing op's error (`unknown-page`, …), with nothing applied.
   *
   * @since 0.2.0
   */
  apply(ops: readonly Op[]): Edited {
    this.#notInTransaction("apply");
    return this.call(() => this.#doc.apply(ops as Op[]));
  }

  /**
   * Applies an op log (`{ v: 1, ops }`, as JSON text or an object) as one transaction.
   *
   * @throws {@link LaminaPdfError} `invalid-op-log`, `unsupported-version`; as {@link Document.apply}.
   *
   * @since 0.2.0
   */
  applyLog(log: string | OpLog): Edited {
    this.#notInTransaction("applyLog");
    const json = typeof log === "string" ? log : JSON.stringify(log);
    return this.call(() => this.#doc.applyLog(json));
  }

  /**
   * Runs `edit` with a transaction handle and applies what it collected as one transaction,
   * undone and redone as one step. When `edit` throws, nothing is applied and the error
   * propagates. The ops are applied when `edit` returns: reads inside `edit` see the document
   * before the transaction.
   *
   * @param edit - collects ops on `tx`; synchronous only (a callback returning a promise is
   *   refused with `invalid-argument` and nothing is applied).
   * @returns what `edit` returned.
   * @throws {@link LaminaPdfError} `nested-transaction` when called (or when `apply`, `undo`, `redo`,
   *   `save`… are called) inside a `transact` of the same document; `invalid-argument` for an
   *   async callback; as {@link Document.apply}.
   * @example
   * ```ts
   * doc.transact((tx) => {
   *   tx.setInfo({ title: "Report" });
   *   tx.rotatePage(doc.page(0).id, 90);
   * });
   * ```
   *
   * @since 0.2.0
   */
  transact<T>(edit: (tx: Transaction) => T): T {
    this.assertLive();
    this.#notInTransaction("transact");
    this.#transacting = true;
    const tx = new Transaction();
    let result: T;
    try {
      result = edit(tx);
    } finally {
      this.#transacting = false;
      tx.close();
    }
    if (isThenable(result)) {
      // The ops of an async callback would be applied before its awaits finish: refuse it.
      throw packageError("invalid-argument", "transact takes a synchronous callback; it returned a promise", {
        data: { argument: "edit", detail: "the callback returned a promise; nothing was applied" },
      });
    }
    if (tx.ops.length > 0) this.apply(tx.ops);
    return result;
  }

  /** Throws `nested-transaction` while a `transact` callback of this document runs. */
  #notInTransaction(call: string): void {
    if (this.#transacting) {
      throw packageError("nested-transaction", `${call} was called inside a transaction of the same document`);
    }
  }

  /**
   * The document's metadata (title, authors, subject, keywords, creator and producer tools,
   * dates), read from its Info dictionary and its XMP metadata together, with the fields where
   * they disagree and the PDF/A and PDF/UA levels it claims.
   *
   * @throws {@link LaminaPdfError} reading errors.
   *
   * @since 0.2.0
   */
  metadata(): DocumentMetadata {
    return this.call(() => this.#doc.metadata());
  }

  /**
   * Edits the document's metadata in one transaction, writing its Info dictionary and its XMP
   * metadata together (`setMetadata`). A document without XMP metadata gets it.
   *
   * @param patch - the fields to set (a value) or remove (`null`); authors are a list.
   * @param options - `at`: the modification date to record (a PDF date, from your clock);
   *   `replaceMalformed`: replace XMP metadata that can't be read, losing what it held.
   * @throws {@link LaminaPdfError} `malformed-xmp` when the XMP metadata can't be read and
   *   `replaceMalformed` isn't set; as {@link Document.apply}.
   *
   * @since 0.2.0
   */
  setMetadata(patch: MetadataPatch, options?: MetadataEditOptions): Edited {
    return this.apply([setMetadataOp(patch, options)]);
  }

  /**
   * Changes how the document opens, in one transaction (`setInitialView`): the page and zoom, the
   * panel, the page layout and the window preferences. What the patch leaves out stays as it is;
   * a value newer than the file raises its PDF version.
   *
   * @throws {@link LaminaPdfError} `open-action-not-destination` when `open` would replace an
   *   open action that runs a script or another action; as {@link Document.apply}.
   *
   * @since 0.3.0
   */
  setInitialView(patch: InitialViewPatch): Edited {
    return this.apply([setInitialViewOp(patch)]);
  }

  /**
   * Replaces the document's page labels, the names viewers show for pages ("iv", "A-3"), in one
   * transaction (`setPageLabels`). Each range names its first page by ID and labels the pages up
   * to the next range; the first must be on the first page, the rest follow in page order. No
   * ranges remove the labels. The new labels are in `doc.pages[i].label`.
   *
   * ```ts
   * const [cover, contents, body] = doc.pages;
   * doc.setPageLabels([
   *   { page: cover.id, prefix: "Cover" },
   *   { page: contents.id, style: "lowerRoman" },
   *   { page: body.id, style: "decimal" },
   * ]);
   * ```
   *
   * @throws {@link LaminaPdfError} `invalid-label-ranges` when the ranges break a rule (its
   *   `data.what` says which); `unknown-page` for a page this document doesn't have; as
   *   {@link Document.apply}.
   *
   * @since 0.3.0
   */
  setPageLabels(ranges: readonly PageLabelRange[]): Edited {
    return this.apply([setPageLabelsOp(ranges)]);
  }

  /**
   * Removes metadata fields from the Info dictionary and the XMP metadata (`removeMetadata`).
   * The file's earlier revisions keep the old values until a full rewrite
   * (`save({ mode: "fullRewrite" })`); the result's `stillInEarlierRevisions` says when.
   *
   * @throws {@link LaminaPdfError} as {@link Document.setMetadata}.
   *
   * @since 0.2.0
   */
  removeMetadata(keys: readonly MetadataKey[], options?: MetadataRemoveOptions): Edited {
    return this.apply([removeMetadataOp(keys, options)]);
  }

  /**
   * The document's XMP metadata as text, with the edits made so far: `status` (`absent`,
   * `parsed`, or `malformed` for metadata that can't be read), the packet's `xml`, and `pdfa`, the
   * PDF/A level it claims.
   *
   * @throws {@link LaminaPdfError} reading errors.
   *
   * @since 0.3.0
   */
  xmpPacket(): XmpText {
    return this.call(() => this.#doc.xmpPacket());
  }

  /**
   * Sets any XMP property in one transaction (`setXmpProperty`): text, a URI, a number, a
   * boolean, a date, a language alternative, or a list. A document without XMP metadata gets it.
   * A property the Info dictionary mirrors (`dc:title`, `pdf:Producer`, …) is written to both, as
   * {@link Document.setMetadata} writes it. In a PDF/A file, a property PDF/A doesn't predefine is
   * declared in the metadata's extension schemas, so the file keeps its claim; the result's
   * `declared` lists them.
   *
   * @param namespace - the property's namespace URI (`https://example.com/ns/review/1.0/`).
   * @param prefix - the prefix to write the namespace with when the metadata has none for it.
   * @param name - the property's local name.
   * @param value - the value, tagged with its type: `{ type: "integer", value: 3 }`.
   * @param options - `replaceMalformed`, as {@link Document.setMetadata}.
   * @throws {@link LaminaPdfError} `invalid-xmp-property` when the name or value can't be
   *   written; `xmp-claim-read-only` for the PDF/A or PDF/UA identification; `malformed-xmp`; as
   *   {@link Document.apply}.
   *
   * @since 0.3.0
   */
  setXmpProperty(namespace: string, prefix: string, name: string, value: XmpValue, options?: XmpEditOptions): Edited {
    return this.apply([setXmpPropertyOp(namespace, prefix, name, value, options)]);
  }

  /**
   * Removes an XMP property (`removeXmpProperty`), from the Info dictionary too when it mirrors
   * it. The file's earlier revisions keep the old value until a full rewrite; the result's
   * `stillInEarlierRevisions` says when.
   *
   * @throws {@link LaminaPdfError} as {@link Document.setXmpProperty}.
   *
   * @since 0.3.0
   */
  removeXmpProperty(namespace: string, name: string, options?: XmpEditOptions): Edited {
    return this.apply([removeXmpPropertyOp(namespace, name, options)]);
  }

  /**
   * Replaces the whole XMP packet with `xml` (`replaceXmpPacket`), for what the property edits
   * can't write: structures, qualifiers, comments. The packet is read first; the Info
   * dictionary's mirrored entries follow it, and in a PDF/A file its custom properties are
   * declared (`declared` in the result).
   *
   * @throws {@link LaminaPdfError} `invalid-xmp-packet` (with `reason` and `offset`) when it
   *   doesn't parse; `xmp-claim-read-only` when it changes the PDF/A or PDF/UA identification; as
   *   {@link Document.apply}.
   *
   * @since 0.3.0
   */
  replaceXmpPacket(xml: string): Edited {
    return this.apply([replaceXmpPacketOp(xml)]);
  }

  /**
   * Inserts `count` blank pages of `size` at `at`, in one transaction (`insertBlankPages`). Each
   * new page gets a new ID, listed in the result's `pageOrder`; in a document with page labels
   * the new pages continue the label range they land in.
   *
   * @param at - `"start"`, `"end"`, or `{ before: id }` / `{ after: id }` of a page.
   * @param count - how many pages; 0 changes nothing.
   * @param size - their size in points (`{ width: 612, height: 792 }` is US Letter).
   * @param options - `rotate`: their rotation in degrees clockwise, 0 when left out.
   * @throws {@link LaminaPdfError} `unknown-page` for an anchor that isn't a page of the
   *   document; as {@link Document.apply}.
   *
   * @since 0.3.0
   */
  insertBlankPages(at: PagePosition, count: number, size: PageSize, options?: InsertBlankPagesOptions): Edited {
    return this.apply([insertBlankPagesOp(at, count, size, options)]);
  }

  /**
   * Deletes `pages`, by ID, in one transaction (`deletePages`). The outline items, links, named
   * destinations and open action that pointed at a deleted page point at the nearest page left
   * (the next one, else the one before), or are removed with `references: "remove"`; the
   * result's `references` lists each one. The deleted pages stay in the file's earlier revisions
   * until a full rewrite (`save({ mode: "fullRewrite" })`).
   *
   * @param pages - the pages' IDs, each once.
   * @param options - `references`: `"retarget"` (the default) or `"remove"`.
   * @throws {@link LaminaPdfError} `no-pages-left` when `pages` is every page of the document;
   *   `unknown-page` for an ID that isn't a page of it; as {@link Document.apply}.
   *
   * @since 0.3.0
   */
  deletePages(pages: readonly string[], options?: DeletePagesOptions): Edited {
    return this.apply([deletePagesOp(pages, options)]);
  }

  /**
   * Moves `pages`, by ID, to `to` in one transaction (`movePages`). They take the order given
   * and keep their IDs; an anchor that is itself moved stands for the nearest page that stays.
   *
   * @param pages - the pages' IDs, each once, in the order they take.
   * @param to - `"start"`, `"end"`, or `{ before: id }` / `{ after: id }` of a page.
   * @throws {@link LaminaPdfError} `unknown-page` for an ID that isn't a page of the document; as
   *   {@link Document.apply}.
   *
   * @since 0.3.0
   */
  movePages(pages: readonly string[], to: PagePosition): Edited {
    return this.apply([movePagesOp(pages, to)]);
  }

  /**
   * Rotates `pages`, by ID, in one transaction (`rotatePages`): each shows at `angle`, or with
   * `mode: "turn"` turns by `angle` from the rotation it has.
   *
   * @param angle - degrees clockwise: 0, 90, 180 or 270.
   * @param options - `mode`: `"set"` (the default) or `"turn"`.
   * @throws {@link LaminaPdfError} `unknown-page` for an ID that isn't a page of the document; as
   *   {@link Document.apply}.
   *
   * @since 0.3.0
   */
  rotatePages(pages: readonly string[], angle: Rotation, options?: RotatePagesOptions): Edited {
    return this.apply([rotatePagesOp(pages, angle, options)]);
  }

  /**
   * Reverses the order of `pages`, by ID, among the places they hold, in one transaction
   * (`reversePages`): the first of them takes the last one's place, and so on, and the other
   * pages stay where they are. Pass every page's ID to reverse the whole document.
   *
   * @throws {@link LaminaPdfError} `unknown-page` for an ID that isn't a page of the document; as
   *   {@link Document.apply}.
   *
   * @since 0.3.0
   */
  reversePages(pages: readonly string[]): Edited {
    return this.apply([reversePagesOp(pages)]);
  }

  /**
   * Sets or removes boxes of `pages`, by ID, in one transaction (`setPageBoxes`). Each box is in
   * the page's own user space and must lie inside its media box (the new one, when `boxes` sets
   * it). A smaller crop box only hides content: the result's `croppedContentRemains` lists the
   * pages whose content is now partly hidden but still in the file, where text extraction finds
   * it. To remove it, redact it.
   *
   * @throws {@link LaminaPdfError} `invalid-op` for a box without an area, outside the media box
   *   or with a value that isn't finite; `unknown-page` for an ID that isn't a page of the
   *   document; as {@link Document.apply}.
   *
   * @since 0.4.0
   */
  setPageBoxes(pages: readonly string[], boxes: PageBoxes): Edited {
    return this.apply([setPageBoxesOp(pages, boxes)]);
  }

  /**
   * Crops `pages`, by ID, by `margins` of each page as it shows, in one transaction
   * (`cropPages`): rotation and user unit are taken into account, so one set of margins
   * crops pages of mixed sizes and rotations by the same visible amount. The result's
   * `croppedContentRemains` lists the pages whose cropped-away content is still in the file.
   *
   * @throws {@link LaminaPdfError} `invalid-op` for a negative margin or margins that leave no
   *   page; `unknown-page` for an ID that isn't a page of the document; as {@link Document.apply}.
   *
   * @since 0.4.0
   */
  cropPages(pages: readonly string[], margins: CropMargins): Edited {
    return this.apply([cropPagesOp(pages, margins)]);
  }

  /**
   * Gives `pages`, by ID, new paper of `size` in one transaction (`resizePages`). `size` is the
   * paper before `/Rotate`, as for `insertBlankPages`, so a page turned on its side stays turned.
   * By default the page as shown is scaled to fit and centred; `{ mode: "extend", anchor }`
   * keeps the content's size and places it at `anchor`. The links, notes, form fields, bookmarks,
   * named destinations and opening view that point into the page move with its content.
   *
   * @param options - `mode`: `"scale"` (the default) or `"extend"`; `anchor` for `"extend"`.
   * @throws {@link LaminaPdfError} `invalid-op` for a side not above 0 and at most 14,400 points;
   *   `unsupported-filter` for a page whose content uses a filter Lamina PDF can't decode;
   *   `unknown-page` for an ID that isn't a page of the document; as {@link Document.apply}.
   *
   * @since 0.4.0
   */
  resizePages(pages: readonly string[], size: PageSize, options?: ResizePagesOptions): Edited {
    return this.apply([resizePagesOp(pages, size, options)]);
  }

  /**
   * Shows `pages`, by ID, in `orientation` in one transaction (`setOrientation`): each page shown
   * the other way turns a quarter clockwise, and the rest, square ones included, stay as they
   * are. `"rotate"` (the default) turns it with `/Rotate`; `"swapBox"` swaps the paper's sides
   * and turns the content, annotations and destinations into it, for printers and tools that
   * ignore `/Rotate`.
   *
   * @throws {@link LaminaPdfError} as {@link Document.resizePages}.
   *
   * @since 0.4.0
   */
  setOrientation(pages: readonly string[], orientation: PageOrientation, method?: OrientationMethod): Edited {
    return this.apply([setOrientationOp(pages, orientation, method)]);
  }

  /**
   * Copies `pages`, by ID and in the order given, to `at` (`"start"`, `"end"`,
   * `{ before: id }` or `{ after: id }`), in one transaction (`duplicatePages`). A copy shares
   * its page's content and gets new IDs for itself and its annotations, listed in the result's
   * `pageOrder`. A form field's widget becomes a new field named `<name>#2` (or the next free
   * number) with its own value, and a signature's widget is not copied; the result's `fields`
   * lists both. In a tagged document the copy gets its own tags, except for content of
   * elements that run on to other pages, which stays untagged: the result's `tagsIncomplete`
   * lists those copies.
   *
   * @throws {@link LaminaPdfError} `unknown-page` for an ID that isn't a page of the document; as
   *   {@link Document.apply}.
   *
   * @since 0.3.0
   */
  duplicatePages(pages: readonly string[], at: PagePosition): Edited {
    return this.apply([duplicatePagesOp(pages, at)]);
  }

  /**
   * Adds another PDF as a source of pages for {@link Document.insertPagesFrom} and
   * {@link Document.replacePages}: its ID (the SHA-256 of its bytes, so the same file gets the
   * same ID everywhere and is added once) and its page IDs in order, which those ops name its
   * pages by. An encrypted file opens with `options.password`, which no op ever holds. Adding a
   * source changes nothing to save or undo.
   *
   * @throws {@link LaminaPdfError} `wrong-password`, `password-required`, and the errors of a file
   *   that doesn't open.
   *
   * @since 0.3.0
   */
  addSource(bytes: Uint8Array | ArrayBuffer, options?: { readonly password?: string }): AddedSource {
    const data = bytesOf(bytes);
    return this.call(() => this.#doc.addSource(data, options?.password));
  }

  /**
   * Copies `pages` of the source document `source` (IDs from {@link Document.addSource}), each
   * once and in the order given, to `at`, in one transaction (`insertPagesFrom`). The copies are
   * new pages with new IDs, listed in the result's `pageOrder`. Their annotations come along
   * unless `carry.annotations` is `false`; a link to a page left behind is removed (the result's
   * `linksRemoved`), and the named destinations the other links use come along. A form field
   * keeps its name, or takes the next free `<name>#<n>` when the name is taken (`carry.fields`
   * `"link"` shares the value instead, where it can); a signature's widget is left off. The
   * result's `fields` lists each. In a tagged document the copies keep their tags, with any
   * role, class or element ID already used another way renamed (`structureRenamed`); in an
   * untagged one they come without them (`tagsDropped`).
   *
   * @throws {@link LaminaPdfError} `unknown-source`, `unknown-page`; as {@link Document.apply}.
   *
   * @since 0.3.0
   */
  insertPagesFrom(source: string, pages: readonly string[], at: PagePosition, carry?: Carry): Edited {
    return this.apply([insertPagesFromOp(source, pages, at, carry)]);
  }

  /**
   * Replaces `targets` with `pages` of the source document `source`, pair by pair, in one
   * transaction (`replacePages`): bookmarks, links and named destinations that pointed at a
   * replaced page point at the page that takes its place. Targets left over are deleted, and
   * pages left over follow the last copy. The copies come in as with
   * {@link Document.insertPagesFrom}.
   *
   * @throws {@link LaminaPdfError} `unknown-source`, `unknown-page`, `no-pages-left`; as
   *   {@link Document.apply}.
   *
   * @since 0.3.0
   */
  replacePages(targets: readonly string[], source: string, pages: readonly string[], carry?: Carry): Edited {
    return this.apply([replacePagesOp(targets, source, pages, carry)]);
  }

  /**
   * Brings in every page of each of `sources` (IDs from {@link Document.addSource}, or a
   * `{ source, pages, title }` to choose pages or name the document) at `at`, one source after
   * another, in one transaction (`mergeDocuments`). Pages come in as with
   * {@link Document.insertPagesFrom}, and so do the documents' bookmarks (as `options.outline`
   * says), named destinations (renamed `<name>#<n>` when taken: the result's
   * `renamedDestinations`), layers and page labels. Bookmarks to pages left behind are dropped
   * (`bookmarksDropped`).
   *
   * @throws {@link LaminaPdfError} `unknown-source`, `unknown-page`; as {@link Document.apply}.
   *
   * @since 0.3.0
   */
  mergeDocuments(sources: readonly (string | MergeSource)[], at: PagePosition, options?: MergeOptions): Edited {
    const list = sources.map((s) => (typeof s === "string" ? { source: s } : s));
    return this.apply([mergeDocumentsOp(list, at, options)]);
  }

  /**
   * A new document of `pages` of this one, each once and in the order given, as this document
   * stands now: the pages with their annotations, the links between them, their form fields
   * (signature fields unsigned), their page labels, and their tags when this document is
   * tagged. Links to pages left behind are removed. Saving it writes a whole file holding only
   * what its pages use, with no password.
   *
   * @throws {@link LaminaPdfError} `unknown-page`; `no-pages-left` for no pages.
   *
   * @since 0.3.0
   */
  extractPages(pages: readonly string[]): Document {
    const raw = this.call(() => this.#doc.extractPages([...pages]));
    return new Document(raw, this.instance);
  }

  /**
   * One new document per range of `ranges` (each a list of page IDs, as
   * {@link Document.extractPages} takes), in order.
   *
   * @throws {@link LaminaPdfError} as {@link Document.extractPages}.
   *
   * @since 0.3.0
   */
  exportRanges(ranges: readonly (readonly string[])[]): Document[] {
    const parts: Document[] = [];
    try {
      for (const pages of ranges) parts.push(this.extractPages(pages));
    } catch (e) {
      // The documents made before the failure have no other owner: free them now.
      for (const part of parts) part.free();
      throw e;
    }
    return parts;
  }

  /**
   * This document cut into new documents: `{ count }` pages each (the last holds what is left),
   * one per list of page IDs in `{ ranges }`, or one from each bookmark at depth
   * `{ bookmarks }` (1: the top level) to the next, the pages before the first making a part of
   * their own. Each part is made as {@link Document.extractPages} makes one, with the bookmarks
   * that go to its pages.
   *
   * @throws {@link LaminaPdfError} as {@link Document.extractPages}.
   *
   * @since 0.3.0
   */
  split(by: SplitBy): Document[] {
    const raws = this.call(() => this.#doc.split(by));
    return raws.map((raw) => new Document(raw, this.instance));
  }

  /**
   * How a save with `options` would go, without saving: the save strip's constraint, the
   * signatures and the edits they forbid, the mode or the refusal code, and the
   * conformance warnings. A save that would be refused is a plan with a `refusal`, not an error.
   *
   * @throws {@link LaminaPdfError} `unknown-option` / `invalid-argument` for bad options; reading errors.
   *
   * @since 0.2.0
   */
  plan(options?: SaveOptions): PlannedSave {
    return this.call(() => this.#doc.plan(options));
  }

  /**
   * The signature restrictions `op` would break if it were applied and saved: for warning
   * before an edit (a metadata change on a certified file, for example).
   *
   * @throws {@link LaminaPdfError} `invalid-argument` for a value that isn't an op; reading errors.
   *
   * @since 0.2.0
   */
  wouldViolate(op: Op): Violation[] {
    return this.call(() => this.#doc.wouldViolate(op));
  }

  /**
   * Undoes the newest transaction.
   *
   * @returns how it went (`outcome`) and the pages it touched (`touchedPages`, to draw again), or
   *   `undefined` when there is nothing to undo.
   * @throws {@link LaminaPdfError} `saved-not-rebased`; reading errors.
   *
   * @since 0.2.0
   */
  undo(): Reverted | undefined {
    this.#notInTransaction("undo");
    return this.call(() => this.#doc.undo());
  }

  /**
   * Redoes the newest undone transaction.
   *
   * @returns how it went (`outcome`) and the pages it touched (`touchedPages`), or `undefined`
   *   when there is nothing to redo.
   * @throws {@link LaminaPdfError} as {@link Document.undo}.
   *
   * @since 0.2.0
   */
  redo(): Reverted | undefined {
    this.#notInTransaction("redo");
    return this.call(() => this.#doc.redo());
  }

  /**
   * Saves and rebases onto the output (later edits and saves build on it).
   *
   * @param options - `mode` (`auto`, `incremental`, `fullRewrite`), overrides, output form.
   * @returns the file's bytes.
   * @throws {@link LaminaPdfError} `would-invalidate-signatures`, `requires-rewrite`, `encrypted-base`, …
   *
   * @since 0.2.0
   */
  save(options?: SaveOptions): Uint8Array<ArrayBuffer> {
    return this.saveWithReport(options).bytes;
  }

  /**
   * As {@link Document.save}, with the save's report (mode used, bytes written, renumbering, …).
   *
   * @throws {@link LaminaPdfError} as {@link Document.save}.
   *
   * @since 0.2.0
   */
  saveWithReport(options?: SaveOptions): { readonly bytes: Uint8Array<ArrayBuffer>; readonly report: SaveReport } {
    this.#notInTransaction("save");
    return this.call(() => this.#doc.save(options));
  }

  /**
   * Writes what {@link Document.save} would, and leaves the document as it was: not rebased, its
   * changes still unsaved ({@link Document.hasUnsavedChanges}), the save-time stamp in the copy
   * only. For Save a copy, and for a copy to draw the pages from while edits stay unsaved.
   *
   * @throws {@link LaminaPdfError} as {@link Document.save}; the document is unchanged either way.
   *
   * @since 0.2.0
   */
  saveCopy(options?: SaveOptions): { readonly bytes: Uint8Array<ArrayBuffer>; readonly report: SaveReport } {
    this.#notInTransaction("saveCopy");
    return this.call(() => this.#doc.saveCopy(options));
  }

  /**
   * Makes `bytes`, the whole output of {@link Document.saveCopy}, the document's saved file (it
   * rebases onto them), once they are written where the document lives, so a failed write leaves
   * the document unsaved. `historyCleared` is the copy's report's. Nothing may change the document
   * in between.
   *
   * @throws {@link LaminaPdfError} `saved-not-rebased`; `not-rebased` when `bytes` can't be read.
   *
   * @since 0.2.0
   */
  adoptCopy(bytes: Uint8Array, historyCleared: boolean): void {
    this.#notInTransaction("adoptCopy");
    this.call(() => this.#doc.adoptCopy(bytes, historyCleared));
  }

  /**
   * A save as a job stepped by the caller (to keep a UI responsive).
   *
   * @throws {@link LaminaPdfError} `unknown-option` / `invalid-argument` for bad options.
   *
   * @since 0.2.0
   */
  saveJob(options?: SaveOptions): SaveJob {
    this.#notInTransaction("saveJob");
    return new SaveJob(
      this.call(() => this.#doc.saveJob(options)),
      this,
      this.instance,
    );
  }

  /**
   * Rebases onto `bytes`, the output of a save whose rebase failed (`not-rebased`).
   *
   * @throws {@link LaminaPdfError} `no-pending-save`; reading errors in `bytes`.
   *
   * @since 0.2.0
   */
  rebase(bytes: Uint8Array): void {
    this.#notInTransaction("rebase");
    this.call(() => this.#doc.rebase(bytes));
  }

  protected release(): void {
    this.#doc.free();
    // A range document's fetched bytes live in its sparse source.
    this.#range?.sparse.free();
  }
}
