// The page side of `@laminapdf/sdk/worker`: `connect()` starts the worker (the literal
// `new Worker(new URL(…, import.meta.url))` lets Vite and webpack bundle it), checks the
// handshake, and returns a WorkerClient whose documents and pages are remote handles: proxies
// whose methods post typed calls. Calls get monotonically increasing IDs; an aborted call is
// rejected at once (`cancelled`, with the signal's reason as `cause`) and a `cancel` is posted;
// a late answer is dropped (the worker frees a document it opened when that cancel arrives).
// Errors come back as ADR 0026 objects and are rebuilt as LaminaPdfError.
// A worker `error` or `messageerror`, or a trap the worker reports, rejects every pending call
// with `worker-crashed` and emits `crashed`; `restart()` starts a new worker. No heartbeat.

import { defined } from "../defined.js";
import { type Diagnostic, LaminaPdfError, packageError } from "../errors.js";
import type { ErrorCode } from "../generated/codes.js";
import type { HttpRangeOptions } from "../source/http.js";
import type { RangeOptions } from "../source/range.js";
import { PageText } from "../text.js";
import type { OpenOptions, Progress } from "../types.js";
import { Emitter } from "./emitter.js";
import {
  type FromWorker,
  HANDSHAKE,
  type HandleRef,
  type Handshake,
  isHandleRef,
  isPageTextData,
  type Request,
  type ToWorker,
} from "./protocol.js";
import { DOCUMENT_METHODS, PAGE_METHODS } from "./support.js";
import { whole } from "./transfer.js";
import type { CallOptions, RemoteDocument, RequestOptions } from "./types.js";

/** `Symbol.asyncDispose` where the runtime has it, else the registered symbol (no globals touched). */
const ASYNC_DISPOSE: symbol = (Symbol as { asyncDispose?: symbol }).asyncDispose ?? Symbol.for("Symbol.asyncDispose");

/**
 * `openUrl`'s options besides `OpenOptions`: `httpRangeSource`'s without a custom `fetch` (a
 * function can't reach the worker), and `openRange`'s (`budget`, `readahead`, `maxRounds`).
 * `signal` is the call's (`CallOptions`), and cancels the fetches in flight too.
 *
 * @since 0.2.0
 */
export type RemoteHttpOptions = Omit<HttpRangeOptions, "fetch" | "signal"> & Omit<RangeOptions, "signal">;

/**
 * Where the worker loads a WebAssembly module from: what can be sent to a worker. A string or
 * `URL` is resolved against the page (not the worker file) before it is sent. A `Response`
 * can't cross: pass its bytes, or a `WebAssembly.Module`.
 *
 * @since 0.2.0
 */
export type WorkerWasmSource = string | URL | ArrayBuffer | ArrayBufferView | WebAssembly.Module;

/**
 * `init()`'s options, with sources the worker can receive.
 *
 * @since 0.2.0
 */
export interface WorkerInitOptions {
  readonly module?: WorkerWasmSource;
  readonly text?: boolean | WorkerWasmSource;
  readonly extract?: boolean | WorkerWasmSource;
}

/** `source` as sent to the worker: URLs made absolute against the page. */
function sendable<T>(source: T): T | string {
  if (source instanceof URL) return source.href;
  if (typeof source === "string") {
    const base = (globalThis as { location?: { href: string } }).location?.href;
    return base === undefined ? source : new URL(source, base).href;
  }
  return source;
}

function sendableInit(init: WorkerWasmSource | WorkerInitOptions): unknown {
  if (
    typeof init === "string" ||
    init instanceof URL ||
    init instanceof ArrayBuffer ||
    ArrayBuffer.isView(init) ||
    init instanceof WebAssembly.Module
  ) {
    return sendable(init);
  }
  return Object.fromEntries(
    Object.entries(defined(init)).map(([k, v]) => [k, typeof v === "boolean" ? v : sendable(v)]),
  );
}

/**
 * `connect()`'s options: the worker to start, a signal to stop connecting, the WebAssembly to load, and what to do with late answers.
 *
 * @since 0.2.0
 */
export interface ConnectOptions {
  /**
   * The worker to use, or a function that makes one (needed for `restart()`). Omitted, the
   * package's own worker file. Pass one for a CSP that forbids the bundled URL, or to serve the
   * worker yourself (from the same `@laminapdf/sdk` build: the handshake checks).
   */
  readonly worker?: Worker | (() => Worker);
  /** Aborts connecting. */
  readonly signal?: AbortSignal;
  /** What the worker passes to `init()`: where the WebAssembly comes from, and options. */
  readonly init?: WorkerWasmSource | WorkerInitOptions;
  /**
   * Called with each answer that arrives for a call already rejected (aborted, or after a crash
   * or terminate), which nobody will read. The default closes the ImageBitmaps in it (at the top
   * level or one level down), so a cancelled render's bitmap is freed, never painted.
   */
  readonly onDiscard?: (value: unknown) => void;
}

/** Closes the ImageBitmaps in `value` and its direct members. */
function closeBitmaps(value: unknown): void {
  const close = (v: unknown) => {
    if (typeof ImageBitmap !== "undefined" && v instanceof ImageBitmap) v.close();
  };
  close(value);
  if (typeof value === "object" && value !== null) for (const v of Object.values(value)) close(v);
}

type EventListener = (name: string, data: unknown) => void;

interface Pending {
  readonly resolve: (value: unknown) => void;
  readonly reject: (error: unknown) => void;
  readonly onProgress: ((progress: Progress) => void) | undefined;
}

type CrashListener = (reason: string) => void;

function defaultWorker(): Worker {
  return new Worker(new URL("./entry.js", import.meta.url), { type: "module", name: "lamina" });
}

function rebuild(error: Diagnostic<ErrorCode>): LaminaPdfError {
  return new LaminaPdfError(error);
}

function crashed(message: string, reason: string): LaminaPdfError {
  return packageError("worker-crashed", message, { data: { reason } });
}

/**
 * Starts the worker and waits for its handshake.
 *
 * @since 0.2.0
 */
export async function connect(options: ConnectOptions = {}): Promise<WorkerClient> {
  const client = new WorkerClient(options);
  await client.start(options.signal);
  return client;
}

/**
 * A connection to one Lamina PDF worker.
 *
 * @since 0.2.0
 */
export class WorkerClient {
  readonly #factory: () => Worker;
  readonly #init: WorkerWasmSource | WorkerInitOptions | undefined;
  readonly #pending = new Map<number, Pending>();
  readonly #emitter = new Emitter<{ crashed: string; event: { name: string; data: unknown } }>();
  readonly #onDiscard: (value: unknown) => void;
  /** The worker once its handshake succeeded; undefined while starting, stopped or crashed. */
  #worker: Worker | undefined;
  /** The handshake in progress (calls made meanwhile wait for it), and how `#stop` ends it. */
  #starting: Promise<void> | undefined;
  #cancelStart: ((error: unknown) => void) | undefined;
  /**
   * Bumped for each worker: a handle remembers the generation it was made in, so a handle of a
   * worker that's gone is `disposed` instead of reaching the new worker's objects.
   */
  #generation = 0;
  #nextId = 1;

  /** @internal Use {@link connect}. */
  constructor(options: ConnectOptions) {
    const { worker } = options;
    if (typeof worker === "function") {
      this.#factory = worker;
    } else {
      // A given Worker serves the first start; restart() then needs a factory.
      let given = worker;
      this.#factory = () => {
        if (given !== undefined) {
          const w = given;
          given = undefined;
          return w;
        }
        if (worker !== undefined) {
          throw crashed("restart() needs a worker factory: pass `worker: () => new Worker(…)`", "terminated");
        }
        return defaultWorker();
      };
    }
    this.#init = options.init;
    this.#onDiscard = options.onDiscard ?? closeBitmaps;
  }

  /** @internal Starts a worker and checks its handshake. */
  start(signal?: AbortSignal): Promise<void> {
    const starting = this.#handshake(signal).finally(() => {
      if (this.#starting === starting) this.#starting = undefined;
    });
    this.#starting = starting;
    return starting;
  }

  async #handshake(signal: AbortSignal | undefined): Promise<void> {
    if (signal?.aborted) throw packageError("cancelled", "connecting was cancelled", { cause: signal.reason });
    const generation = ++this.#generation;
    const worker = this.#factory();
    let onAbort: (() => void) | undefined;
    try {
      const welcome = await new Promise<{ worker: Handshake; error?: Diagnostic<ErrorCode> }>((resolve, reject) => {
        onAbort = () => reject(packageError("cancelled", "connecting was cancelled", { cause: signal?.reason }));
        signal?.addEventListener("abort", onAbort, { once: true });
        this.#cancelStart = reject;
        worker.onmessage = (event: MessageEvent<FromWorker>) => {
          if (event.data.kind === "welcome") resolve(event.data);
        };
        worker.onerror = (event) => {
          event.preventDefault();
          reject(crashed("the worker failed to start", "error"));
        };
        worker.onmessageerror = () => reject(crashed("the worker's answer couldn't be read", "messageerror"));
        const hello: ToWorker = {
          kind: "hello",
          client: HANDSHAKE,
          ...(this.#init === undefined ? {} : { init: sendableInit(this.#init) }),
        };
        try {
          worker.postMessage(hello);
        } catch (e) {
          reject(packageError("invalid-argument", "connect()'s `init` can't be sent to the worker", { cause: e }));
        }
      });
      if (welcome.worker.protocol !== HANDSHAKE.protocol || welcome.worker.version !== HANDSHAKE.version) {
        throw packageError("worker-version-mismatch", "the worker is from another @laminapdf/sdk build", {
          data: { expected: HANDSHAKE, actual: welcome.worker },
        });
      }
      if (welcome.error !== undefined) throw rebuild(welcome.error);
    } catch (e) {
      worker.terminate();
      throw e;
    } finally {
      this.#cancelStart = undefined;
      if (onAbort !== undefined) signal?.removeEventListener("abort", onAbort);
    }
    if (generation !== this.#generation) {
      // terminate() or another start() came first: this worker is not wanted any more.
      worker.terminate();
      throw crashed("the worker was stopped while it started", "terminated");
    }
    worker.onmessage = (event: MessageEvent<FromWorker>) => this.#receive(event.data);
    worker.onerror = (event) => {
      event.preventDefault();
      this.#crash("error");
    };
    worker.onmessageerror = () => this.#crash("messageerror");
    this.#worker = worker;
  }

  #receive(message: FromWorker): void {
    switch (message.kind) {
      case "result": {
        const pending = this.#pending.get(message.id);
        // A late answer to a call already rejected (aborted or crashed) is dropped.
        if (pending === undefined) {
          if (message.ok) this.#onDiscard(message.result);
          return;
        }
        this.#pending.delete(message.id);
        if (message.ok) pending.resolve(message.result);
        else pending.reject(rebuild(message.error));
        return;
      }
      case "event":
        this.#emitter.emit("event", { name: message.name, data: message.data });
        return;
      case "progress":
        this.#pending.get(message.id)?.onProgress?.(message.progress as Progress);
        return;
      case "crashed":
        this.#crash(message.reason);
        return;
      case "welcome":
        return;
    }
  }

  /** Ends the current worker and rejects its pending calls; `notify` fires the `crashed` listeners. */
  #stop(reason: string, notify: boolean): void {
    const worker = this.#worker;
    this.#worker = undefined;
    // Handles of this worker, and a start in progress, are now stale.
    this.#generation++;
    worker?.terminate();
    // A worker still in its handshake (a stalled `init()`) is ended too.
    this.#cancelStart?.(crashed("the worker was stopped while it started", reason));
    const pending = [...this.#pending.values()];
    this.#pending.clear();
    for (const p of pending) p.reject(crashed("the worker stopped", reason));
    if (notify) this.#emitter.emit("crashed", reason);
  }

  #crash(reason: string): void {
    if (this.#worker === undefined) return;
    this.#stop(reason, true);
  }

  /**
   * Listens for the worker crashing (`reason`: `error`, `messageerror` or `trap`; not `terminate()`).
   *
   * @since 0.2.0
   */
  on(event: "crashed", listener: CrashListener): () => void;
  /** Listens for an extension's events (`emit()` in the worker). */
  on(event: "event", listener: EventListener): () => void;
  on(event: "crashed" | "event", listener: CrashListener | EventListener): () => void {
    if (event === "crashed") return this.#emitter.on("crashed", listener as CrashListener);
    if (event === "event") {
      const l = listener as EventListener;
      return this.#emitter.on("event", ({ name, data }) => l(name, data));
    }
    throw new TypeError(`unknown event ${String(event)}`);
  }

  /**
   * Calls an extension method in the worker (for packages that extend it with `serve(…)`, such
   * as `@laminapdf/viewer`). Same cancellation, transfers and errors as every other call.
   *
   * @since 0.2.0
   */
  request(method: string, params: readonly unknown[], options: RequestOptions = {}): Promise<unknown> {
    return this.call(0, method, params, options);
  }

  /**
   * Ends the worker; pending calls reject with `worker-crashed` (no `crashed` event).
   *
   * @since 0.2.0
   */
  terminate(): void {
    this.#stop("terminated", false);
  }

  /**
   * Starts a new worker (after a crash, or to reset); handles of the old one are `disposed`.
   *
   * @since 0.2.0
   */
  async restart(signal?: AbortSignal): Promise<void> {
    this.terminate();
    await this.start(signal);
  }

  /** @internal The current worker generation (handles record it). */
  get generation(): number {
    return this.#generation;
  }

  /**
   * @internal Posts a call and waits for its answer. `generation`, for a handle, is the one it
   * was made in. Calls made while a worker starts wait for its handshake.
   */
  async call(
    handle: number,
    method: string,
    params: readonly unknown[],
    options: CallOptions = {},
    generation?: number,
  ): Promise<unknown> {
    if (this.#worker === undefined && this.#starting !== undefined) {
      // The start's own error reaches its caller; here, the call just finds no worker. The
      // call's signal still rejects it meanwhile.
      await waitWithSignal(
        this.#starting.catch(() => {}),
        options.signal,
      );
    }
    const worker = this.#worker;
    if (worker === undefined) {
      throw crashed("the worker isn't running: call restart()", "terminated");
    }
    if (generation !== undefined && generation !== this.#generation) {
      throw packageError("disposed", "the handle belongs to a worker that has stopped");
    }
    const { signal, onProgress, transfer } = options;
    if (signal?.aborted) {
      throw packageError("cancelled", "the call was cancelled", { cause: signal.reason });
    }
    const id = this.#nextId++;
    return new Promise<unknown>((resolve, reject) => {
      const onAbort = () => {
        if (!this.#pending.delete(id)) return;
        worker.postMessage({ kind: "cancel", id } satisfies ToWorker);
        reject(packageError("cancelled", "the call was cancelled", { cause: signal?.reason }));
      };
      const settle =
        <T>(f: (value: T) => void) =>
        (value: T) => {
          signal?.removeEventListener("abort", onAbort);
          f(value);
        };
      signal?.addEventListener("abort", onAbort, { once: true });
      this.#pending.set(id, { resolve: settle(resolve), reject: settle(reject), onProgress });
      const { priority, key } = options as RequestOptions;
      const request: Request = {
        kind: "call",
        id,
        handle,
        method,
        params,
        ...(priority === undefined ? {} : { priority }),
        ...(key === undefined ? {} : { key }),
      };
      // Only a buffer the parameter covers whole moves; a subarray or a shared buffer is copied.
      const buffers = transfer === true ? params.flatMap(whole) : [];
      try {
        worker.postMessage(request, buffers as Transferable[]);
      } catch (e) {
        // A parameter that can't be cloned or transferred: nothing reached the worker.
        this.#pending
          .get(id)
          ?.reject(
            packageError("invalid-argument", `a parameter of ${method} can't be sent to the worker`, { cause: e }),
          );
        this.#pending.delete(id);
      }
    });
  }

  /**
   * Opens `bytes` in the worker. They are copied unless `transfer: true` (then the caller's
   * buffer is detached). Opening is one synchronous call there: `signal` drops it if it hasn't
   * started, but can't stop it midway (`terminate()` is the hard stop). An open aborted midway
   * still rejects with `cancelled`, and the worker frees the document it opened.
   *
   * @since 0.2.0
   */
  async open(bytes: Uint8Array | ArrayBuffer, options: OpenOptions & CallOptions = {}): Promise<RemoteDocument> {
    const { signal, onProgress, transfer, ...open } = options;
    const ref = await this.call(0, "open", [bytes, open], defined({ signal, onProgress, transfer }));
    return this.#remoteDocument(ref as HandleRef);
  }

  /**
   * Opens a remote file by HTTP range requests: the worker fetches what each call needs.
   *
   * @since 0.2.0
   */
  async openUrl(url: string, options: OpenOptions & RemoteHttpOptions & CallOptions = {}): Promise<RemoteDocument> {
    const { signal, onProgress, transfer: _, ...open } = options;
    const ref = await this.call(0, "openUrl", [url, open], defined({ signal, onProgress }));
    return this.#remoteDocument(ref as HandleRef);
  }

  /**
   * Loads the text module in the worker (`loadText()`).
   *
   * @since 0.2.0
   */
  async loadText(options?: CallOptions): Promise<void> {
    await this.call(0, "loadText", [], options);
  }

  /**
   * Loads the extract module in the worker (`loadExtract()`), for `page.text()`.
   *
   * @since 0.2.0
   */
  async loadExtract(options?: CallOptions): Promise<void> {
    await this.call(0, "loadExtract", [], options);
  }

  #remoteDocument(ref: HandleRef): RemoteDocument {
    // A stop rejects every pending call, so an answer comes from the current worker.
    const generation = this.#generation;
    let closed = false;
    // Idempotent, like Document.free(): a second close, or one after the worker stopped, is a no-op.
    const close = async () => {
      if (closed) return;
      closed = true;
      if (generation !== this.#generation || this.#worker === undefined) return;
      await this.call(ref.$handle, "close", [], {}, generation);
    };
    const incoming = (value: unknown): unknown => {
      if (isHandleRef(value)) return remote(this, value.$handle, generation, PAGE_METHODS, undefined, incoming);
      if (isPageTextData(value)) return new PageText(value.$pageText as never);
      return value;
    };
    return remote(this, ref.$handle, generation, DOCUMENT_METHODS, close, incoming) as RemoteDocument;
  }
}

/** Waits for `p`, or rejects with `cancelled` when `signal` aborts first. */
async function waitWithSignal(p: Promise<unknown>, signal: AbortSignal | undefined): Promise<void> {
  if (signal === undefined) {
    await p;
    return;
  }
  let onAbort: (() => void) | undefined;
  const aborted = new Promise<never>((_, reject) => {
    onAbort = () => reject(packageError("cancelled", "the call was cancelled", { cause: signal.reason }));
    if (signal.aborted) onAbort();
    else signal.addEventListener("abort", onAbort, { once: true });
  });
  try {
    await Promise.race([p, aborted]);
  } finally {
    if (onAbort !== undefined) signal.removeEventListener("abort", onAbort);
  }
}

/** A proxy whose members (by `arity`) post calls on `handle` of worker `generation`. */
function remote(
  client: WorkerClient,
  handle: number,
  generation: number,
  arity: Readonly<Record<string, number>>,
  close: (() => Promise<void>) | undefined,
  incoming: (value: unknown) => unknown,
): object {
  return new Proxy(Object.create(null) as object, {
    get(_, key) {
      if (key === ASYNC_DISPOSE || (close !== undefined && key === "close")) return close;
      if (typeof key !== "string" || !Object.hasOwn(arity, key)) return undefined;
      const n = arity[key] ?? 0;
      return async (...args: unknown[]) => {
        const options = (args.length > n ? args[n] : undefined) as CallOptions | undefined;
        return incoming(await client.call(handle, key, args.slice(0, n), options, generation));
      };
    },
    has(_, key) {
      return (
        key === ASYNC_DISPOSE ||
        (typeof key === "string" && (Object.hasOwn(arity, key) || (close !== undefined && key === "close")))
      );
    },
  });
}
