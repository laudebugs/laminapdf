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
import { isLaminaPdfError } from "@laminapdf/sdk";
import {
  connect,
  Emitter,
  type WorkerClient,
  type WorkerInitOptions,
  type WorkerWasmSource,
} from "@laminapdf/sdk/worker";
import {
  type BackendInit,
  type LoadProgress,
  type MethodName,
  type OpenedDocument,
  type Params,
  PRIORITY,
  type Priority,
  type RenderedBitmap,
  type Result,
  renderKey,
  type SourceInit,
  type ViewerOpenOptions,
} from "./protocol.js";

/** Default render timeouts (ADR 0009 §3.7), in ms. */
const DEFAULT_TIMEOUTS = Object.freeze({ soft: 10_000, hard: 120_000 });

/** `ViewerClient.create`'s options: the backend, Lamina PDF's WebAssembly, the timeouts, the worker and a signal to stop starting. */
export interface ViewerOptions {
  /** The render engine; PDFium needs its `.wasm` URL from your bundler (`pdfium.wasm?url`). */
  readonly backend: BackendInit;
  /** Where the worker loads Lamina PDF's own WebAssembly from (`@laminapdf/sdk/worker`'s `init`). */
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
  /** A render passed the soft timeout: offer Stop. */
  "render-slow": { readonly id: number; readonly page: number };
  /** An encrypted document's render passed the hard cap: `stop()` would ask for the password again. */
  "render-stuck": { readonly id: number; readonly page: number; readonly needsPassword: true };
  /** The worker crashed (not on `stop()`). */
  crashed: { readonly reason: string };
  /** A new worker serves again (and has reopened the document, unless it needed a password). */
  restarted: { readonly reason: RestartReason; readonly reopened: boolean };
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

/** One document worker and the typed RPC to it. */
export class ViewerClient {
  readonly #pdf: WorkerClient;
  readonly #backend: BackendInit;
  readonly #timeouts: { readonly soft: number; readonly hard: number };
  readonly #events = new Emitter<ViewerEvents>();
  /** What to reopen after a restart: the source and the options without the password. */
  #reopen: { readonly source: SourceInit; readonly options: Omit<ViewerOpenOptions, "password"> } | undefined;
  #document: OpenedDocument | undefined;
  #recovering: Promise<void> | undefined;
  #nextRender = 1;
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
      await pdf.request("initBackend", [options.backend], { priority: PRIORITY.document });
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
    const level = method === "render" ? PRIORITY[priority ?? "visible"] : PRIORITY.document;
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
    const opened = await this.call("openDocument", [source, options], call);
    this.#reopen = { source, options: rest };
    this.#document = opened;
    return opened;
  }

  /** The open document, if any. */
  get document(): OpenedDocument | undefined {
    return this.#document;
  }

  /** Closes the document in the worker (the worker stays, ready for another `open`). */
  async close(): Promise<void> {
    this.#reopen = undefined;
    this.#document = undefined;
    await this.call("closeDocument", []);
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
      return await this.call("render", [request, documentId], { ...options, key });
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
    this.#pdf.terminate();
  }

  #recover(reason: RestartReason): Promise<void> {
    if (this.#recovering !== undefined) return this.#recovering;
    const recovering = (async () => {
      await this.#pdf.restart();
      await this.#pdf.request("initBackend", [this.#backend], { priority: PRIORITY.document });
      let reopened = false;
      const reopen = this.#reopen;
      if (reopen !== undefined) {
        if (this.#document?.needsPassword === true) {
          // The password died with the worker; it is never kept on the page.
          this.#reopen = undefined;
          this.#document = undefined;
          this.#events.emit("password-required", {});
        } else {
          this.#document = (await this.#pdf.request("openDocument", [reopen.source, reopen.options], {
            priority: PRIORITY.document,
          })) as OpenedDocument;
          reopened = true;
        }
      }
      this.#events.emit("restarted", { reason, reopened });
    })();
    const tracked = recovering.finally(() => {
      if (this.#recovering === tracked) this.#recovering = undefined;
    });
    this.#recovering = tracked;
    return tracked;
  }
}
