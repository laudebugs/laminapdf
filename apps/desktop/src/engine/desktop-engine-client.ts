// The desktop app's engine client (ADR 0008): the `ViewerEngine` `@laminapdf/viewer` draws from,
// over the main process and its engine process instead of a document worker. The viewer's code
// runs unchanged; only this client differs from the web reader's `ViewerClient`.
//
// - Tiles come over the `lamina://` scheme as lossless WebP (raw RGBA as the fallback); the
//   webview fetches them, and an aborted render's ticket goes to `cancel_tiles`, so the engine
//   skips it. A tile that arrives after its render was aborted, or after its document went, is
//   closed and never painted.
// - The main process restarts a crashed engine and reopens its documents, with their kept
//   passwords (D2b): `engine://restarted` becomes the viewer's `restarted`, which redraws what is
//   on screen; `engine://failed` and a document that didn't come back become `restart-failed`.
// - Text, links, search and editing aren't served by the engine yet: their calls answer as for
//   a document without them (no text runs, no links, untagged), and edits reject.

import { DEFAULT_LIMITS, type PixelRect, planTile, type RenderRequest } from "@laminapdf/render";
import type {
  EditHistory,
  EditResult,
  FindPageResult,
  OpenedDocument,
  PageTextData,
  ReadingBlock,
  RenderedBitmap,
  ViewerCallOptions,
  ViewerEngine,
  ViewerEvent,
  ViewerEvents,
} from "@laminapdf/viewer";
import { EMPTY_HISTORY } from "@laminapdf/viewer";
import { type TileTransport, tileUrl } from "./tile-url.ts";

/** A page as the main process reports it: displayed size in points and `/Rotate`. */
export interface DesktopPage {
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
export type Reopened = "open" | "locked" | "closed";

/** `engine://restarted`'s payload. */
export interface RestartedPayload {
  readonly window: string;
  readonly docs: readonly { readonly doc: number; readonly state: Reopened }[];
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

const NOT_YET = "the desktop reader doesn't edit documents yet";

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
  #shown = 0;
  #nextTicket = 1;
  /** Tickets of aborted renders, sent together in one `cancel_tiles`. */
  #cancelled: number[] = [];

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

  /** Shows `opened`: the viewer's document for it, with an ID of its own. */
  show(opened: DesktopDocument): OpenedDocument {
    this.#shown += 1;
    this.#current = opened;
    this.#document = {
      id: this.#shown,
      pageCount: opened.pageCount,
      pages: opened.pages.map((page, index) => ({
        index,
        id: `page-${index}`,
        width: page.width,
        height: page.height,
        rotation: page.rotate,
      })),
      needsPassword: opened.needsPassword,
      backend: { name: "pdfium", version: "desktop" },
      encrypted: opened.encrypted,
    };
    return this.#document;
  }

  /** Forgets the shown document (its tiles in flight are dropped) and closes it in the main process. */
  async close(): Promise<void> {
    const doc = this.#current?.doc;
    this.#current = undefined;
    this.#document = undefined;
    this.#shown += 1;
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

  /** The engine came back: the viewer redraws the document if it reopened, else the UI is told. */
  #restarted(payload: RestartedPayload): void {
    const doc = this.#current?.doc;
    const state = payload.docs.find((entry) => entry.doc === doc)?.state;
    if (state === "open" || (state === undefined && doc === undefined)) {
      this.#emit("restarted", { reason: "error", reopened: doc !== undefined });
    } else if (state === "locked") {
      this.#emit("password-required", {});
    } else {
      const error = new DesktopEngineError("closed", "the document didn't reopen after its process restarted");
      this.#emit("restart-failed", { reason: "error", error });
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
    const shown = this.#shown;
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
      if (!response.ok) throw await this.#failure(response);
      const bitmap = await this.#bitmap(response, clip);
      // Aborted while it travelled, or the document went: dropped, never painted.
      if (aborted() || this.#shown !== shown) {
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

  applyEdit(): Promise<EditResult> {
    return Promise.reject(new DesktopEngineError("not-yet-supported", NOT_YET));
  }

  undoEdit(): Promise<EditResult> {
    return Promise.reject(new DesktopEngineError("not-yet-supported", NOT_YET));
  }

  redoEdit(): Promise<EditResult> {
    return Promise.reject(new DesktopEngineError("not-yet-supported", NOT_YET));
  }

  goToStep(): Promise<EditResult> {
    return Promise.reject(new DesktopEngineError("not-yet-supported", NOT_YET));
  }

  editHistory(): Promise<EditHistory> {
    return Promise.resolve(EMPTY_HISTORY);
  }
}
