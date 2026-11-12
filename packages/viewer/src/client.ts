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
//   source, but the call that was running is never re-sent (the UI offers Retry). An encrypted
//   document isn't reopened: the password lived only in the dead worker, so `password-required`
//   fires and the caller opens again with the password.

import type { RenderRequest } from "@laminapdf/render";
import { isLaminaPdfError, type Op, type PlannedSave, type SaveOptions } from "@laminapdf/sdk";
import {
  connect,
  Emitter,
  type WorkerClient,
  type WorkerInitOptions,
  type WorkerWasmSource,
} from "@laminapdf/sdk/worker";
import { EditLog } from "./edit-log.js";
import type { FindOptions } from "./find/fold.js";
import { FirstDraw } from "./first-draw.js";
import {
  type BackendInit,
  type EditHistory,
  type EditResult,
  type LoadProgress,
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
  | "find"
  | "applyEdit"
  | "undoEdit"
  | "redoEdit"
  | "goToStep"
  | "editHistory"
  | "planSave"
  | "copyEdits"
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

/**
 * Whether a call failed only because the worker crashed under it (its error, messageerror or a
 * trap). The client restarts the worker and reopens the document, so the caller draws again rather
 * than show an error: a call made now waits for the restart, then runs in the new worker. A worker
 * the client ended itself (`stop()`: the reader's Stop, or a render past the hard cap) rejects with
 * the reason `terminated`, and isn't retried, since that render is what was stopped.
 */
export function interruptedByCrash(e: unknown): boolean {
  return isLaminaPdfError(e, "worker-crashed") && e.data?.reason !== "terminated";
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
    options: ViewerCallOptions & { key?: string } = {},
  ): Promise<Result<M>> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    const { signal, priority, key } = options;
    // Renders, and calls that name a priority (text, at `background`), queue with the tiles; other
    // document calls run first.
    const level = method === "render" || priority !== undefined ? PRIORITY[priority ?? "visible"] : PRIORITY.document;
    try {
      return (await this.#pdf.request(method, params, {
        priority: level,
        ...(signal === undefined ? {} : { signal }),
        ...(key === undefined ? {} : { key }),
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

  /** How saving the open document with `options` would go: its constraint, signatures, mode and warnings. */
  async planSave(options: SaveOptions, call?: ViewerCallOptions): Promise<PlannedSave> {
    if (this.#recovering !== undefined) await this.#recovering.catch(() => {});
    return this.call("planSave", [options, this.#document?.id ?? 0], call);
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

  /** The pages keep their sizes in `document`, for whoever lays them out. */
  #takePages(opened: OpenedDocument, result: EditResult): void {
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
   * What a properties view shows about the open document (`@laminapdf/sdk`'s `properties()`,
   * read in the worker). It reads every object once: ask when the view opens.
   */
  properties(options?: ViewerCallOptions) {
    return this.call("properties", [], options);
  }

  /**
   * Page `index` drawn for printing at `print.dpi` with annotations as their print flags say, as
   * a PNG, at `background` priority unless `options` says otherwise, so the pages in view draw first.
   */
  printPage(index: number, print: PrintOptions, options: ViewerCallOptions = {}) {
    return this.call("printPage", [index, print], { priority: "background", ...options });
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
   * Renders `request` as an ImageBitmap (close it when done). Identical requests in flight are
   * drawn once. An aborted render rejects with `AbortError` and never paints.
   */
  async render(request: RenderRequest, options: ViewerCallOptions = {}): Promise<RenderedBitmap> {
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
   * restarts it as after a crash: pending calls reject, the running one isn't re-sent, and an
   * encrypted document asks for its password again.
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
    const { steps, undone } = replay;
    try {
      const result = (await this.#pdf.request("restoreEdits", [steps, undone, opened.id], {
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
