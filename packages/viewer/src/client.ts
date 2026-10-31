// The page side of the document worker (C3a, ADR 0009 §3): a ViewerClient owns one worker for one
// document, through `@lamina/pdf/worker`'s client (`request` with a priority and a coalescing
// key). On top of it:
//
// - typed calls (`call(method, params)`, checked against `Methods`), and `open`/`render`/…;
// - cancellation that rejects with an `AbortError` at once; a late answer's bitmap is closed by
//   the worker client, so a cancelled render never paints;
// - render timers on the page (a worker stuck in a synchronous render can't post): `render-slow`
//   after the soft timeout, and after the hard cap either a restart (`hard-timeout`) or, for an
//   encrypted document, `render-stuck` and a restart only on `stop()`;
// - restart after a crash (the worker's error, messageerror or a reported trap; never silence):
//   pending calls reject with `worker-crashed`, a new worker reopens the document from the same
//   source, but the call that was running is never re-sent (the UI offers Retry). An encrypted
//   document isn't reopened: the password lived only in the dead worker, so `password-required`
//   fires and the caller opens again with the password.
import { isLaminaError } from "@lamina/pdf";
import { connect, type WorkerClient, type WorkerInitOptions, type WorkerWasmSource } from "@lamina/pdf/worker";
import type { RenderRequest } from "@lamina/render";
import {
  type BackendInit,
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
export const DEFAULT_TIMEOUTS = Object.freeze({ soft: 10_000, hard: 120_000 });

export interface ViewerOptions {
  /** The render engine; PDFium needs its `.wasm` URL from your bundler (`pdfium.wasm?url`). */
  readonly backend: BackendInit;
  /** Where the worker loads lamina's own WebAssembly from (`@lamina/pdf/worker`'s `init`). */
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
  /** The worker started drawing a render (for the perf harness, C3c). */
  "render-start": { readonly page: number; readonly scale: number };
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
}

export type ViewerEvent = keyof ViewerEvents;

/** Per-call options. */
export interface ViewerCallOptions {
  readonly signal?: AbortSignal;
  /** Default `visible` for renders; document calls run above every render. */
  readonly priority?: Priority;
}

function workerFactory(): Worker {
  return new Worker(new URL("./worker/entry.js", import.meta.url), { type: "module", name: "lamina-viewer" });
}

function abortError(e: unknown): unknown {
  return isLaminaError(e, "cancelled") ? new DOMException("the call was cancelled", "AbortError") : e;
}

/** One document worker and the typed RPC to it. */
export class ViewerClient {
  readonly #pdf: WorkerClient;
  readonly #backend: BackendInit;
  readonly #timeouts: { readonly soft: number; readonly hard: number };
  readonly #listeners = new Map<ViewerEvent, Set<(data: never) => void>>();
  /** What to reopen after a restart: the source and the options without the password. */
  #reopen: { readonly source: SourceInit; readonly options: Omit<ViewerOpenOptions, "password"> } | undefined;
  #document: OpenedDocument | undefined;
  #recovering: Promise<void> | undefined;
  #nextRender = 1;
  #discarded = 0;

  private constructor(pdf: WorkerClient, options: ViewerOptions) {
    this.#pdf = pdf;
    this.#backend = options.backend;
    this.#timeouts = { ...DEFAULT_TIMEOUTS, ...options.timeouts };
    pdf.on("event", (name, data) => {
      if (name === "warning") this.#emit("warning", data as ViewerEvents["warning"]);
      else if (name === "render-start") this.#emit("render-start", data as ViewerEvents["render-start"]);
    });
    pdf.on("crashed", (reason) => {
      this.#emit("crashed", { reason });
      void this.#recover(reason as RestartReason);
    });
  }

  /**
   * Starts the document worker and loads the backend at once, so the WebAssembly downloads while
   * the user picks a file.
   */
  static async create(options: ViewerOptions): Promise<ViewerClient> {
    let client: ViewerClient | undefined;
    const pdf = await connect({
      worker: options.worker ?? workerFactory,
      // A late answer to an aborted render: its bitmap is closed, never painted.
      onDiscard: (value) => {
        const bitmap = (value as { bitmap?: unknown } | undefined)?.bitmap;
        if (bitmap instanceof ImageBitmap) {
          bitmap.close();
          if (client !== undefined) client.#discarded++;
        }
      },
      ...(options.signal === undefined ? {} : { signal: options.signal }),
      ...(options.lamina === undefined ? {} : { init: options.lamina }),
    });
    client = new ViewerClient(pdf, options);
    await pdf.request("initBackend", [options.backend], { priority: PRIORITY.document });
    return client;
  }

  /** Listens for `event`; returns a function that stops listening. */
  on<E extends ViewerEvent>(event: E, listener: (data: ViewerEvents[E]) => void): () => void {
    let set = this.#listeners.get(event);
    if (set === undefined) {
      set = new Set();
      this.#listeners.set(event, set);
    }
    const l = listener as (data: never) => void;
    set.add(l);
    return () => set.delete(l);
  }

  #emit<E extends ViewerEvent>(event: E, data: ViewerEvents[E]): void {
    for (const listener of this.#listeners.get(event) ?? []) (listener as (d: ViewerEvents[E]) => void)(data);
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
   * Opens `source` in the worker (lamina and the backend, from the same bytes). The password is
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

  /** @internal Bitmaps of late answers closed unpainted (the cancellation tests read it). */
  get discardedBitmaps(): number {
    return this.#discarded;
  }

  /** @internal What a restart reopens with (the tests check it holds no password). */
  get reopenState(): unknown {
    return this.#reopen;
  }

  async close(): Promise<void> {
    this.#reopen = undefined;
    this.#document = undefined;
    await this.call("closeDocument", []);
  }

  pageInfo(index: number, options?: ViewerCallOptions) {
    return this.call("pageInfo", [index], options);
  }

  /**
   * Renders `request` as an ImageBitmap (close it when done). Identical requests in flight are
   * drawn once. An aborted render rejects with `AbortError` and never paints.
   */
  render(request: RenderRequest, options: ViewerCallOptions = {}): Promise<RenderedBitmap> {
    const id = this.#nextRender++;
    const needsPassword = this.#document?.needsPassword === true;
    const soft = setTimeout(() => this.#emit("render-slow", { id, page: request.page }), this.#timeouts.soft);
    const hard = setTimeout(() => {
      if (needsPassword) this.#emit("render-stuck", { id, page: request.page, needsPassword: true });
      else void this.stop("hard-timeout");
    }, this.#timeouts.hard);
    return this.call("render", [request], { ...options, key: renderKey(request) }).finally(() => {
      clearTimeout(soft);
      clearTimeout(hard);
    });
  }

  /**
   * Stops whatever the worker is doing by terminating it (the user's Stop, or the hard cap), then
   * restarts it as after a crash: pending calls reject, the running one isn't re-sent, and an
   * encrypted document asks for its password again.
   */
  stop(reason: "cancelled" | "hard-timeout" = "cancelled"): Promise<void> {
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
          this.#emit("password-required", {});
        } else {
          this.#document = (await this.#pdf.request("openDocument", [reopen.source, reopen.options], {
            priority: PRIORITY.document,
          })) as OpenedDocument;
          reopened = true;
        }
      }
      this.#emit("restarted", { reason, reopened });
    })();
    const tracked = recovering.finally(() => {
      if (this.#recovering === tracked) this.#recovering = undefined;
    });
    this.#recovering = tracked;
    return tracked;
  }
}
