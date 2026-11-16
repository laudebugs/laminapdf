// The desktop app's engine client (ADR 0008): the `ViewerEngine` `@laminapdf/viewer` draws from,
// over the main process and its engine process instead of a document worker. The viewer's code
// runs unchanged; only this client differs from the web reader's `ViewerClient`.
//
// - Tiles come over the `lamina://` scheme as lossless WebP (raw RGBA as the fallback); the
//   webview fetches them, and an aborted render's ticket goes to `cancel_tiles`, so the engine
//   skips it. A tile that arrives after its render was aborted, or after its document went, is
//   closed and never painted.
// - The main process restarts a crashed engine and reopens its documents, with their kept
//   passwords: `engine://restarted` becomes the viewer's `restarted`, which redraws what is
//   on screen; `engine://failed` and a document that didn't come back become `restart-failed`.
// - Edits go to the engine, which draws the pages from the edited file from then on; the history
//   is kept here, per document, so it survives a tab switch (desktop-edits.ts). A save runs in the
//   main process, which writes the file (desktop-saves.ts); checking an edit against the
//   signatures before it applies (`wouldViolate`) and the contents page (`createContentsPage`)
//   aren't served yet.
// - The Properties dialog's facts, metadata and XMP packet come from the engine in one call
//   (`properties`), with the edits so far.
// - Text, links and search aren't served by the engine yet: their calls answer as for a document
//   without them (no text runs, no links, untagged).
// - Only types come from `@laminapdf/viewer` here, so the app's Node tests run without the
//   viewer's build (which needs the WebAssembly); the app itself imports `createViewer`.

import { DEFAULT_LIMITS, type PixelRect, planTile, type RenderRequest } from "@laminapdf/render";
import type { DocumentPropertiesData } from "@laminapdf/ui";
import type { DocumentMetadataData, XmpPacketData } from "@laminapdf/ui/dialogs";
import type {
  EditHistory,
  EditResult,
  FindPageResult,
  OpenedDocument,
  PageTextData,
  ReadingBlock,
  RenderedBitmap,
  SavedCopy,
  ViewerCallOptions,
  ViewerEngine,
  ViewerEvent,
  ViewerEvents,
} from "@laminapdf/viewer";
import { DesktopEdits, type EditOutcome, type EditRequest, NO_EDITS, pagesOf } from "./desktop-edits.ts";
import {
  asked,
  type DesktopSaved,
  type PlannedSave,
  plannedSave,
  SAVE_CANCELLED,
  type SaveOptions,
  type SaveTarget,
  saveCall,
} from "./desktop-saves.ts";
import { type TileTransport, tileUrl } from "./tile-url.ts";

/** A page as the main process reports it: its stable ID, displayed size in points and `/Rotate`. */
interface DesktopPage {
  readonly id: string;
  readonly width: number;
  readonly height: number;
  readonly rotate: number;
}

/** A document the main process opened (`open_dialog`, `unlock_document`): `DocumentMeta` in Rust. */
export interface DesktopDocument {
  /** The main process's id for it. */
  readonly doc: number;
  /** The file's name, without its folder. */
  readonly name: string;
  readonly size: number;
  readonly pageCount: number;
  readonly encrypted: boolean;
  readonly needsPassword: boolean;
  /** Whether reading it needed repairs (ADR 0003). */
  readonly repaired: boolean;
  readonly pages: readonly DesktopPage[];
}

/** How a document came back after its engine restarted (`Reopened` in Rust). */
type Reopened = "open" | "locked" | "closed";

/** `engine://restarted`'s payload. */
export interface RestartedPayload {
  readonly window: string;
  readonly docs: readonly { readonly doc: number; readonly state: Reopened }[];
}

/**
 * What the Properties dialog shows of a document, as the engine reads it in one call
 * (`laminapdf-desktop-engine`'s `properties` module): `@laminapdf/ui`'s shapes.
 */
export interface DesktopProperties {
  readonly properties: DocumentPropertiesData;
  readonly metadata: DocumentMetadataData;
  readonly xmp: XmpPacketData;
  /** Each page's label, when the document labels its pages. */
  readonly labels?: readonly string[];
}

/** The main process's error, `{ kind, message }` (`DesktopError` in Rust); `code` is its kind. */
export class DesktopEngineError extends Error {
  override readonly name = "DesktopEngineError";
  /** The stable name the UI matches on (`wrong-password`, `engine-restarted`, `invalid` …). */
  readonly code: string;

  constructor(code: string, message: string) {
    super(message);
    this.code = code;
  }
}

/** Turns what a failed `invoke` threw into a {@link DesktopEngineError}. */
export function engineError(error: unknown): DesktopEngineError {
  if (error instanceof DesktopEngineError) return error;
  if (typeof error === "object" && error !== null && "kind" in error) {
    const { kind, message } = error as { kind: unknown; message?: unknown };
    return new DesktopEngineError(String(kind), typeof message === "string" ? message : String(kind));
  }
  return new DesktopEngineError("internal", String(error));
}

/** What the client needs of the Tauri shell; the app passes Tauri's own, tests a fake. */
export interface DesktopShell {
  invoke<T>(command: string, args?: Record<string, unknown>): Promise<T>;
  listen<T>(event: string, handler: (payload: T) => void): Promise<() => void>;
  /** The scheme's base: `convertFileSrc("", "lamina")`. */
  readonly tileBase: string;
  fetch(url: string, init: { readonly signal?: AbortSignal }): Promise<Response>;
  /** Decodes an encoded tile (`createImageBitmap(blob)` in the webview). */
  decode(blob: Blob): Promise<ImageBitmap>;
  /** Makes a bitmap from raw RGBA (`createImageBitmap(new ImageData(…))` in the webview). */
  fromPixels(pixels: Uint8ClampedArray<ArrayBuffer>, width: number, height: number): Promise<ImageBitmap>;
}

/** The engine events, sent to this window only (src-tauri/src/engine/mod.rs). */
export const RESTARTED_EVENT = "engine://restarted";
export const FAILED_EVENT = "engine://failed";

const NOT_YET = "the desktop reader doesn't check edits against signatures yet";

/** A save waiting in the edit session's queue: its document, where it goes, and what it did once it ran. */
interface QueuedSave {
  readonly doc: number;
  readonly target: SaveTarget;
  saved: DesktopSaved | undefined;
}

/** The answer of the calls the engine doesn't serve yet: rejected, with why. */
function notYet<T>(): Promise<T> {
  return Promise.reject(new DesktopEngineError("not-yet-supported", NOT_YET));
}

function abortError(): DOMException {
  return new DOMException("the render was cancelled", "AbortError");
}

/** The client over this window's engine. Open documents with {@link show}. */
export class DesktopEngineClient implements ViewerEngine {
  readonly #shell: DesktopShell;
  readonly #transport: TileTransport;
  readonly #listeners = new Map<ViewerEvent, Set<(data: never) => void>>();
  readonly #unlisten: Promise<(() => void)[]>;
  /** The main process's document behind the viewer's, if any. */
  #current: DesktopDocument | undefined;
  #document: OpenedDocument | undefined;
  /** Counts documents shown, as the viewer's document IDs; renders name it, so a stale tile is dropped. */
  #generation = 0;
  #nextTicket = 1;
  /** Tickets of aborted renders, sent together in one `cancel_tiles`. */
  #cancelled: number[] = [];
  /** Each open document's edit session, by the main process's id, kept while it is in another tab. */
  readonly #edits = new Map<number, DesktopEdits>();
  /** Each edited document's pages as its last edit left them. */
  readonly #pagesNow = new Map<number, readonly DesktopPage[]>();
  /** Where the saves the edit session will run go, in its order. */
  readonly #saveTargets: QueuedSave[] = [];
  /** The saves written and not yet marked in their history, by token. */
  readonly #adopting = new Map<number, { readonly doc: number; readonly historyCleared: boolean }>();
  #nextSave = 1;

  constructor(shell: DesktopShell, options: { readonly transport?: TileTransport } = {}) {
    this.#shell = shell;
    this.#transport = options.transport ?? "webp";
    this.#unlisten = Promise.all([
      shell.listen<RestartedPayload>(RESTARTED_EVENT, (payload) => this.#restarted(payload)),
      shell.listen<unknown>(FAILED_EVENT, () => {
        this.#emit("restart-failed", {
          reason: "error",
          error: new DesktopEngineError("engine-failed", "the document process failed repeatedly"),
        });
      }),
    ]);
  }

  /** The document the viewer shows, if any. */
  get document(): OpenedDocument | undefined {
    return this.#document;
  }

  /** The main process's document behind it, if any. */
  get current(): DesktopDocument | undefined {
    return this.#current;
  }

  /**
   * Shows `opened`: the viewer's document for it, with an ID of its own, its pages as its edits
   * left them. A document with edits tells the viewer its history once the viewer shows it.
   */
  show(opened: DesktopDocument): OpenedDocument {
    this.#generation += 1;
    const pages = this.#pagesNow.get(opened.doc) ?? opened.pages;
    this.#current = { ...opened, pages, pageCount: pages.length };
    this.#document = {
      id: this.#generation,
      pageCount: pages.length,
      pages: pagesOf({ pages }),
      needsPassword: opened.needsPassword,
      backend: { name: "pdfium", version: "desktop" },
      encrypted: opened.encrypted,
    };
    const edits = this.#edits.get(opened.doc);
    if (edits !== undefined) {
      const generation = this.#generation;
      // After the caller's `viewer.open`, which starts the viewer's session empty.
      queueMicrotask(() => {
        if (this.#generation === generation) this.#emit("edit", { pages: [], history: edits.history() });
      });
    }
    return this.#document;
  }

  /** Lets go of the shown document without closing it (its tiles in flight are dropped): it is in
      another tab now, or moved to another window. */
  forget(): void {
    this.#current = undefined;
    this.#document = undefined;
    this.#generation += 1;
  }

  /** Forgets the shown document (its tiles in flight are dropped) and closes it in the main process. */
  async close(): Promise<void> {
    const doc = this.#current?.doc;
    this.#current = undefined;
    this.#document = undefined;
    this.#generation += 1;
    if (doc !== undefined) this.dropEdits(doc);
    if (doc !== undefined)
      await this.#shell.invoke("close_document", { doc }).catch((e: unknown) => {
        throw engineError(e);
      });
  }

  /** Stops listening to the engine's events. */
  async dispose(): Promise<void> {
    for (const unlisten of await this.#unlisten) unlisten();
  }

  on<E extends ViewerEvent>(event: E, listener: (data: ViewerEvents[E]) => void): () => void {
    let set = this.#listeners.get(event);
    if (set === undefined) {
      set = new Set();
      this.#listeners.set(event, set);
    }
    set.add(listener as (data: never) => void);
    return () => set.delete(listener as (data: never) => void);
  }

  #emit<E extends ViewerEvent>(event: E, data: ViewerEvents[E]): void {
    for (const listener of this.#listeners.get(event) ?? []) (listener as (data: ViewerEvents[E]) => void)(data);
  }

  /**
   * The engine came back: the viewer redraws the document if it reopened, else the UI is told.
   * Each reopened document's edits are made again in it first, since it opened the file as it is
   * on disk, so the shown one is never drawn without them. A document that came back locked or
   * closed had none: an encrypted document takes no edits.
   */
  #restarted(payload: RestartedPayload): void {
    const doc = this.#current?.doc;
    const state = payload.docs.find((entry) => entry.doc === doc)?.state;
    for (const entry of payload.docs) {
      if (entry.state !== "open") this.dropEdits(entry.doc);
      else if (entry.doc !== doc) void this.#replay(entry.doc);
    }
    if (state === "open" && doc !== undefined) {
      void this.#replay(doc).then((result) => {
        if (this.#current?.doc !== doc) return;
        this.#emit("restarted", { reason: "error", reopened: true });
        if (result !== undefined) this.#emit("edit", result);
      });
    } else if (state === undefined && doc === undefined) {
      this.#emit("restarted", { reason: "error", reopened: false });
    } else if (state === "locked") {
      this.#emit("password-required", {});
    } else {
      const error = new DesktopEngineError("closed", "the document didn't reopen after its process restarted");
      this.#emit("restart-failed", { reason: "error", error });
    }
  }

  /**
   * Makes document `doc`'s edits again, and keeps its pages as they leave it. Resolves to what the
   * viewer redraws, or undefined with none; a replay that failed resolves to the history, which
   * says the document diverged.
   */
  async #replay(doc: number): Promise<EditResult | undefined> {
    const edits = this.#edits.get(doc);
    if (edits === undefined) return undefined;
    try {
      const result = await edits.replay();
      this.#keepPages(doc);
      return result;
    } catch {
      return { pages: [], history: edits.history() };
    }
  }

  /**
   * Renders `request` as an ImageBitmap (close it when done). An aborted render rejects with
   * `AbortError`, tells the engine to skip it, and never paints.
   */
  async render(request: RenderRequest, options: ViewerCallOptions = {}): Promise<RenderedBitmap> {
    const { signal } = options;
    // A function, so the check after each await reads the signal again.
    const aborted = () => signal?.aborted === true;
    if (aborted()) throw abortError();
    const current = this.#current;
    const page = current?.pages[request.page];
    if (current === undefined || page === undefined) throw new DesktopEngineError("disposed", "no document is open");
    if (current.needsPassword) throw new DesktopEngineError("password-required", "this document needs its password");
    const generation = this.#generation;
    const clip: PixelRect = request.clip ?? planTile(page, request, DEFAULT_LIMITS).tile;
    const ticket = this.#nextTicket++;
    const url = tileUrl(this.#shell.tileBase, {
      doc: current.doc,
      page: request.page,
      scale: request.scale,
      rotation: request.rotation ?? 0,
      appearance: request.appearance,
      clip,
      visible: (options.priority ?? "visible") === "visible",
      ticket,
      transport: this.#transport,
    });
    const onAbort = () => this.#cancel(ticket);
    signal?.addEventListener("abort", onAbort, { once: true });
    try {
      const response = await this.#shell.fetch(url, signal === undefined ? {} : { signal });
      // A document replaced while its tile travelled: whatever came back is stale, failures too.
      if (this.#generation !== generation) {
        void response.body?.cancel();
        throw abortError();
      }
      if (!response.ok) throw await this.#failure(response);
      const bitmap = await this.#bitmap(response, clip);
      // Aborted while it travelled, or the document went: dropped, never painted.
      if (aborted() || this.#generation !== generation) {
        bitmap.close();
        throw abortError();
      }
      return { bitmap, width: bitmap.width, height: bitmap.height };
    } finally {
      signal?.removeEventListener("abort", onAbort);
    }
  }

  async #bitmap(response: Response, clip: PixelRect): Promise<ImageBitmap> {
    if (this.#transport === "webp") return this.#shell.decode(await response.blob());
    const width = Number(response.headers.get("x-tile-width") ?? clip.width);
    const height = Number(response.headers.get("x-tile-height") ?? clip.height);
    const pixels = new Uint8ClampedArray(await response.arrayBuffer());
    if (pixels.length !== width * height * 4)
      throw new DesktopEngineError("protocol", "a tile came back the wrong size");
    return this.#shell.fromPixels(pixels, width, height);
  }

  /** A failed tile's error: `AbortError` for a cancelled one, else the main process's kind. */
  async #failure(response: Response): Promise<unknown> {
    const body: unknown = await response.json().catch(() => undefined);
    const error = engineError(body ?? { kind: `http-${response.status}`, message: response.statusText });
    return error.code === "cancelled" ? abortError() : error;
  }

  /** Queues `ticket` for one `cancel_tiles` with the others aborted in the same turn. */
  #cancel(ticket: number): void {
    this.#cancelled.push(ticket);
    if (this.#cancelled.length > 1) return;
    queueMicrotask(() => {
      const tickets = this.#cancelled;
      this.#cancelled = [];
      // Best effort: a tile already drawn is simply dropped on arrival.
      this.#shell.invoke("cancel_tiles", { tickets }).catch(() => undefined);
    });
  }

  /** The engine reads no text yet: every page answers as one without text. */
  pageText(_index: number, _options?: ViewerCallOptions): Promise<PageTextData> {
    return Promise.resolve({ runs: [], quads: new Float32Array(0) });
  }

  readingOrder(_index: number, _options?: ViewerCallOptions): Promise<ReadingBlock[]> {
    return Promise.resolve([]);
  }

  structure(_options?: ViewerCallOptions): ReturnType<ViewerEngine["structure"]> {
    return Promise.resolve({ tagged: false });
  }

  pageLinks(_index: number, _options?: ViewerCallOptions): ReturnType<ViewerEngine["pageLinks"]> {
    return Promise.resolve([]);
  }

  find(
    _query: string,
    _find: unknown,
    pages: readonly number[],
    _options?: ViewerCallOptions,
  ): Promise<FindPageResult[]> {
    return Promise.resolve(pages.map((page) => ({ page, matches: [], snippets: [], noText: true as const })));
  }

  /** Forgets document `doc`'s edits: it closed, or reloaded as another document. */
  dropEdits(doc: number): void {
    this.#edits.delete(doc);
    this.#pagesNow.delete(doc);
  }

  /** Whether any open document holds edits not yet saved. */
  get unsaved(): boolean {
    return this.unsavedSteps > 0;
  }

  /** The edits not yet saved, over every open document. */
  get unsavedSteps(): number {
    return [...this.#edits.values()].reduce((sum, edits) => sum + edits.history().unsaved, 0);
  }

  /** The open documents that hold edits not yet saved, by the main process's id. */
  get unsavedDocs(): number[] {
    return [...this.#edits].filter(([, edits]) => edits.history().unsaved > 0).map(([doc]) => doc);
  }

  /** Whether document `doc` holds edits not yet saved. */
  hasUnsaved(doc: number): boolean {
    return (this.#edits.get(doc)?.history().unsaved ?? 0) > 0;
  }

  /**
   * Applies `ops` to the shown document as one transaction, recorded as a step named `label`; the
   * engine draws the pages it touched from the edited file from then on.
   */
  applyEdit(ops: Parameters<ViewerEngine["applyEdit"]>[0], options?: { readonly label?: string }): Promise<EditResult> {
    return this.#edit((edits) => edits.apply(ops, options?.label ?? ops.map((op) => op.op).join(", ")));
  }

  undoEdit(): Promise<EditResult> {
    return this.#edit((edits) => edits.step(-1));
  }

  redoEdit(): Promise<EditResult> {
    return this.#edit((edits) => edits.step(1));
  }

  goToStep(position: number): Promise<EditResult> {
    return this.#edit((edits) => edits.goTo(position));
  }

  editHistory(): Promise<EditHistory> {
    const doc = this.#current?.doc;
    const edits = doc === undefined ? undefined : this.#edits.get(doc);
    return Promise.resolve(edits?.history() ?? NO_EDITS);
  }

  /**
   * Runs one edit session call on the shown document, then keeps its pages as the call left them
   * and tells the viewer (`edit`).
   */
  async #edit(call: (edits: DesktopEdits) => Promise<EditResult>): Promise<EditResult> {
    const current = this.#current;
    if (current === undefined) throw new DesktopEngineError("disposed", "no document is open");
    if (current.needsPassword) throw new DesktopEngineError("password-required", "this document needs its password");
    const { doc } = current;
    let edits = this.#edits.get(doc);
    if (edits === undefined) {
      const run = (request: EditRequest) =>
        this.#shell.invoke<EditOutcome>("edit_document", { doc, request }).then(
          (outcome) => {
            this.#pagesNow.set(doc, outcome.pages);
            return outcome;
          },
          (e: unknown) => {
            throw engineError(e);
          },
        );
      edits = new DesktopEdits(run, pagesOf(current));
      this.#edits.set(doc, edits);
    }
    const result = await call(edits);
    // Another document shows now: its pages and events are its own.
    if (this.#current?.doc !== doc) return result;
    this.#keepPages(doc);
    this.#emit("edit", result);
    return result;
  }

  /** The shown document `doc`'s pages as its last edit left them, in `current` and `document`. */
  #keepPages(doc: number): void {
    const current = this.#current;
    const pages = this.#pagesNow.get(doc);
    if (current?.doc !== doc || pages === undefined) return;
    this.#current = { ...current, pages, pageCount: pages.length };
    if (this.#document !== undefined)
      this.#document = { ...this.#document, pageCount: pages.length, pages: pagesOf({ pages }) };
  }

  /** How a save of the shown document with `options` would go. */
  async planSave(options: SaveOptions): Promise<PlannedSave> {
    const doc = this.#shownDoc();
    const plan = await this.#shell
      .invoke<Parameters<typeof plannedSave>[0]>("plan_save", { doc, options: asked(options) })
      .catch((e: unknown) => {
        throw engineError(e);
      });
    return plannedSave(plan);
  }

  /** What the Properties dialog shows of the shown document, with its edits so far. */
  async properties(): Promise<DesktopProperties> {
    const doc = this.#shownDoc();
    const json = await this.#shell.invoke<string>("document_properties", { doc }).catch((e: unknown) => {
      throw engineError(e);
    });
    return JSON.parse(json) as DesktopProperties;
  }

  wouldViolate(): ReturnType<ViewerEngine["wouldViolate"]> {
    return notYet();
  }

  /** A contents page made from the bookmarks: the engine doesn't make one yet. */
  createContentsPage(): ReturnType<ViewerEngine["createContentsPage"]> {
    return notYet();
  }

  /** A copy with a refused change applied: the engine doesn't plan saves against signatures yet. */
  copyWithEdit(): ReturnType<ViewerEngine["copyWithEdit"]> {
    return notYet();
  }

  /**
   * Saves the shown document to `target` through `edits`, the viewer's session for it, which runs
   * the save after the edits before it and marks its history saved. Undefined when the user
   * cancelled Save as's dialog.
   */
  async saveTo(
    edits: { save(options: SaveOptions, write: () => Promise<void>): Promise<unknown> },
    target: SaveTarget,
    options: SaveOptions,
  ): Promise<DesktopSaved | undefined> {
    const queued: QueuedSave = { doc: this.#shownDoc(), target, saved: undefined };
    this.#saveTargets.push(queued);
    try {
      await edits.save(options, () => Promise.resolve());
      return queued.saved;
    } catch (error) {
      if (error === SAVE_CANCELLED) return undefined;
      throw error;
    } finally {
      // A save the session refused before it ran never took its target.
      const left = this.#saveTargets.indexOf(queued);
      if (left >= 0) this.#saveTargets.splice(left, 1);
    }
  }

  /**
   * The edit session's copy step, which on the desktop is the whole save: the main process writes
   * the file and the engine adopts it ({@link saveTo} says where). The copy holds no bytes, since
   * they never reach the webview, and {@link adoptEdits} marks the history saved.
   */
  async copyEdits(options: SaveOptions): Promise<SavedCopy> {
    const queued = this.#saveTargets.shift();
    if (queued === undefined) throw new DesktopEngineError("unsupported", "the desktop saves through saveTo");
    // The document the save was asked for, whichever tab shows now.
    const { doc } = queued;
    const { command, args } = saveCall(doc, queued.target, options);
    const saved = await this.#shell.invoke<DesktopSaved | null>(command, args).catch((e: unknown) => {
      throw engineError(e);
    });
    if (saved === null) throw SAVE_CANCELLED;
    queued.saved = saved;
    if (this.#current?.doc === doc && saved.name !== this.#current.name)
      this.#current = { ...this.#current, name: saved.name };
    const token = this.#nextSave++;
    this.#adopting.set(token, { doc, historyCleared: saved.historyCleared });
    const report = { metadataNotUpdated: saved.metadataNotUpdated, historyCleared: saved.historyCleared };
    return { token, bytes: new Uint8Array(0), report: report as unknown as SavedCopy["report"] };
  }

  /** The history once the save `token` names is in place: saved at the point it wrote. */
  async adoptEdits(token: number): Promise<EditHistory> {
    const adopting = this.#adopting.get(token);
    this.#adopting.delete(token);
    if (adopting === undefined) throw new DesktopEngineError("unsupported", "no such save");
    return (await this.#edits.get(adopting.doc)?.saved(adopting.historyCleared)) ?? NO_EDITS;
  }

  /** The shown document's id. @throws DesktopEngineError when none is shown. */
  #shownDoc(): number {
    const doc = this.#current?.doc;
    if (doc === undefined) throw new DesktopEngineError("disposed", "no document is open");
    return doc;
  }
}
