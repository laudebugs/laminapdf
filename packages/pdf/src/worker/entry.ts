// The worker side of `@lamina/pdf/worker` (L2a): `dist/worker/entry.js`, started by `connect()`.
// It answers the handshake, calls `init()`, keeps the documents and pages it made in a handle
// table, and runs one call at a time (the WASM is synchronous). A queued call can be cancelled
// before it starts; `save` and `saveWithReport` run as stepped save jobs that yield to the event
// loop between steps, so a cancel arrives and trips the job (ADR 0028); a range document's
// fetches take the call's AbortSignal. Results that are buffers are transferred. A WebAssembly
// trap is reported as a crash and ends the worker.
import { Document, PageRef } from "../document.js";
import { isDiagnostic, LaminaError, packageError } from "../errors.js";
import { init, loadExtract, loadText } from "../index.js";
import { httpRangeSource } from "../source/http.js";
import type { RangeOptions } from "../source/range.js";
import { PageText, rawOf } from "../text.js";
import type { OpenOptions, SaveOptions } from "../types.js";
import {
  type Cancel,
  type FromWorker,
  HANDSHAKE,
  type HandleRef,
  type Hello,
  isHandleRef,
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
/** Each document's page handles by page ID: `page(i)` again returns the same handle. */
const pageHandles = new Map<number, Map<string, number>>();
let nextHandle = 1;
/** Calls waiting their turn, and the queued calls cancelled before they start. */
const queue: Request[] = [];
const cancelled = new Set<number>();
/** The running call's ID and the controller its `cancel` aborts. */
let current: { readonly id: number; readonly controller: AbortController } | undefined;
let running = false;
/** Calls actually started (the tests check that a cancelled call never starts). */
let started = 0;
let ready = false;

function post(message: FromWorker, transfer: Transferable[] = []): void {
  scope.postMessage(message, transfer);
}

/**
 * The ADR 0026 object for anything thrown. lamina's own errors pass through; anything else is a
 * failure of the worker client itself (`worker-failed`, kind internal), with the original
 * message. Caller mistakes the worker detects (an unknown method) are thrown as
 * `invalid-argument` where they are found.
 */
function diagnostic(e: unknown) {
  if (e instanceof LaminaError) return e.toJSON();
  if (isDiagnostic(e)) return e;
  return packageError("worker-failed", e instanceof Error ? e.message : String(e)).toJSON();
}

/** The buffer of `view` if transferring it moves exactly `view` (not a larger or shared buffer). */
function whole(view: ArrayBufferView): ArrayBuffer[] {
  const { buffer } = view;
  return buffer instanceof ArrayBuffer && view.byteOffset === 0 && view.byteLength === buffer.byteLength
    ? [buffer]
    : [];
}

/** Lets queued messages (a `cancel`) in, without the 4 ms clamp of nested `setTimeout`s. */
function yieldToMessages(): Promise<void> {
  return new Promise((resolve) => {
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      channel.port1.close();
      resolve();
    };
    channel.port2.postMessage(null);
  });
}

function register(entry: Entry, fields?: Record<string, unknown>): HandleRef {
  const handle = nextHandle++;
  handles.set(handle, entry);
  return { $handle: handle, type: entry.type, ...(fields === undefined ? {} : { fields }) };
}

function pageRef(page: PageRef, document: number): HandleRef {
  const fields = { id: page.id, index: page.index };
  let pages = pageHandles.get(document);
  if (pages === undefined) {
    pages = new Map();
    pageHandles.set(document, pages);
  }
  const known = pages.get(page.id);
  if (known !== undefined && handles.has(known)) return { $handle: known, type: "page", fields };
  const ref = register({ type: "page", value: page, document }, fields);
  pages.set(page.id, ref.$handle);
  return ref;
}

/** Values to transfer out of `result`, and the result as it should be posted. */
function outgoing(result: unknown): [unknown, Transferable[]] {
  if (result instanceof Uint8Array) return [result, whole(result)];
  if (result instanceof PageText) {
    const raw = rawOf(result);
    return [{ $pageText: raw }, [...whole(raw.quads), ...whole(raw.glyphOffsets)]];
  }
  if (typeof result === "object" && result !== null && "bytes" in result && result.bytes instanceof Uint8Array) {
    return [result, whole(result.bytes)];
  }
  return [result, []];
}

/** Frees a document handle (and its pages), or a page handle. */
function release(handle: number): void {
  const entry = handles.get(handle);
  if (entry === undefined) return;
  handles.delete(handle);
  if (entry.type === "document") {
    for (const h of pageHandles.get(handle)?.values() ?? []) handles.delete(h);
    pageHandles.delete(handle);
    entry.value.free();
  }
}

async function stepSave(
  id: number,
  signal: AbortSignal,
  doc: Document,
  method: string,
  options: SaveOptions | undefined,
) {
  const job = doc.saveJob(options);
  let last = 0;
  try {
    for (;;) {
      if (signal.aborted) job.cancel();
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
      await yieldToMessages();
    }
  } finally {
    job.free();
  }
}

/** `openRange`'s own options; the rest of `openUrl`'s go to `httpRangeSource`. */
const RANGE_KEYS = ["readahead", "maxRounds", "budget"] as const;

async function clientCall(method: string, params: readonly unknown[], signal: AbortSignal): Promise<unknown> {
  switch (method) {
    case "open": {
      const [bytes, options] = params as [Uint8Array, OpenOptions | undefined];
      return register({ type: "document", value: Document.open(bytes, options), range: false });
    }
    case "openUrl": {
      const [url, options] = params as [string, Record<string, unknown> | undefined];
      const { password, strict, limits, ...http } = options ?? {};
      const open: Record<string, unknown> = { signal };
      for (const [k, v] of Object.entries({ password, strict, limits, blockSize: http.blockSize })) {
        if (v !== undefined) open[k] = v;
      }
      for (const key of RANGE_KEYS) {
        if (http[key] !== undefined) open[key] = http[key];
        delete http[key];
      }
      const doc = await Document.openRange(
        httpRangeSource(url, { ...http, signal }),
        open as OpenOptions & RangeOptions,
      );
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
      return { started, handles: handles.size, cancels: cancelled.size };
    case "__crash":
      // An uncaught error: the page sees the worker's `error` event.
      setTimeout(() => {
        throw new Error("crash requested by a test");
      });
      return new Promise(() => {});
    default:
      throw packageError("invalid-argument", `the worker client has no method ${method}`);
  }
}

async function documentCall(
  id: number,
  signal: AbortSignal,
  handle: number,
  entry: Extract<Entry, { type: "document" }>,
  method: string,
  params: readonly unknown[],
) {
  const doc = entry.value;
  if (method === "close") {
    release(handle);
    return undefined;
  }
  if (!Object.hasOwn(DOCUMENT_METHODS, method)) {
    throw packageError("invalid-argument", `Document has no remote method ${method}`);
  }
  const run = async (): Promise<unknown> => {
    if (method === "save" || method === "saveWithReport") {
      return stepSave(id, signal, doc, method, params[0] as SaveOptions | undefined);
    }
    const member = (doc as unknown as Record<string, unknown>)[method];
    const value =
      typeof member === "function" ? (member as (...a: unknown[]) => unknown).apply(doc, [...params]) : member;
    if (value instanceof PageRef) return pageRef(value, handle);
    return value;
  };
  // A range document fetches what a call needs, then calls again (L2c).
  return entry.range ? doc.ensure(run, { signal }) : run();
}

function pageCall(
  signal: AbortSignal,
  entry: Extract<Entry, { type: "page" }>,
  method: string,
  params: readonly unknown[],
) {
  if (!Object.hasOwn(PAGE_METHODS, method)) {
    throw packageError("invalid-argument", `PageRef has no remote method ${method}`);
  }
  const member = (entry.value as unknown as Record<string, unknown>)[method];
  const run = () =>
    typeof member === "function" ? (member as (...a: unknown[]) => unknown).apply(entry.value, [...params]) : member;
  const doc = handles.get(entry.document);
  return doc?.type === "document" && doc.range ? doc.value.ensure(run, { signal }) : run();
}

async function execute(request: Request): Promise<void> {
  const { id, handle, method, params } = request;
  const controller = new AbortController();
  const { signal } = controller;
  current = { id, controller };
  started++;
  try {
    let result: unknown;
    if (handle === 0) result = await clientCall(method, params, signal);
    else {
      const entry = handles.get(handle);
      if (entry === undefined) throw packageError("disposed", "the handle was closed or belongs to another worker");
      result =
        entry.type === "document"
          ? await documentCall(id, signal, handle, entry, method, params)
          : await pageCall(signal, entry, method, params);
    }
    if (signal.aborted) {
      // Nobody will hold a handle the cancelled call made (`openUrl` aborted mid-fetch): free it.
      if (isHandleRef(result)) release(result.$handle);
      throw packageError("cancelled", "the call was cancelled");
    }
    const [value, transfer] = outgoing(result);
    post({ kind: "result", id, ok: true, result: value }, transfer);
  } catch (e) {
    if (e instanceof WebAssembly.RuntimeError) {
      // A trap: the module's state can't be trusted any more.
      post({ kind: "crashed", reason: "trap" });
      scope.close();
      return;
    }
    // The page dropped an aborted call's promise already; the answer is only for the record.
    const error = signal.aborted ? packageError("cancelled", "the call was cancelled").toJSON() : diagnostic(e);
    post({ kind: "result", id, ok: false, error });
  } finally {
    current = undefined;
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
    void drain();
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
    case "cancel": {
      // Only a call still queued or running can be cancelled; a cancel that crossed its answer
      // is ignored (and leaves nothing behind).
      const { id } = message as Cancel;
      if (current?.id === id) current.controller.abort();
      else if (queue.some((r) => r.id === id)) cancelled.add(id);
      return;
    }
    case "call":
      // Calls before the handshake finished wait for it (hello() drains them).
      queue.push(message);
      if (ready) void drain();
      return;
  }
});
