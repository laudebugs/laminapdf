// The worker side of `@lamina/pdf/worker` (L2a): `dist/worker/entry.js`, started by `connect()`.
// It answers the handshake, calls `init()`, keeps the documents and pages it made in a handle
// table, and runs one call at a time (the WASM is synchronous). A queued call can be cancelled
// before it starts; `save` and `saveWithReport` run as stepped save jobs that yield to the event
// loop between steps, so a cancel arrives and trips the job (ADR 0028). Results that are buffers
// are transferred. A WebAssembly trap is reported as a crash and ends the worker.
import { Document, PageRef } from "../document.js";
import { isDiagnostic, LaminaError, packageError } from "../errors.js";
import type { ErrorCode } from "../generated/codes.js";
import { init, loadExtract, loadText } from "../index.js";
import { httpRangeSource } from "../source/http.js";
import { PageText, rawOf } from "../text.js";
import type { OpenOptions, SaveOptions } from "../types.js";
import {
  type Cancel,
  type FromWorker,
  HANDSHAKE,
  type HandleRef,
  type Hello,
  type Request,
  type ToWorker,
} from "./protocol.js";
import { DOCUMENT_METHODS, PAGE_METHODS } from "./support.js";

/** Work units per save step: small enough that a cancel waits at most a few milliseconds. */
const SAVE_BUDGET = 20_000;
/** The budget in use (`__budget`, test-only, shrinks it so a small save takes many steps). */
let saveBudget = SAVE_BUDGET;
/** Least time between two progress events of one call, in ms. */
const PROGRESS_EVERY = 50;

interface Scope {
  postMessage(message: FromWorker, transfer?: Transferable[]): void;
  addEventListener(type: "message", listener: (event: MessageEvent<ToWorker>) => void): void;
  close(): void;
}
const scope = globalThis as unknown as Scope;

type Entry =
  | { readonly type: "document"; readonly value: Document; readonly range: boolean }
  | { readonly type: "page"; readonly value: PageRef; readonly document: number };

const handles = new Map<number, Entry>();
let nextHandle = 1;
/** Calls waiting their turn, by ID, and the cancels that arrived for them. */
const queue: Request[] = [];
const cancelled = new Set<number>();
let running = false;
/** Calls actually started (the tests check that a cancelled call never starts). */
let started = 0;
let ready = false;

function post(message: FromWorker, transfer: Transferable[] = []): void {
  scope.postMessage(message, transfer);
}

/** The ADR 0026 object for anything thrown. */
function diagnostic(e: unknown) {
  if (e instanceof LaminaError) return e.toJSON();
  if (isDiagnostic(e)) return e;
  const message = e instanceof Error ? e.message : String(e);
  return {
    code: "invalid-argument" as ErrorCode,
    kind: "usage" as const,
    message,
    docsUrl: "https://github.com/laudebugs/lamina/blob/main/docs/errors/invalid-argument.md",
  };
}

function register(entry: Entry, fields?: Record<string, unknown>): HandleRef {
  const handle = nextHandle++;
  handles.set(handle, entry);
  return { $handle: handle, type: entry.type, ...(fields === undefined ? {} : { fields }) };
}

function pageRef(page: PageRef, document: number): HandleRef {
  return register({ type: "page", value: page, document }, { id: page.id, index: page.index });
}

/** Values to transfer out of `result`, and the result as it should be posted. */
function outgoing(result: unknown): [unknown, Transferable[]] {
  if (result instanceof Uint8Array) return [result, [result.buffer]];
  if (result instanceof PageText) {
    const raw = rawOf(result);
    return [{ $pageText: raw }, [raw.quads.buffer, raw.glyphOffsets.buffer]];
  }
  if (typeof result === "object" && result !== null && "bytes" in result && result.bytes instanceof Uint8Array) {
    return [result, [result.bytes.buffer]];
  }
  return [result, []];
}

async function stepSave(id: number, doc: Document, method: string, options: SaveOptions | undefined) {
  const job = doc.saveJob(options);
  let last = 0;
  try {
    for (;;) {
      if (cancelled.has(id)) job.cancel();
      const step = job.step(saveBudget);
      if (step.state === "done") {
        return method === "save" ? step.bytes : { bytes: step.bytes, report: step.report };
      }
      const now = performance.now();
      if (now - last >= PROGRESS_EVERY) {
        last = now;
        const { state: _, ...progress } = step;
        post({ kind: "progress", id, progress });
      }
      // Let a `cancel` message in.
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  } finally {
    job.free();
  }
}

async function clientCall(method: string, params: readonly unknown[]): Promise<unknown> {
  switch (method) {
    case "open": {
      const [bytes, options] = params as [Uint8Array, OpenOptions | undefined];
      return register({ type: "document", value: Document.open(bytes, options), range: false });
    }
    case "openUrl": {
      const [url, options] = params as [string, (OpenOptions & Record<string, unknown>) | undefined];
      const { password, strict, limits, ...http } = options ?? {};
      const open = {
        ...(password === undefined ? {} : { password }),
        ...(strict === undefined ? {} : { strict }),
        ...(limits === undefined ? {} : { limits }),
      };
      const doc = await Document.openRange(httpRangeSource(url, http), open);
      return register({ type: "document", value: doc, range: true });
    }
    case "loadText":
      return loadText();
    case "loadExtract":
      return loadExtract();
    case "__budget":
      saveBudget = typeof params[0] === "number" && params[0] > 0 ? params[0] : SAVE_BUDGET;
      return saveBudget;
    case "__stats":
      return { started, handles: handles.size };
    case "__crash":
      // An uncaught error: the page sees the worker's `error` event.
      setTimeout(() => {
        throw new Error("crash requested by a test");
      });
      return new Promise(() => {});
    default:
      throw new TypeError(`unknown client method ${method}`);
  }
}

async function documentCall(
  id: number,
  handle: number,
  entry: Extract<Entry, { type: "document" }>,
  method: string,
  params: readonly unknown[],
) {
  const doc = entry.value;
  if (method === "close") {
    for (const [h, e] of handles) if (e.type === "page" && e.document === handle) handles.delete(h);
    handles.delete(handle);
    doc.free();
    return undefined;
  }
  if (!Object.hasOwn(DOCUMENT_METHODS, method)) throw new TypeError(`Document has no remote method ${method}`);
  const run = async (): Promise<unknown> => {
    if (method === "save" || method === "saveWithReport") {
      return stepSave(id, doc, method, params[0] as SaveOptions | undefined);
    }
    const member = (doc as unknown as Record<string, unknown>)[method];
    const value =
      typeof member === "function" ? (member as (...a: unknown[]) => unknown).apply(doc, [...params]) : member;
    if (value instanceof PageRef) return pageRef(value, handle);
    return value;
  };
  // A range document fetches what a call needs, then calls again (L2c).
  return entry.range ? doc.ensure(run) : run();
}

function pageCall(entry: Extract<Entry, { type: "page" }>, method: string, params: readonly unknown[]) {
  if (!Object.hasOwn(PAGE_METHODS, method)) throw new TypeError(`PageRef has no remote method ${method}`);
  const member = (entry.value as unknown as Record<string, unknown>)[method];
  const run = () =>
    typeof member === "function" ? (member as (...a: unknown[]) => unknown).apply(entry.value, [...params]) : member;
  const doc = handles.get(entry.document);
  return doc?.type === "document" && doc.range ? doc.value.ensure(run) : run();
}

async function execute(request: Request): Promise<void> {
  const { id, handle, method, params } = request;
  started++;
  try {
    let result: unknown;
    if (handle === 0) result = await clientCall(method, params);
    else {
      const entry = handles.get(handle);
      if (entry === undefined) throw packageError("disposed", "the handle was closed or belongs to another worker");
      result =
        entry.type === "document"
          ? await documentCall(id, handle, entry, method, params)
          : await pageCall(entry, method, params);
    }
    if (cancelled.has(id)) throw packageError("cancelled", "the call was cancelled");
    const [value, transfer] = outgoing(result);
    post({ kind: "result", id, ok: true, result: value }, transfer);
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) {
      // A trap: the module's state can't be trusted any more.
      post({ kind: "crashed", reason: "trap" });
      scope.close();
      return;
    }
    post({ kind: "result", id, ok: false, error: diagnostic(e) });
  } finally {
    cancelled.delete(id);
  }
}

async function drain(): Promise<void> {
  if (running) return;
  running = true;
  try {
    for (let request = queue.shift(); request !== undefined; request = queue.shift()) {
      if (cancelled.has(request.id)) {
        cancelled.delete(request.id);
        post({
          kind: "result",
          id: request.id,
          ok: false,
          error: packageError("cancelled", "the call was cancelled before it started").toJSON(),
        });
        continue;
      }
      await execute(request);
    }
  } finally {
    running = false;
  }
}

async function hello(message: Hello): Promise<void> {
  const { protocol, version } = message.client;
  if (protocol !== HANDSHAKE.protocol || version !== HANDSHAKE.version) {
    // Answer with what this worker is, and do nothing else: the client rejects.
    post({ kind: "welcome", worker: HANDSHAKE });
    return;
  }
  try {
    await init(message.init as Parameters<typeof init>[0]);
    ready = true;
    post({ kind: "welcome", worker: HANDSHAKE });
  } catch (e) {
    post({ kind: "welcome", worker: HANDSHAKE, error: diagnostic(e) });
  }
}

scope.addEventListener("message", (event) => {
  const message = event.data;
  switch (message.kind) {
    case "hello":
      void hello(message);
      return;
    case "cancel":
      cancelled.add((message as Cancel).id);
      return;
    case "call":
      if (!ready) return;
      queue.push(message);
      void drain();
      return;
  }
});
