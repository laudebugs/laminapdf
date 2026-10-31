// The page side of `@lamina/pdf/worker` (L2a): `connect()` starts the worker (the literal
// `new Worker(new URL(…, import.meta.url))` lets Vite and webpack bundle it), checks the
// handshake, and returns a WorkerClient whose documents and pages are remote handles: proxies
// whose methods post typed calls. Calls get monotonically increasing IDs; an aborted call is
// rejected at once (`cancelled`, with the signal's reason as `cause`) and a `cancel` is posted;
// a late answer is dropped. Errors come back as ADR 0026 objects and are rebuilt as LaminaError.
// A worker `error` or `messageerror`, or a trap the worker reports, rejects every pending call
// with `worker-crashed` and emits `crashed`; `restart()` starts a new worker. No heartbeat.
import { type Diagnostic, LaminaError, packageError } from "../errors.js";
import type { ErrorCode } from "../generated/codes.js";
import type { InitOptions, WasmSource } from "../init.js";
import type { HttpRangeOptions } from "../source/http.js";
import { PageText } from "../text.js";
import type { OpenOptions, Progress } from "../types.js";
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
import type { CallOptions, RemoteDocument } from "./types.js";

/** `Symbol.asyncDispose` where the runtime has it, else the registered symbol (no globals touched). */
const ASYNC_DISPOSE: symbol = (Symbol as { asyncDispose?: symbol }).asyncDispose ?? Symbol.for("Symbol.asyncDispose");

/** `openUrl`'s HTTP options: everything but a custom `fetch` (a function can't reach the worker). */
export type RemoteHttpOptions = Omit<HttpRangeOptions, "fetch" | "signal">;

export interface ConnectOptions {
  /**
   * The worker to use, or a function that makes one (needed for `restart()`). Omitted, the
   * package's own worker file. Pass one for a CSP that forbids the bundled URL, or to serve the
   * worker yourself (from the same `@lamina/pdf` build: the handshake checks).
   */
  readonly worker?: Worker | (() => Worker);
  /** Aborts connecting. */
  readonly signal?: AbortSignal;
  /** What the worker passes to `init()`: where the WebAssembly comes from, and options. */
  readonly init?: WasmSource | InitOptions;
}

interface Pending {
  readonly resolve: (value: unknown) => void;
  readonly reject: (error: unknown) => void;
  readonly onProgress: ((progress: Progress) => void) | undefined;
}

type CrashListener = (reason: string) => void;

function defaultWorker(): Worker {
  return new Worker(new URL("./entry.js", import.meta.url), { type: "module", name: "lamina" });
}

function rebuild(error: Diagnostic<ErrorCode>): LaminaError {
  return new LaminaError(error);
}

function crashed(message: string, reason: string): LaminaError {
  return packageError("worker-crashed", message, { data: { reason } });
}

/** Starts the worker and waits for its handshake. */
export async function connect(options: ConnectOptions = {}): Promise<WorkerClient> {
  const client = new WorkerClient(options);
  await client.start(options.signal);
  return client;
}

/** A connection to one lamina worker. */
export class WorkerClient {
  readonly #factory: () => Worker;
  readonly #init: WasmSource | InitOptions | undefined;
  readonly #pending = new Map<number, Pending>();
  readonly #listeners = new Set<CrashListener>();
  /** The worker once its handshake succeeded; undefined while starting, stopped or crashed. */
  #worker: Worker | undefined;
  /** The handshake in progress (calls made meanwhile wait for it). */
  #starting: Promise<void> | undefined;
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
        worker.onmessage = (event: MessageEvent<FromWorker>) => {
          if (event.data.kind === "welcome") resolve(event.data);
        };
        worker.onerror = (event) => {
          event.preventDefault();
          reject(crashed("the worker failed to start", "error"));
        };
        const hello: ToWorker = {
          kind: "hello",
          client: HANDSHAKE,
          ...(this.#init === undefined ? {} : { init: this.#init }),
        };
        worker.postMessage(hello);
      });
      if (welcome.worker.protocol !== HANDSHAKE.protocol || welcome.worker.version !== HANDSHAKE.version) {
        throw packageError("worker-version-mismatch", "the worker is from another @lamina/pdf build", {
          data: { expected: HANDSHAKE, actual: welcome.worker },
        });
      }
      if (welcome.error !== undefined) throw rebuild(welcome.error);
    } catch (e) {
      worker.terminate();
      throw e;
    } finally {
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
        if (pending === undefined) return;
        this.#pending.delete(message.id);
        if (message.ok) pending.resolve(message.result);
        else pending.reject(rebuild(message.error));
        return;
      }
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
    const pending = [...this.#pending.values()];
    this.#pending.clear();
    for (const p of pending) p.reject(crashed("the worker stopped", reason));
    if (notify) for (const listener of this.#listeners) listener(reason);
  }

  #crash(reason: string): void {
    if (this.#worker === undefined) return;
    this.#stop(reason, true);
  }

  /** Listens for the worker crashing (`reason`: `error`, `messageerror` or `trap`; not `terminate()`). */
  on(event: "crashed", listener: CrashListener): () => void {
    if (event !== "crashed") throw new TypeError(`unknown event ${String(event)}`);
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Ends the worker; pending calls reject with `worker-crashed` (no `crashed` event). */
  terminate(): void {
    this.#stop("terminated", false);
  }

  /** Starts a new worker (after a crash, or to reset); handles of the old one are `disposed`. */
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
      // The start's own error reaches its caller; here, the call just finds no worker.
      await this.#starting.catch(() => {});
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
      const request: Request = { kind: "call", id, handle, method, params };
      const buffers =
        transfer === true
          ? params.flatMap((p) => (ArrayBuffer.isView(p) ? [p.buffer] : p instanceof ArrayBuffer ? [p] : []))
          : [];
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
   * started, but can't stop it midway (`terminate()` is the hard stop).
   */
  async open(bytes: Uint8Array | ArrayBuffer, options: OpenOptions & CallOptions = {}): Promise<RemoteDocument> {
    const { signal, onProgress, transfer, ...open } = options;
    const ref = await this.call(0, "open", [bytes, open], pick({ signal, onProgress, transfer }));
    return this.#remoteDocument(ref as HandleRef);
  }

  /** Opens a remote file by HTTP range requests (L2c): the worker fetches what each call needs. */
  async openUrl(url: string, options: OpenOptions & RemoteHttpOptions & CallOptions = {}): Promise<RemoteDocument> {
    const { signal, onProgress, transfer: _, ...open } = options;
    const ref = await this.call(0, "openUrl", [url, open], pick({ signal, onProgress }));
    return this.#remoteDocument(ref as HandleRef);
  }

  /** Loads the text module in the worker (`loadText()`). */
  async loadText(options?: CallOptions): Promise<void> {
    await this.call(0, "loadText", [], options);
  }

  /** Loads the extract module in the worker (`loadExtract()`), for `page.text()`. */
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

/** The defined members of `options`, so `exactOptionalPropertyTypes` holds. */
function pick(options: {
  signal?: AbortSignal | undefined;
  onProgress?: ((p: Progress) => void) | undefined;
  transfer?: boolean | undefined;
}): CallOptions {
  return Object.fromEntries(Object.entries(options).filter(([, v]) => v !== undefined)) as CallOptions;
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
