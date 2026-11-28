// The page side of the document worker (ADR 0009 §3): a ViewerClient owns one worker for one
// document, through `@laminapdf/sdk/worker`'s client (`request` with a priority and a coalescing
// key). On top of it:
//
// - typed calls (`call(method, params)`, checked against `Methods`), and `open`/`render`/…;
// - cancellation that rejects with an `AbortError` at once; a late answer's bitmap is closed by
//   the worker client, so a cancelled render never paints;
// - render timers on the page, started when the worker reports it began drawing (a render waiting
//   in the queue isn't slow) and kept on the page because a worker stuck in a synchronous render
//   can't post: `render-slow` after the soft timeout, and after the hard cap either a restart
//   (`hard-timeout`) or, for an encrypted document, `render-stuck` and a restart only on `stop()`;
// - restart after a crash (the worker's error, messageerror or a reported trap; never silence):
//   pending calls reject with `worker-crashed`, a new worker reopens the document from the same
//   source, and renders that were waiting are sent again once, there; the render that was running
//   is never re-sent, so a page that crashes the engine can't loop (the UI offers Retry). An encrypted
//   document isn't reopened: the password lived only in the dead worker, so `password-required`
//   fires and the caller opens again with the password.

import type { RenderRequest } from "@laminapdf/render";
import {
  type AddedObject,
  type AnnotFont,
  type AttachmentFile,
  type Collection,
  type CommentThread,
  type ContentsPlacement,
  type ContentTextRun,
  type DocumentAttachment,
  type DocumentMetadata,
  type ExportCommentsOptions,
  type FormDataFormat,
  type ImportCommentsOptions,
  type ImportSummary,
  isLaminaPdfError,
  type Op,
  type PageBoxesRead,
  type PageLabelRange,
  type PageMarkInfo,
  type PageObject,
  type PlannedSave,
  type SaveOptions,
  type TextEditReport,
  type TextRunRef,
  type Violation,
  type XmpText,
} from "@laminapdf/sdk";
import {
  connect,
  Emitter,
  type WorkerClient,
  type WorkerInitOptions,
  type WorkerWasmSource,
} from "@laminapdf/sdk/worker";
import { withSummary } from "./edit/with-summary.js";
import { EditLog } from "./edit-log.js";
import type { FindOptions } from "./find/fold.js";
import { FirstDraw } from "./first-draw.js";
import {
  type AddedSource,
  type BackendInit,
  type CommentSummaryRequest,
  type ContentsPreview,
  type EditedCopy,
  type EditHistory,
  type EditResult,
  type ExportPageOptions,
  type ExportTiffOptions,
  type ExtractPicturesOptions,
  type ExtractTextOptions,
  type FormImportPlan,
  type ImagesToPdfOptions,
  type ImageToPdf,
  type ImportedComments,
  type InspectionTab,
  type LoadProgress,
  type MadeSummary,
  type MethodName,
  type NetworkRecord,
  type OpenedDocument,
  type Params,
  type PolicyViolation,
  PRIORITY,
  type PrintOptions,
  type Priority,
  type RenderedBitmap,
  type Result,
  renderKey,
  type SavedCopy,
  type SourceInit,
  type ViewerOpenOptions,
  type WorkerNetworkLog,
} from "./protocol.js";

/** Default render timeouts (ADR 0009 §3.7), in ms. */
const DEFAULT_TIMEOUTS = Object.freeze({ soft: 10_000, hard: 120_000 });

/** `ViewerClient.create`'s options: the backend, Lamina PDF's WebAssembly, the timeouts, the worker and a signal to stop starting. */
export interface ViewerOptions {
  /** The render engine; PDFium needs its `.wasm` URL from your bundler (`pdfium.wasm?url`). */
  readonly backend: BackendInit;
  /**
   * Where the worker loads Lamina PDF's own WebAssembly from (`@laminapdf/sdk/worker`'s `init`).
   * The worker loads the extract module itself, for the text layer, once the backend is ready.
   */
  readonly lamina?: WorkerWasmSource | WorkerInitOptions;
  /** Render timeouts in ms (`soft` emits `render-slow`; `hard` restarts or emits `render-stuck`). */
  readonly timeouts?: { readonly soft?: number; readonly hard?: number };
  /** Makes the worker (CSP, or your own hosting of `dist/worker/entry.js`). */
  readonly worker?: () => Worker;
  /** Aborts starting. */
  readonly signal?: AbortSignal;
}

/** Why the worker was restarted. */
export type RestartReason = "error" | "messageerror" | "trap" | "cancelled" | "hard-timeout";

/** The client's events: from the worker (`warning`) or raised on the page (the rest). */
export interface ViewerEvents {
  /** An ADR 0003 warning about the open document (a repaired xref, …). */
  warning: { readonly code: string; readonly message: string };
  /** How much of a remote document the worker has fetched (bytes; the size). */
  progress: LoadProgress;
  /** The worker started drawing a render (so a performance test can see what the worker is busy with). */
  "render-start": { readonly page: number; readonly scale: number; readonly key: string };
  /** A remote document's render stopped drawing to fetch bytes it needs; `render-start` follows when it draws again. */
  "render-fetching": { readonly key: string };
  /** The document worker made a network request (`worker` names the worker, so a restart's records stay apart). */
  network: { readonly worker: string; readonly index: number; readonly record: NetworkRecord };
  /** The worker's Content Security Policy blocked a request it tried. */
  "csp-violation": { readonly worker: string; readonly index: number; readonly violation: PolicyViolation };
  /** A render passed the soft timeout: offer Stop. */
  "render-slow": { readonly id: number; readonly page: number };
  /** An encrypted document's render passed the hard cap: `stop()` would ask for the password again. */
  "render-stuck": { readonly id: number; readonly page: number; readonly needsPassword: true };
  /** The worker crashed (not on `stop()`). */
  crashed: { readonly reason: string };
  /**
   * A new worker serves again (and has reopened the document, unless it needed a password).
   * `edits` says what became of the edit session's steps: `restored` (applied again, the same
   * history), `lost` (the file is as it was opened), or absent when there were none.
   */
  restarted: { readonly reason: RestartReason; readonly reopened: boolean; readonly edits?: "restored" | "lost" };
  /**
   * An edit, undo or redo was made (the document's semantic dirty state is `history.unsaved`):
   * the pages it touched, as they are now, and the edit session's history.
   */
  edit: EditResult;
  /** The restarted worker can't reopen the document without its password: open it again. */
  "password-required": Record<string, never>;
  /** A restart failed (the backend couldn't load again, …): the client serves nothing until `stop()` works. */
  "restart-failed": { readonly reason: RestartReason; readonly error: unknown };
}

/** The names of the client's events. */
export type ViewerEvent = keyof ViewerEvents;

/** Per-call options. */
export interface ViewerCallOptions {
  readonly signal?: AbortSignal;
  /** Default `visible` for renders; document calls run above every render. */
  readonly priority?: Priority;
}

/**
 * What a `Viewer` needs of the engine it draws from: the open document, its events, renders,
 * and the text, link, find, edit and save calls of the layers and the edit session it builds. A
 * `ViewerClient` over its document worker is one; the desktop app's client over its engine
 * process is another, so the same viewer runs on both.
 */
export type ViewerEngine = Pick<
  ViewerClient,
  | "document"
  | "on"
  | "render"
  | "pageText"
  | "readingOrder"
  | "structure"
  | "pageLinks"
  | "pageAnnotations"
  | "find"
  | "applyEdit"
  | "createContentsPage"
  | "importComments"
  | "undoEdit"
  | "redoEdit"
  | "goToStep"
  | "editHistory"
  | "planSave"
  | "wouldViolate"
  | "copyEdits"
  | "copyWithEdit"
  | "adoptEdits"
>;

/** One pending render's soft and hard timers. */
interface RenderTimers {
  readonly start: () => void;
  readonly pause: () => void;
}

function workerFactory(): Worker {
  return new Worker(new URL("./worker/entry.js", import.meta.url), { type: "module", name: "lamina-viewer" });
}

function abortError(e: unknown): unknown {
  return isLaminaPdfError(e, "cancelled") ? new DOMException("the call was cancelled", "AbortError") : e;
}

/**
 * PDFium's module compiled on the page from its address, while the worker starts (compiling
 * streams on the browser's own threads); the backend as given when it names no address, or when
 * fetching or compiling fails, so the worker loads it as before.
 */
function compileEarly(backend: BackendInit): Promise<BackendInit> {
  const { wasm } = backend;
  if (backend.backend !== "pdfium" || typeof wasm !== "string" || typeof WebAssembly.compileStreaming !== "function") {
    return Promise.resolve(backend);
  }
  return WebAssembly.compileStreaming(fetch(wasm, { credentials: "same-origin" })).then(
    (module): BackendInit => ({ ...backend, wasm: module }),
    () => backend,
  );
}

/** One document worker and the typed RPC to it. */
export class ViewerClient {
  readonly #pdf: WorkerClient;
  readonly #backend: BackendInit;
  readonly #timeouts: { readonly soft: number; readonly hard: number };
  readonly #events = new Emitter<ViewerEvents>();
  /** What to reopen after a restart: the source and the options without the password. */
  #reopen: { readonly source: SourceInit; readonly options: Omit<ViewerOpenOptions, "password"> } | undefined;
  /** What `editInTextCopy` asked of the open document, for a restarted worker to ask again. */
  #textCopy: { readonly text: boolean } | undefined;
  #document: OpenedDocument | undefined;
  /** The open document's edit session, kept so a restarted worker takes it back. */
  readonly #edits = new EditLog();
  /** The last copy a save wrote (`copyEdits`), until it is adopted or another replaces it. */
  #copy: { readonly token: number; readonly bytes: Uint8Array } | undefined;
  /** Opens asked for so far: each open's ticket. */
  #opens = 0;
  #recovering: Promise<void> | undefined;
  #nextRender = 1;
  /** The open document's gate for text and link calls (first-draw.ts); none before a document opens. */
  #firstDraw: FirstDraw | undefined;
  /** Each pending render's timers, by coalescing key: started on `render-start`, paused on `render-fetching`. */
  readonly #timers = new Map<string, Set<RenderTimers>>();
  /** The key the worker is drawing now: a render that joins it starts its timers at once. */
  #drawing: string | undefined;
  /** The render that was drawing when the worker crashed or was stopped: it isn't sent again. */
  #lost: string | undefined;

  private constructor(pdf: WorkerClient, options: ViewerOptions) {
    this.#pdf = pdf;
    this.#backend = options.backend;
    this.#timeouts = { ...DEFAULT_TIMEOUTS, ...options.timeouts };
    pdf.on("event", (name, data) => this.#onEvent(name, data));
    pdf.on("crashed", (reason) => {
      this.#events.emit("crashed", { reason });
      this.#recover(reason as RestartReason).catch((error) =>
        this.#events.emit("restart-failed", { reason: reason as RestartReason, error }),
      );
    });
  }

  /** A worker event: forwarded, and for renders, their timers started or paused. */
  #onEvent(name: string, data: unknown): void {
    if (name === "warning") this.#events.emit("warning", data as ViewerEvents["warning"]);
    else if (name === "progress") this.#events.emit("progress", data as LoadProgress);
    else if (name === "network") this.#events.emit("network", data as ViewerEvents["network"]);
    else if (name === "csp-violation") this.#events.emit("csp-violation", data as ViewerEvents["csp-violation"]);
    else if (name === "render-start") {
      const started = data as ViewerEvents["render-start"];
      this.#drawing = started.key;
      for (const timers of this.#timers.get(started.key) ?? []) timers.start();
      this.#events.emit("render-start", started);
    } else if (name === "render-fetching") {
      // Fetching isn't drawing: a slow network must not count as a stuck render.
      const { key } = data as ViewerEvents["render-fetching"];
      if (this.#drawing === key) this.#drawing = undefined;
      for (const timers of this.#timers.get(key) ?? []) timers.pause();
      this.#events.emit("render-fetching", { key });
    }
  }

  /**
   * Starts the document worker and loads the backend at once, so the WebAssembly downloads while
   * the user picks a file.
   */
  static async create(options: ViewerOptions): Promise<ViewerClient> {
    // PDFium downloads and compiles here while the worker starts and loads Lamina PDF's own
    // module, rather than after it: the two downloads overlap.
    const early = compileEarly(options.backend);
    const pdf = await connect({
      worker: options.worker ?? workerFactory,
      // A late answer to an aborted render: its bitmap is closed, never painted.
      onDiscard: (value) => {
        const bitmap = (value as { bitmap?: unknown } | undefined)?.bitmap;
        if (bitmap instanceof ImageBitmap) bitmap.close();
      },
      ...(options.signal === undefined ? {} : { signal: options.signal }),
      ...(options.lamina === undefined ? {} : { init: options.lamina }),
    });
    const client = new ViewerClient(pdf, options);
    try {
      const backend = await early;
      try {
        await pdf.request("initBackend", [backend], { priority: PRIORITY.document });
      } catch (e) {
        // A browser that can't post a compiled module to a worker: the worker loads its own.
        if (backend === options.backend) throw e;
        await pdf.request("initBackend", [options.backend], { priority: PRIORITY.document });
      }
    } catch (e) {
      // Nobody will hold this client: end its worker.
      pdf.terminate();
      throw e;
    }
    return client;
  }

  /** Listens for `event`; returns a function that stops listening. */
  on<E extends ViewerEvent>(event: E, listener: (data: ViewerEvents[E]) => void): () => void {
    return this.#events.on(event, listener);
  }

  /** A typed call: `method` and `params` are checked against `Methods`. */
  async call<M extends MethodName>(
    method: M,
    params: Params<M>,
    options: ViewerCallOptions & { key?: string; onProgress?: (progress: unknown) => void } = {},
  ): Promise<Result<M>> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const { signal, priority, key, onProgress } = options;
    // Renders, and calls that name a priority (text, at `background`), queue with the tiles; other
    // document calls run first.
    const level = method === "render" || priority !== undefined ? PRIORITY[priority ?? "visible"] : PRIORITY.document;
    try {
      return (await this.#pdf.request(method, params, {
        priority: level,
        ...(signal === undefined ? {} : { signal }),
        ...(key === undefined ? {} : { key }),
        ...(onProgress === undefined ? {} : { onProgress }),
      })) as Result<M>;
    } catch (e) {
      throw abortError(e);
    }
  }

  /**
   * Opens `source` in the worker (Lamina PDF and the backend, from the same bytes). The password is
   * sent once and kept only in the worker; a Blob is sent by reference, bytes are copied (and
   * kept here, so a restarted worker can reopen them).
   */
  async open(
    source: SourceInit,
    options: ViewerOpenOptions = {},
    call: ViewerCallOptions = {},
  ): Promise<OpenedDocument> {
    const { password: _, ...rest } = options;
    const ticket = ++this.#opens;
    let opened: OpenedDocument;
    try {
      opened = await this.call("openDocument", [source, options, ticket], call);
    } catch (e) {
      // A cancel can cross the answer of an open the worker had already committed: then the
      // cancel came too late, and this page takes the document the worker now holds.
      const landed = call.signal?.aborted === true ? await this.#committed(ticket) : undefined;
      if (landed === undefined) throw e;
      opened = landed;
    }
    this.#reopen = { source, options: rest };
    this.#textCopy = undefined;
    this.#document = opened;
    this.#edits.reset();
    // Only once it opened: a failed open leaves the shown document's gate as it was.
    this.#firstDraw = new FirstDraw(opened.id);
    return opened;
  }

  /** The worker's document, if open `ticket` made it (the worker runs one call at a time, so it answers after that open). */
  async #committed(ticket: number): Promise<OpenedDocument | undefined> {
    const now = await this.call("currentDocument", []).catch(() => undefined);
    return now?.ticket === ticket ? now.document : undefined;
  }

  /** The open document, if any. */
  get document(): OpenedDocument | undefined {
    return this.#document;
  }

  /** Closes the document in the worker (the worker stays, ready for another `open`). */
  async close(): Promise<void> {
    this.#reopen = undefined;
    this.#textCopy = undefined;
    this.#document = undefined;
    this.#edits.reset();
    await this.call("closeDocument", []);
  }

  /**
   * Applies `ops` to the open document as one transaction (`@laminapdf/sdk`'s ops, such as
   * `rotatePage`), and draws the pages it touched from the edited file from then on. The edit
   * session records it as one step named `label` ("Rotate page 5"), which undo takes back; an
   * edit that changed nothing records none. Resolves with those pages as they now are and the
   * history; `document` gives their new sizes too. Pass the pages to `viewer.invalidate`, or call
   * `viewer.applyEdit`, which does both. A failed edit changes nothing. A restarted worker opens
   * the file and applies the session's steps again (`restarted`'s `edits`).
   */
  applyEdit(ops: readonly Op[], options?: ViewerCallOptions & { readonly label?: string }): Promise<EditResult> {
    const label = options?.label;
    return this.#edit((id) => this.call("applyEdit", label === undefined ? [ops, id] : [ops, id, label], options), {
      ops,
      label: label ?? "",
    });
  }

  /**
   * Inserts a contents page made from the open document's bookmarks, laid out in `font` (TrueType
   * or OpenType bytes, embedded and subset): one step named `label` ("Create a contents page"),
   * which undo takes back with its links and bookmark. The worker loads the text module for it.
   * A restarted worker inserts the same page again (`restarted`'s `edits`).
   *
   * @throws `no-bookmarks`, `missing-glyph`, `invalid-font`; as `applyEdit`.
   */
  createContentsPage(
    placement: ContentsPlacement,
    font: Uint8Array,
    options?: ViewerCallOptions & { readonly label?: string },
  ): Promise<EditResult> {
    const label = options?.label;
    return this.#edit(
      (id) =>
        this.call(
          "createContentsPage",
          label === undefined ? [placement, font, id] : [placement, font, id, label],
          options,
        ),
      { ops: [], label: label ?? "" },
    );
  }

  /**
   * Imports the XFDF or FDF file `bytes` into the open document as one step named `label` (undo
   * takes it back whole), as `options` say. The result's `summary` is what it added, matched and
   * left out. A restarted worker imports the same file again (`restarted`'s `edits`).
   *
   * @throws `interchange-malformed`, `interchange-unknown-format`, `interchange-dtd`,
   *   `interchange-too-large`; as `applyEdit`.
   */
  async importComments(
    bytes: Uint8Array,
    options: ImportCommentsOptions,
    call: ViewerCallOptions & { readonly label: string },
  ): Promise<ImportedComments> {
    const { label } = call;
    return withSummary((pass) =>
      this.#edit(async (id) => pass(await this.call("importComments", [bytes, options, id, label], call)), {
        ops: [],
        label,
      }),
    );
  }

  /** Resolves once a worker restart in progress is over, failed or not. */
  async #settled(): Promise<void> {
    await this.#recovering?.catch(() => {});
  }

  /**
   * What importing the XFDF or FDF file `bytes` would do, as `options` say; changes nothing.
   *
   * @throws as {@link importComments}.
   */
  async previewImport(
    bytes: Uint8Array,
    options: ImportCommentsOptions,
    call?: ViewerCallOptions,
  ): Promise<ImportSummary> {
    await this.#settled();
    return this.call("previewImport", [bytes, options, this.#document?.id ?? 0], call);
  }

  /** The open document's comments as XFDF, FDF or Markdown, as `options` say; the PDF doesn't change. */
  async exportComments(options: ExportCommentsOptions, call?: ViewerCallOptions): Promise<Uint8Array> {
    await this.#settled();
    return this.call("exportComments", [options, this.#document?.id ?? 0], call);
  }

  /**
   * A comment summary of the open document as its edits leave them, saved as a new document: its
   * bytes, and the warnings it raised (`summary-page-not-drawn`). One call, not a job: aborting
   * `call.signal` drops the answer.
   *
   * @throws `missing-glyph`, `font-not-embedded`, `invalid-font`, `limit-exceeded`.
   */
  async commentSummary(request: CommentSummaryRequest, call?: ViewerCallOptions): Promise<MadeSummary> {
    await this.#settled();
    return this.call("commentSummary", [request, this.#document?.id ?? 0], call);
  }

  /**
   * What a contents page placed as `placement` would list, for a preview: the entries and their
   * labels and, given the `font` it will be set in, how many pages it takes (laid out in the text
   * module, which loads for it), so the preview can show the labels it will print.
   */
  async contentsListing(
    placement: ContentsPlacement,
    font?: Uint8Array,
    call?: ViewerCallOptions,
  ): Promise<ContentsPreview> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const id = this.#document?.id ?? 0;
    return this.call("contentsListing", font === undefined ? [placement, id] : [placement, id, font], call);
  }

  /**
   * Lets the open document take text boxes, keeping its edit session. A document opened before
   * the extract module loaded refuses them (`appearance-needs-text`). The worker moves it into a
   * copy in the extract module (Helvetica, Times and Courier), or in the text module when `text`
   * is set (a font given by its bytes, fallback fonts). A document already in that module needs
   * nothing, and a restarted worker moves it again.
   *
   * @throws when the document's file isn't kept (a large file read from its OPFS copy that is
   *   gone), or the module didn't load; as `applyEdit` for a document that takes no edits.
   */
  async editInTextCopy(options: ViewerCallOptions & { readonly text?: boolean } = {}): Promise<void> {
    const { text = false, ...call } = options;
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const id = this.#document?.id ?? 0;
    await this.call("editInTextCopy", [text, id], call);
    this.#textCopy = { text: text || this.#textCopy?.text === true };
  }

  /** Undoes the newest step of the edit session; the pages it touched are drawn as they were. */
  undoEdit(options?: ViewerCallOptions): Promise<EditResult> {
    return this.#edit((id) => this.call("undoEdit", [id], options));
  }

  /** Redoes the oldest undone step. */
  redoEdit(options?: ViewerCallOptions): Promise<EditResult> {
    return this.#edit((id) => this.call("redoEdit", [id], options));
  }

  /** Undoes or redoes steps until `position` of the history's steps are done, drawing once. */
  goToStep(position: number, options?: ViewerCallOptions): Promise<EditResult> {
    return this.#edit((id) => this.call("goToStep", [position, id], options));
  }

  /** The edit session's history, read from the worker (after a failed call, which answers none). */
  async editHistory(options?: ViewerCallOptions): Promise<EditHistory> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const opened = this.#document;
    const history = await this.call("editHistory", [opened?.id ?? 0], options);
    // A failed edit clears the steps undone: what a restart would replay follows.
    if (opened !== undefined && this.#document === opened) this.#edits.take({ pages: [], history });
    return history;
  }

  /** The open document's ID in the worker, once a restarted worker has opened it again; 0 when none is open. */
  async documentId(): Promise<number> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.#document?.id ?? 0;
  }

  /**
   * Makes `bytes` a source document of the open document, for the ops that copy its pages
   * (`insertPagesFrom`, `replacePages`, `mergeDocuments`): its ID and page IDs. The page keeps
   * the bytes while an edit names the source, so a restarted worker adds it again before it
   * applies the edits; it keeps no password, so edits from an encrypted source are lost then.
   */
  async addSource(
    bytes: Uint8Array,
    options?: { readonly password?: string },
    call?: ViewerCallOptions,
  ): Promise<AddedSource> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const opened = this.#document;
    const password = options?.password;
    const added = await this.call("addSource", [bytes, password, opened?.id ?? 0], call);
    if (opened !== undefined && this.#document === opened) {
      this.#edits.addedSource(added.id, bytes, password !== undefined && password !== "");
    }
    return added;
  }

  /** The files the open document carries, each with what holds it, as its edits leave them. */
  async attachments(call?: ViewerCallOptions): Promise<DocumentAttachment[]> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("attachments", [this.#document?.id ?? 0], call);
  }

  /** The bytes of the open document's attachment `id`, exactly as attached, and its checksum's verdict. */
  async attachmentBytes(id: string, call?: ViewerCallOptions): Promise<AttachmentFile> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("attachmentBytes", [id, this.#document?.id ?? 0], call);
  }

  /** The largest file, in bytes, the open document can read back out and so attach. */
  async attachmentLimit(call?: ViewerCallOptions): Promise<number> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("attachmentLimit", [this.#document?.id ?? 0], call);
  }

  /** The portfolio the open document is, or `undefined`. */
  async collection(call?: ViewerCallOptions): Promise<Collection | undefined> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("collection", [this.#document?.id ?? 0], call);
  }

  /**
   * Keeps `bytes` in the open document's session and answers their ID, which an `attachFile` edit
   * names them by; a restarted worker gets them again before it replays the edits.
   */
  async addBlob(bytes: Uint8Array, call?: ViewerCallOptions): Promise<string> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const opened = this.#document;
    const id = await this.call("addBlob", [bytes, opened?.id ?? 0], call);
    if (opened !== undefined && this.#document === opened) this.#edits.addedBlob(id, bytes);
    return id;
  }

  /** The open document's metadata as its edits leave it: Info and XMP read together. */
  async metadata(call?: ViewerCallOptions): Promise<DocumentMetadata> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("metadata", [this.#document?.id ?? 0], call);
  }

  /** The open document's XMP metadata as text, as its edits leave it. */
  async xmpPacket(call?: ViewerCallOptions): Promise<XmpText> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("xmpPacket", [this.#document?.id ?? 0], call);
  }

  /** The open document's answers as a JSON, XFDF or FDF file; XFDF and FDF name the PDF `file`. */
  async exportFormData(format: FormDataFormat, file?: string, call?: ViewerCallOptions): Promise<Uint8Array> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("exportFormData", [format, file, this.#document?.id ?? 0], call);
  }

  /**
   * What importing the form data file `bytes` (JSON, XFDF or FDF) would do: the `setFieldValue`
   * ops that make its changes, to apply as one step, and what it left out. Changes nothing.
   *
   * @throws `interchange-malformed` or `interchange-unknown-format` for a file it can't read.
   */
  async planFormImport(bytes: Uint8Array, call?: ViewerCallOptions): Promise<FormImportPlan> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("planFormImport", [bytes, this.#document?.id ?? 0], call);
  }

  /** Page `index`'s boxes in user space, as the open document's edits leave them. */
  async pageBoxes(index: number, call?: ViewerCallOptions): Promise<PageBoxesRead> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("pageBoxes", [index, this.#document?.id ?? 0], call);
  }

  /** The objects Lamina PDF added to page `index` as page content, in painting order, as the open document's edits leave them. */
  async addedObjects(index: number, call?: ViewerCallOptions): Promise<AddedObject[]> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("addedObjects", [index, this.#document?.id ?? 0], call);
  }

  /** The page marks on every page of the open document, as its edits leave them: one list for each page, in page order. */
  async pageMarks(call?: ViewerCallOptions): Promise<PageMarkInfo[][]> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("pageMarks", [this.#document?.id ?? 0], call);
  }

  /** The objects page `index` draws, in painting order, as the open document's edits leave them, with their boxes in page space. */
  async contentObjects(index: number, call?: ViewerCallOptions): Promise<PageObject[]> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("contentObjects", [index, this.#document?.id ?? 0], call);
  }

  /** The runs of text page `index` shows, as the open document's edits leave them, with their boxes in page space. */
  async textRuns(index: number, call?: ViewerCallOptions): Promise<ContentTextRun[]> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("textRuns", [index, this.#document?.id ?? 0], call);
  }

  /**
   * What `editText` would do on page `index`, without doing it: the characters the run's font can't draw,
   * the font that would draw each and how much wider the line would grow, for the editor to show before it
   * commits.
   */
  async planEditText(
    index: number,
    run: TextRunRef,
    text: string,
    fonts?: readonly AnnotFont[],
    call?: ViewerCallOptions,
  ): Promise<TextEditReport> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("planEditText", [index, run, text, fonts ? [...fonts] : undefined, this.#document?.id ?? 0], call);
  }

  /** The open document's page label ranges, in page order, as its edits leave them; empty without labels. */
  async labelRanges(call?: ViewerCallOptions): Promise<PageLabelRange[]> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("labelRanges", [this.#document?.id ?? 0], call);
  }

  /** How saving the open document with `options` would go: its constraint, signatures, mode and warnings. */
  async planSave(options: SaveOptions, call?: ViewerCallOptions): Promise<PlannedSave> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("planSave", [options, this.#document?.id ?? 0], call);
  }

  /**
   * What saving the open document with `ops` applied would break, before they apply: each
   * signature restriction they violate, so a page edit a signature forbids can be refused first.
   */
  async wouldViolate(ops: readonly Op[], call?: ViewerCallOptions): Promise<Violation[]> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("wouldViolate", [ops, this.#document?.id ?? 0], call);
  }

  /**
   * Writes the open document as a save with `options` would, for the caller to write where the
   * file lives; the document stays unsaved until `adoptEdits(copy.token)`. Save a copy uses it
   * and never adopts.
   */
  async copyEdits(options: SaveOptions, call?: ViewerCallOptions): Promise<SavedCopy> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const copy = await this.call("copyEdits", [options, this.#document?.id ?? 0], call);
    this.#copy = { token: copy.token, bytes: copy.bytes };
    return copy;
  }

  /**
   * Writes the open document as a save with `options` would with `ops` applied, for Save a copy
   * of a change the document doesn't take: the document stays as it was, and the history loses
   * only its undone steps. Emits `edit` with that history.
   */
  async copyWithEdit(ops: readonly Op[], options: SaveOptions, call?: ViewerCallOptions): Promise<EditedCopy> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const opened = this.#document;
    const copy = this.call("copyWithEdit", [ops, options, opened?.id ?? 0], call);
    // A copy that failed once the ops applied still cleared the undone steps: the worker's history follows.
    const history = await copy.then(
      (c) => c.history,
      () => this.editHistory().catch(() => undefined),
    );
    if (history !== undefined && opened !== undefined && this.#document === opened) {
      const edited = { pages: [], history };
      this.#edits.take(edited);
      this.#events.emit("edit", edited);
    }
    return copy;
  }

  /**
   * Makes the copy `token` names the document's saved file, once it is written where the file
   * lives: the worker rebases onto it, the history marks the point saved, and a restarted worker
   * reopens that file. Emits `edit` with the history.
   *
   * @throws when the document changed since the copy (it stays unsaved).
   */
  async adoptEdits(token: number, call?: ViewerCallOptions): Promise<EditHistory> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const opened = this.#document;
    const history = await this.call("adoptEdits", [token, opened?.id ?? 0], call);
    const copy = this.#copy;
    if (opened === undefined || this.#document !== opened) return history;
    this.#copy = undefined;
    if (copy?.token === token && this.#reopen !== undefined) this.#reopen = { ...this.#reopen, source: copy.bytes };
    this.#edits.saved(history);
    this.#events.emit("edit", { pages: [], history });
    return history;
  }

  /**
   * Runs one edit session call on the open document, then keeps what it changed: the pages'
   * sizes in `document`, the steps for a restart, and an `edit` event.
   */
  async #edit(
    call: (document: number) => Promise<EditResult>,
    edit?: { readonly ops: readonly Op[]; readonly label: string },
  ): Promise<EditResult> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const opened = this.#document;
    // 0 names no document: the worker answers `disposed`.
    const result = await call(opened?.id ?? 0);
    // Another document opened meanwhile: this answer is the old one's.
    if (opened === undefined || this.#document !== opened) return result;
    this.#takePages(opened, result);
    this.#edits.take(result, edit);
    this.#events.emit("edit", result);
    return result;
  }

  /**
   * The pages keep their sizes in `document`, for whoever lays them out; an edit that inserted,
   * deleted or moved pages gives `document` its new page list.
   */
  #takePages(opened: OpenedDocument, result: EditResult): void {
    const list = result.order ?? result.relabelled;
    if (list !== undefined) {
      this.#document = { ...opened, pageCount: list.length, pages: [...list] };
      return;
    }
    if (result.pages.length === 0) return;
    const pages = [...opened.pages];
    for (const page of result.pages) pages[page.index] = page;
    this.#document = { ...opened, pages };
  }

  /**
   * Every request the document worker has made, and had blocked, since it started: what its
   * `network` and `csp-violation` events reported, including those sent before anyone listened.
   */
  networkLog(options?: ViewerCallOptions): Promise<WorkerNetworkLog> {
    return this.call("networkLog", [], options);
  }

  /** The worker's WebAssembly memory in bytes (see `WorkerMemory`), for measuring. */
  memory(options?: ViewerCallOptions) {
    return this.call("memory", [], options);
  }

  /** Page `index`'s size, rotation, ID and label. */
  pageInfo(index: number, options?: ViewerCallOptions) {
    return this.call("pageInfo", [index], options);
  }

  /**
   * Page `index`'s text runs and glyph quads, read by Lamina PDF (needs the extract or text
   * module, loaded by default). At `background` priority unless `options` says otherwise.
   */
  async pageText(index: number, options: ViewerCallOptions = {}) {
    await this.#firstDraw?.wait(options.signal);
    const key = `text:${this.#document?.id ?? 0}:${index}`;
    return this.call("pageText", [index], { priority: "background", key, ...options });
  }

  /**
   * Page `index`'s text blocks in reading order (Lamina PDF's extraction), for copy as paragraph.
   * At `background` priority unless `options` says otherwise.
   */
  async readingOrder(index: number, options: ViewerCallOptions = {}) {
    await this.#firstDraw?.wait(options.signal);
    const key = `order:${this.#document?.id ?? 0}:${index}`;
    return this.call("readingOrder", [index], { priority: "background", key, ...options });
  }

  /**
   * The matches of `query` on each of `pages`, at `background` priority unless `options` says
   * otherwise, so renders run first. The find controller calls it a few pages at a time.
   */
  find(query: string, find: FindOptions, pages: readonly number[], options: ViewerCallOptions = {}) {
    return this.call("find", [query, find, pages], { priority: "background", ...options });
  }

  /** Whether the open document says it is tagged, and its structure tree when it has one. */
  async structure(options?: ViewerCallOptions) {
    await this.#firstDraw?.wait(options?.signal);
    return this.call("structure", [], options);
  }

  /**
   * The open document for reader mode: its blocks in the structure tree's order, figures,
   * formulas and tables as crops with their alternate text, or why its tree can't be trusted.
   */
  reflow(options?: ViewerCallOptions) {
    return this.call("reflow", [], options);
  }

  /**
   * The open document's outline (bookmarks): top-level items, each with its children and where
   * it goes as a page and a page-space view; empty without one. Read when first asked for.
   */
  outline(options?: ViewerCallOptions) {
    return this.call("outline", [], options);
  }

  /** Where the named destination `name` goes, or `undefined` when the document has no such name. */
  namedDestination(name: string, options?: ViewerCallOptions) {
    return this.call("namedDestination", [name], options);
  }

  /**
   * How the open document asks to be shown when it opens: the page and zoom, the panel, the page
   * layout and the window preferences, as its edits leave them.
   */
  initialView(options?: ViewerCallOptions) {
    return this.call("initialView", [], options);
  }

  /** The open document's article threads, each with its parts in reading order; empty without any. */
  threads(options?: ViewerCallOptions) {
    return this.call("threads", [], options);
  }

  /**
   * What a properties view shows about the open document (`@laminapdf/sdk`'s `properties()`,
   * read in the worker). It reads every object once: ask when the view opens.
   */
  properties(options?: ViewerCallOptions) {
    return this.call("properties", [], options);
  }

  /** What the object inspector reads of the open document as it opens: the trailer, the counts and the version. */
  inspectionSummary(options?: ViewerCallOptions) {
    return this.call("inspectionSummary", [], options);
  }

  /** Tab `tab`'s facts of the open document (revisions, fonts, colour profiles or claims). */
  inspectionTab(tab: InspectionTab, options?: ViewerCallOptions) {
    return this.call("inspectionTab", [tab], options);
  }

  /** Object `num` of the open document with the entries of what it refers to; `undefined` without one. */
  objectDetail(num: number, options?: ViewerCallOptions) {
    return this.call("objectDetail", [num], options);
  }

  /** The places that refer to object `num` of the open document; the first call indexes every reference. */
  objectReferrers(num: number, options?: ViewerCallOptions) {
    return this.call("objectReferrers", [num], options);
  }

  /**
   * The first `max` bytes of stream `num` of the open document, raw or decoded, and its whole
   * length; `undefined` when it isn't a stream. A stream that decodes past the document's limits
   * is refused (`limit-exceeded`).
   */
  streamRead(num: number, view: "raw" | "decoded", max: number, options?: ViewerCallOptions) {
    return this.call("streamRead", [num, view, max], options);
  }

  /**
   * Page `index` drawn for printing at `print.dpi` with annotations as their print flags say, as
   * a PNG, at `background` priority unless `options` says otherwise, so the pages in view draw first.
   */
  printPage(index: number, print: PrintOptions, options: ViewerCallOptions = {}) {
    return this.call("printPage", [index, print], { priority: "background", ...options });
  }

  /**
   * Page `index` drawn at `exported.dpi` and encoded as PNG or JPEG, its resolution recorded in
   * the file, at `background` priority unless `options` says otherwise, so the pages in view draw
   * first. One page per call: an export of many pages asks for them one after another.
   */
  exportPage(index: number, exported: ExportPageOptions, options: ViewerCallOptions = {}) {
    return this.call("exportPage", [index, exported], { priority: "background", ...options });
  }

  /**
   * Pages `indexes` drawn at `exported.dpi` as TIFF: one multi-page file when `exported.multiPage`
   * asks for it and the pages fit the export's memory budget, else a file per page. The worker
   * loads the TIFF module the first time, at `background` priority unless `options` says otherwise.
   */
  exportTiff(indexes: readonly number[], exported: ExportTiffOptions, options: ViewerCallOptions = {}) {
    return this.call("exportTiff", [indexes, exported], { priority: "background", ...options });
  }

  /**
   * The open document's text as it is now, edits included: plain text, Markdown or the
   * `laminapdf.extract/1` JSON with every block's box, from the pages `extracted.pages` names
   * (every page when absent), in the reading order `extracted.readingOrder` asks for. At
   * `background` priority unless `options` says otherwise.
   *
   * @throws `LaminaPdfError` `page-index-out-of-range`; `extract-module-not-loaded` when the
   *   worker can't load the extract module.
   */
  extractText(extracted: ExtractTextOptions, options: ViewerCallOptions = {}) {
    return this.call("extractText", [extracted], { priority: "background", ...options });
  }

  /**
   * The open document's pictures as files, edits included, each once on the first page that
   * draws it: JPEG and JPEG 2000 as stored or (`keepOriginals: false`) JPEG as PNG, the rest as
   * PNG; those under `minimumSize` pixels on their longer side are left out and counted in
   * `skipped`. At `background` priority unless `options` says otherwise. Every picture of the
   * pages comes in one answer: ask for a few pages at a time to keep the memory down.
   *
   * @throws `LaminaPdfError` `page-index-out-of-range`.
   */
  extractPictures(extracted: ExtractPicturesOptions, options: ViewerCallOptions = {}) {
    return this.call("extractPictures", [extracted], { priority: "background", ...options });
  }

  /**
   * How many pictures, links and bookmarks the open document has as it is now, and whether it
   * is tagged, for an export dialog to offer only what is there. Counting the pictures decodes
   * them all once; the worker keeps the answer until an edit, undo or redo changes the document.
   */
  exportSummary(options: ViewerCallOptions = {}) {
    return this.call("exportSummary", [], { priority: "background", ...options });
  }

  /**
   * The open document's links (every page's) and bookmarks as they are now, as JSON
   * (`laminapdf.links/1`, `laminapdf.outline/1`) or as CSV with a header row. At `background`
   * priority unless `options` says otherwise.
   */
  linksAndBookmarks(format: "json" | "csv", options: ViewerCallOptions = {}) {
    return this.call("linksAndBookmarks", [format], { priority: "background", ...options });
  }

  /**
   * A new PDF with one page per picture, in order: its bytes, to open or save as a new
   * document; the open one, if any, is untouched, and none need be open. A JPEG is embedded as
   * it is; a PNG keeps its transparency. At `background` priority unless `options` says otherwise.
   *
   * @throws `LaminaPdfError` `unsupported-image` for a type other than `image/jpeg` or
   *   `image/png`; `invalid-image` for bytes that aren't that type; `invalid-geometry` for no
   *   pictures, a negative margin, or a page too small for its margins.
   */
  imagesToPdf(images: readonly ImageToPdf[], layout: ImagesToPdfOptions, options: ViewerCallOptions = {}) {
    return this.call("imagesToPdf", [images, layout], { priority: "background", ...options });
  }

  /**
   * Every byte of the open remote file as it was opened, fetched where the block cache lacks
   * them. A local file's bytes are the page's own: it never asks.
   */
  fileBytes(options?: ViewerCallOptions) {
    return this.call("fileBytes", [], options);
  }

  /** Page `index`'s links, in annotation order, each with what following it does (the link layer asks as pages mount). */
  async pageLinks(index: number, options: ViewerCallOptions = {}) {
    // Links read with the extract module: like the text layer, they wait for the first page drawn,
    // and come after renders.
    await this.#firstDraw?.wait(options.signal);
    return this.call("pageLinks", [index], { priority: "background", ...options });
  }

  /**
   * Page `index`'s annotations, in the order of its `/Annots`, each a record in page space with
   * its place there (`index`), as the annotation layer reads them. An edit to the page reads
   * them again.
   */
  async pageAnnotations(index: number, options: ViewerCallOptions = {}) {
    // Read with the extract module, as the links are: after the first page drawn.
    await this.#firstDraw?.wait(options.signal);
    return this.call("pageAnnotations", [index], { priority: "background", ...options });
  }

  /**
   * The document's form fields, in the order the form lists them, each with its widgets, value,
   * flags and options, as Fill & Sign's form layer reads them. An edit reads them again.
   */
  async formFields(options: ViewerCallOptions = {}) {
    // Read with the extract module, as the annotations are: after the first page drawn.
    await this.#firstDraw?.wait(options.signal);
    return this.call("formFields", [], { priority: "background", ...options });
  }

  /** The order Tab visits page `index`'s widgets in, as the page asks (`/Tabs`). */
  async tabOrder(index: number, options: ViewerCallOptions = {}) {
    await this.#firstDraw?.wait(options.signal);
    return this.call("tabOrder", [index], { priority: "background", ...options });
  }

  /**
   * The text box `id` (an annotation ID from `pageAnnotations`) read back as it is now: where it
   * is in page space, and the spec a `freeText` patch to `updateAnnot` changes. Undefined for an
   * annotation of another kind.
   */
  async textBox(id: string, options: ViewerCallOptions = {}) {
    return this.call("textBox", [id], options);
  }

  /**
   * Sound comment `id`'s recording as a WAV file, which every browser's `<audio>` plays: raw,
   * signed, mu-law and A-law samples at their own rate and channels.
   *
   * @throws `sound-unavailable` when `id` isn't a sound comment whose samples can be played.
   */
  async soundAsWav(id: string, options: ViewerCallOptions = {}): Promise<Uint8Array> {
    return this.call("soundAsWav", [id], options);
  }

  /**
   * The document's comments as threads in document order, as they are now: each comment with
   * its replies, review states, the text it marks and whether its thread is broken, what a
   * Comments panel lists.
   *
   * @since 0.7.0
   */
  async commentThreads(options: ViewerCallOptions = {}): Promise<readonly CommentThread[]> {
    return this.call("commentThreads", [], { priority: "background", ...options });
  }

  /**
   * Renders `request` as an ImageBitmap (close it when done). Identical requests in flight are
   * drawn once. An aborted render rejects with `AbortError` and never paints.
   */
  async render(request: RenderRequest, options: ViewerCallOptions = {}): Promise<RenderedBitmap> {
    const attempt = { interrupted: false };
    try {
      return await this.#render(request, options, attempt);
    } catch (e) {
      if (!attempt.interrupted) throw e;
    }
    // The worker was lost while this render waited: it is sent again, once, in the restarted
    // worker (ADR 0009 §3.6). A restart that didn't reopen the document (an encrypted one waits
    // for its password) leaves it nothing to draw from: it ends as cancelled, as for a new open.
    await this.#recovering?.catch(() => {});
    if (this.#document === undefined) throw new DOMException("the document closed in a restart", "AbortError");
    return this.#render(request, options, { interrupted: false });
  }

  /** One render; `attempt.interrupted` says a failure was only the worker's loss (see `render`). */
  async #render(
    request: RenderRequest,
    options: ViewerCallOptions,
    attempt: { interrupted: boolean },
  ): Promise<RenderedBitmap> {
    const id = this.#nextRender++;
    // A restarted worker numbers its documents again: read the ID once it serves.
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    // 0 names no document: the worker answers `disposed`.
    const documentId = this.#document?.id ?? 0;
    const needsPassword = this.#document?.needsPassword === true;
    const key = renderKey(request, documentId);
    let soft: ReturnType<typeof setTimeout> | undefined;
    let hard: ReturnType<typeof setTimeout> | undefined;
    const pause = () => {
      clearTimeout(soft);
      clearTimeout(hard);
    };
    const start = () => {
      pause();
      soft = setTimeout(() => this.#events.emit("render-slow", { id, page: request.page }), this.#timeouts.soft);
      hard = setTimeout(() => {
        if (needsPassword) this.#events.emit("render-stuck", { id, page: request.page, needsPassword: true });
        else
          this.stop("hard-timeout").catch((error) =>
            this.#events.emit("restart-failed", { reason: "hard-timeout", error }),
          );
      }, this.#timeouts.hard);
    };
    const timers: RenderTimers = { start, pause };
    let set = this.#timers.get(key);
    if (set === undefined) {
      set = new Set();
      this.#timers.set(key, set);
    }
    set.add(timers);
    // Joining a render already being drawn: its render-start has passed.
    if (this.#drawing === key) start();
    try {
      const drawn = await this.call("render", [request, documentId], { ...options, key });
      // A page has been drawn: text calls may queue now. (A cancelled render doesn't count: the
      // render that replaces it would wait behind them.)
      this.#firstDraw?.drawn(documentId);
      return drawn;
    } catch (e) {
      // Lost with its worker while it waited: a restart is under way, and it wasn't the render
      // drawing then, which isn't re-sent so a page that crashes the engine can't loop.
      attempt.interrupted =
        isLaminaPdfError(e, "worker-crashed") && this.#recovering !== undefined && key !== this.#lost;
      throw e;
    } finally {
      const own = this.#timers.get(key);
      own?.delete(timers);
      if (own?.size === 0) {
        this.#timers.delete(key);
        if (this.#drawing === key) this.#drawing = undefined;
      }
      pause();
    }
  }

  /**
   * Stops whatever the worker is doing by terminating it (the user's Stop, or the hard cap), then
   * restarts it as after a crash: pending calls reject, renders that were waiting are sent again,
   * the running one isn't, and an encrypted document asks for its password again.
   */
  stop(reason: "cancelled" | "hard-timeout" = "cancelled"): Promise<void> {
    // A restart already under way serves: a second would end the worker it is starting.
    if (this.#recovering !== undefined) return this.#recovering;
    this.#pdf.terminate();
    return this.#recover(reason);
  }

  /** Ends the worker for good. */
  terminate(): void {
    this.#reopen = undefined;
    this.#document = undefined;
    this.#edits.reset();
    this.#pdf.terminate();
  }

  /**
   * Applies the edit session's steps to the document a new worker reopened, which is the file as
   * first opened. When they can't all be applied again, the session starts over.
   */
  /**
   * `restored`; `lost` when the steps couldn't be replayed, and nothing was sent; `failed` when the
   * replay failed half way, leaving edits in the model that aren't drawn.
   */
  async #restoreEdits(opened: OpenedDocument): Promise<"restored" | "lost" | "failed" | undefined> {
    if (this.#edits.empty) return undefined;
    const replay = this.#edits.replay();
    // A replay that can't reproduce the document would show something else as restored.
    if (replay === undefined) {
      this.#edits.reset();
      return "lost";
    }
    const { steps, undone, sources, blobs } = replay;
    try {
      const result = (await this.#pdf.request("restoreEdits", [steps, undone, opened.id, sources, blobs], {
        priority: PRIORITY.document,
      })) as EditResult;
      this.#takePages(opened, result);
      this.#edits.replayed(result);
      this.#events.emit("edit", result);
      return "restored";
    } catch {
      this.#edits.reset();
      return "failed";
    }
  }

  /**
   * Opens the document again in a restarted worker, with its edits: `reopened` when it did,
   * `edits` what became of the session's steps. An encrypted document waits for its password.
   */
  async #reopenAfterRestart(): Promise<{ reopened: boolean; edits: "restored" | "lost" | undefined }> {
    const reopen = this.#reopen;
    if (reopen === undefined) return { reopened: false, edits: undefined };
    if (this.#document?.needsPassword === true) {
      // The password died with the worker; it is never kept on the page.
      this.#reopen = undefined;
      this.#document = undefined;
      const edits = this.#edits.empty ? undefined : "lost";
      this.#edits.reset();
      this.#events.emit("password-required", {});
      return { reopened: false, edits };
    }
    const open = async () => {
      this.#document = (await this.#pdf.request("openDocument", [reopen.source, reopen.options], {
        priority: PRIORITY.document,
      })) as OpenedDocument;
      // The restarted worker has no extract module yet: its first page goes first again.
      this.#firstDraw = new FirstDraw(this.#document.id);
      // Its edits may hold text boxes, which only a text-capable copy takes.
      const textCopy = this.#textCopy;
      if (textCopy !== undefined) {
        // A copy that can't open now leaves the base model, whose replay of a text box then fails
        // and reports the edits lost, rather than failing the recovery itself.
        await this.#pdf
          .request("editInTextCopy", [textCopy.text, this.#document.id], { priority: PRIORITY.document })
          .catch(() => undefined);
      }
      return this.#document;
    };
    const restored = await this.#restoreEdits(await open());
    // A replay that failed half way left edits in the model that aren't drawn: open the file once
    // more, as it was. One that never started left the file as opened, so it isn't opened again.
    if (restored === "failed") await open();
    return { reopened: true, edits: restored === "failed" ? "lost" : restored };
  }

  #recover(reason: RestartReason): Promise<void> {
    if (this.#recovering !== undefined) return this.#recovering;
    // Read before the lost calls' rejections run, which clear what was drawing.
    this.#lost = this.#drawing;
    const recovering = (async () => {
      await this.#pdf.restart();
      await this.#pdf.request("initBackend", [this.#backend], { priority: PRIORITY.document });
      const { reopened, edits } = await this.#reopenAfterRestart();
      this.#events.emit("restarted", { reason, reopened, ...(edits === undefined ? {} : { edits }) });
    })();
    const tracked = recovering.finally(() => {
      if (this.#recovering === tracked) this.#recovering = undefined;
    });
    this.#recovering = tracked;
    return tracked;
  }
}
