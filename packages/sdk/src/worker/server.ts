// The worker side of `@laminapdf/sdk/worker`: `serve()` answers the
// handshake, calls `init()`, keeps the documents and pages it made in a handle table, and runs one
// call at a time (the WASM is synchronous). `dist/worker/entry.js` is `serve()` alone; a package
// that needs more in the same worker (`@laminapdf/viewer`: render methods) passes a ServerExtension
// and gets the same protocol, queue, cancellation and errors, so there is one RPC, not two.
//
// The queue runs the highest `priority` first (FIFO within one), and coalesces calls with the same
// `key`: a call whose key matches a queued or running one joins it, runs once, and each caller gets
// its own reply (an extension's `reply` builds one per caller, so transferables aren't shared). A
// queued call can be cancelled before it starts; a running one sees its AbortSignal trip once all
// its callers cancelled: `save` and `saveWithReport` run as stepped jobs that yield between steps
// (ADR 0028), and a range document's fetches take the signal. A cancel that arrives after the
// answer frees the document the call opened, since the page drops that answer. A WebAssembly trap
// is reported as a crash and ends the worker.

import { defined } from "../defined.js";
import { Document, PageRef } from "../document.js";
import { type Diagnostic, isDiagnostic, LaminaPdfError, packageError } from "../errors.js";
import type { ErrorCode } from "../generated/codes.js";
import { init, loadExtract, loadText } from "../index.js";
import { isTrap } from "../internal/instance.js";
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
import { whole } from "./transfer.js";

/** What an extension method sees of its call. */
export interface CallContext {
  /** Trips when every caller of this (possibly coalesced) call has cancelled. */
  readonly signal: AbortSignal;
  /** Sends a progress event to each caller (`onProgress`). */
  progress(progress: unknown): void;
}

/** A method an extension adds (called on the client itself, handle 0). */
export interface ExtensionMethod {
  /** Runs the method in the worker: its result, or a promise of it; what it throws is the caller's error. */
  run(params: readonly unknown[], context: CallContext): unknown;
  /**
   * One caller's answer from the result, and what to transfer with it. Called once per caller of
   * a coalesced call, so each gets its own transferables (an ImageBitmap each). Omitted, the
   * result is posted as is, its buffers transferred to the last caller and copied to the others.
   */
  reply?(result: unknown): [unknown, Transferable[]] | Promise<[unknown, Transferable[]]>;
}

/** What a package adds to the worker: methods on the client (handle 0). */
export interface ServerExtension {
  readonly methods: Readonly<Record<string, ExtensionMethod>>;
  /**
   * Work units per save step (default 20,000: a cancel waits at most a few milliseconds). A
   * smaller budget answers a cancel sooner and reports progress more often, at some cost in speed.
   */
  readonly saveBudget?: number;
}

/** What the worker holds and has done: for diagnostics, such as checking that handles are freed. */
export interface WorkerStats {
  /** Calls that started running (a call cancelled while queued never starts). */
  readonly started: number;
  /** Documents and pages the page side holds a handle to. */
  readonly handles: number;
  /** Calls waiting their turn. */
  readonly queued: number;
}

/** Work units per save step: small enough that a cancel waits at most a few milliseconds. */
const SAVE_BUDGET = 20_000;
/** The budget in use: `serve()`'s `saveBudget`, or the default. */
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
/**
 * Each open document's handle by the ID of the call that made it (`open`, `openUrl`). A cancel
 * for a call already answered means the page dropped that answer, so nobody holds the document.
 */
const madeBy = new Map<number, number>();

/** One run of a call, and every caller waiting for it (more than one when coalesced). */
interface Job {
  readonly first: Request;
  readonly waiters: Request[];
  priority: number;
  readonly controller: AbortController;
}

/** Jobs waiting their turn, highest priority first (FIFO within one); the running one. */
const queue: Job[] = [];
let current: Job | undefined;
let running = false;
/** Jobs actually started (a cancelled call never starts). */
let started = 0;
let ready = false;
let extension: ServerExtension = { methods: {} };

function post(message: FromWorker, transfer: Transferable[] = []): void {
  scope.postMessage(message, transfer);
}

/** Posts an extension event (`client.on("event", …)` on the page side). */
export function emit(name: string, data?: unknown, transfer: Transferable[] = []): void {
  post({ kind: "event", name, data }, transfer);
}

/**
 * The ADR 0026 object for anything thrown. Lamina PDF's own errors pass through, and so does the §1
 * object the WASM modules throw. Any other error that can describe itself (`toDiagnostic()`,
 * as `@laminapdf/render`'s RenderError does) is posted as that object, so a render
 * `data-pending` keeps its `[offset, length]` ranges in `data.ranges` (`LaminaPdfError.ranges`).
 * Anything else is a failure of the worker itself (`worker-failed`, kind internal). Caller
 * mistakes the worker detects (an unknown method) are thrown as `invalid-argument` where they
 * are found.
 */
function diagnostic(e: unknown): Diagnostic<ErrorCode> {
  if (e instanceof LaminaPdfError) return e.toJSON();
  if (isDiagnostic(e)) return e;
  const described = fromToDiagnostic(e);
  if (described !== undefined) return described;
  return packageError("worker-failed", e instanceof Error ? e.message : String(e)).toJSON();
}

/** What `e.toDiagnostic()` returns, when `e` has that method and it returns a §1 object. */
function fromToDiagnostic(e: unknown): Diagnostic<ErrorCode> | undefined {
  const toDiagnostic = (e as { toDiagnostic?: unknown } | null | undefined)?.toDiagnostic;
  if (typeof toDiagnostic !== "function") return undefined;
  const value: unknown = toDiagnostic.call(e);
  return isDiagnostic(value) ? value : undefined;
}

/** One channel for every yield, and the yields waiting on it (in order). */
let yieldChannel: MessageChannel | undefined;
const yielding: (() => void)[] = [];

/**
 * Lets queued messages (calls, a `cancel`) in, without the 4 ms clamp of nested `setTimeout`s.
 * Exported for extensions whose methods must notice a cancel before committing a change.
 */
export function yieldToMessages(): Promise<void> {
  if (yieldChannel === undefined) {
    yieldChannel = new MessageChannel();
    yieldChannel.port1.onmessage = () => yielding.shift()?.();
  }
  const channel = yieldChannel;
  return new Promise((resolve) => {
    yielding.push(resolve);
    channel.port2.postMessage(null);
  });
}

/** The client methods the worker itself serves; an extension can't reuse their names. */
const BUILT_IN = new Set(["open", "openUrl", "loadText", "loadExtract"]);

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
    for (const [id, h] of madeBy) if (h === handle) madeBy.delete(id);
    entry.value.free();
  }
}

async function stepSave(context: CallContext, doc: Document, method: string, options: SaveOptions | undefined) {
  const job = doc.saveJob(options);
  // The first progress event goes out after the first step, so a caller hears at once that a
  // stepped save is under way; later ones at most every PROGRESS_EVERY ms. (Counting from 0 would
  // wait for the worker's own clock, which starts when the worker does.)
  let last = Number.NEGATIVE_INFINITY;
  try {
    for (;;) {
      if (context.signal.aborted) job.cancel();
      const step = job.step(saveBudget);
      if (step.state === "done") {
        return method === "save" ? step.bytes : { bytes: step.bytes, report: step.report };
      }
      const now = performance.now();
      if (now - last >= PROGRESS_EVERY) {
        last = now;
        const { state: _, ...progress } = step;
        context.progress(progress);
      }
      await yieldToMessages();
    }
  } finally {
    job.free();
  }
}

/** `openRange`'s own options; the rest of `openUrl`'s go to `httpRangeSource`. */
const RANGE_KEYS = ["readahead", "maxRounds", "budget"] as const;

async function clientCall(method: string, params: readonly unknown[], context: CallContext): Promise<unknown> {
  const { signal } = context;
  switch (method) {
    case "open": {
      const [bytes, options] = params as [Uint8Array, OpenOptions | undefined];
      return register({ type: "document", value: Document.open(bytes, options), range: false });
    }
    case "openUrl": {
      const [url, options] = params as [string, Record<string, unknown> | undefined];
      const { password, strict, limits, ...http } = options ?? {};
      const open: Record<string, unknown> = {
        signal,
        ...defined({ password, strict, limits, blockSize: http.blockSize }),
      };
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
    default: {
      const own = Object.hasOwn(extension.methods, method) ? extension.methods[method] : undefined;
      if (own === undefined) throw packageError("invalid-argument", `the worker client has no method ${method}`);
      return own.run(params, context);
    }
  }
}

async function documentCall(
  context: CallContext,
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
      return stepSave(context, doc, method, params[0] as SaveOptions | undefined);
    }
    const member = (doc as unknown as Record<string, unknown>)[method];
    const value =
      typeof member === "function" ? (member as (...a: unknown[]) => unknown).apply(doc, [...params]) : member;
    if (value instanceof PageRef) return pageRef(value, handle);
    return value;
  };
  // A range document fetches what a call needs, then calls again.
  return entry.range ? doc.ensure(run, { signal: context.signal }) : run();
}

function pageCall(
  context: CallContext,
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
  return doc?.type === "document" && doc.range ? doc.value.ensure(run, { signal: context.signal }) : run();
}

/** The extension method `request` calls, if it is one (it may build a reply per caller). */
function extensionMethod(request: Request): ExtensionMethod | undefined {
  return request.handle === 0 && !BUILT_IN.has(request.method) && Object.hasOwn(extension.methods, request.method)
    ? extension.methods[request.method]
    : undefined;
}

async function execute(job: Job): Promise<void> {
  const { signal } = job.controller;
  const context: CallContext = {
    signal,
    progress: (progress) => {
      for (const w of job.waiters) post({ kind: "progress", id: w.id, progress });
    },
  };
  current = job;
  started++;
  try {
    const result = await run(job.first, context);
    if (signal.aborted) {
      // Nobody will hold a handle the cancelled call made (`openUrl` aborted mid-fetch): free it.
      if (isHandleRef(result)) release(result.$handle);
      throw packageError("cancelled", "the call was cancelled");
    }
    // The result is in: no caller joins this job any more (a reply can take a while, and a caller
    // that joined now would get none); a same-key call from here on runs again.
    current = undefined;
    // The caller's cancel may still arrive after this answer: cancel() then frees the document.
    const opened = openedFor(job, result);
    if (opened !== undefined) madeBy.set(...opened);
    await replyAll(job, result);
  } catch (e) {
    if (isTrap(e)) {
      // A trap: the module's state can't be trusted any more. The sync API reports Lamina PDF's
      // own traps as `wasm-trap` or `out-of-memory`; an extension's module (PDFium) throws the
      // RuntimeError itself.
      post({ kind: "crashed", reason: "trap" });
      scope.close();
      return;
    }
    // The page dropped an aborted call's promise already; the answer is only for the record.
    const error = signal.aborted ? packageError("cancelled", "the call was cancelled").toJSON() : diagnostic(e);
    for (const w of job.waiters) post({ kind: "result", id: w.id, ok: false, error });
  } finally {
    current = undefined;
  }
}

/**
 * `[call ID, document handle]` when `job` is an `open` or `openUrl` that answered one caller
 * alone (one coalesced by a `key` shares its document, so no single cancel may free it).
 */
function openedFor(job: Job, result: unknown): [number, number] | undefined {
  const { handle, method } = job.first;
  if (handle !== 0 || (method !== "open" && method !== "openUrl") || !isHandleRef(result)) return undefined;
  const [only, ...others] = job.waiters;
  return only !== undefined && others.length === 0 ? [only.id, result.$handle] : undefined;
}

/** Runs the call: a client method, or a method of the document or page its handle names. */
async function run(request: Request, context: CallContext): Promise<unknown> {
  const { handle, method, params } = request;
  if (handle === 0) return clientCall(method, params, context);
  const entry = handles.get(handle);
  if (entry === undefined) throw packageError("disposed", "the handle was closed or belongs to another worker");
  return entry.type === "document"
    ? documentCall(context, handle, entry, method, params)
    : pageCall(context, entry, method, params);
}

/**
 * Answers each caller of the job on its own: a reply that fails (createImageBitmap out of
 * memory) fails only that caller, and those already answered aren't answered twice.
 */
async function replyAll(job: Job, result: unknown): Promise<void> {
  const own = extensionMethod(job.first);
  const waiters = [...job.waiters];
  for (const [i, w] of waiters.entries()) {
    try {
      const [value, transfer] = await answer(own, result, i === waiters.length - 1);
      post({ kind: "result", id: w.id, ok: true, result: value }, transfer);
    } catch (e) {
      post({ kind: "result", id: w.id, ok: false, error: diagnostic(e) });
    }
  }
}

/** One caller's answer and what it transfers: the extension's reply, else the result itself. */
async function answer(
  own: ExtensionMethod | undefined,
  result: unknown,
  last: boolean,
): Promise<readonly [unknown, Transferable[]]> {
  if (own?.reply !== undefined) return own.reply(result);
  const [value, transfer] = outgoing(result);
  // One set of buffers: transferred to the last caller, copied to the others.
  return [value, last ? transfer : []];
}

async function drain(): Promise<void> {
  if (running) return;
  running = true;
  try {
    for (;;) {
      // Let every message already sent in first, so the queue sees all waiting calls and
      // cancels before it picks: a synchronous call otherwise finishes inside one message's
      // handler, before the next is even read, and nothing is ever queued to order or drop.
      await yieldToMessages();
      const job = queue.shift();
      if (job === undefined) break;
      await execute(job);
    }
  } finally {
    running = false;
  }
}

/** Puts `job` after every job of its priority or higher. */
function enqueue(job: Job): void {
  const at = queue.findIndex((j) => j.priority < job.priority);
  if (at === -1) queue.push(job);
  else queue.splice(at, 0, job);
}

function call(request: Request): void {
  const { key } = request;
  const priority = request.priority ?? 0;
  if (key !== undefined) {
    // Same key, method and handle: a key from one method never answers another's call.
    const same = (j: Job) =>
      j.first.key === key && j.first.method === request.method && j.first.handle === request.handle;
    if (current !== undefined && same(current) && !current.controller.signal.aborted) {
      current.waiters.push(request);
      return;
    }
    const queued = queue.find(same);
    if (queued !== undefined) {
      queued.waiters.push(request);
      if (priority > queued.priority) {
        // The more urgent caller moves the shared job up.
        queue.splice(queue.indexOf(queued), 1);
        queued.priority = priority;
        enqueue(queued);
      }
      return;
    }
  }
  enqueue({ first: request, waiters: [request], priority, controller: new AbortController() });
  if (ready) void drain();
}

function cancel(id: number): void {
  // Only a call still queued or running can be cancelled. A cancel that crossed its answer (the
  // page aborted after the worker answered, before it read the answer) finds no job: the page
  // drops that answer, so a document the call made is freed here, and nothing else is left.
  const job = current?.waiters.some((w) => w.id === id)
    ? current
    : queue.find((j) => j.waiters.some((w) => w.id === id));
  if (job === undefined) {
    const made = madeBy.get(id);
    if (made !== undefined) release(made);
    return;
  }
  job.waiters.splice(
    job.waiters.findIndex((w) => w.id === id),
    1,
  );
  if (job.waiters.length > 0) {
    // The job keeps the priority of its most urgent remaining caller.
    const priority = Math.max(...job.waiters.map((w) => w.priority ?? 0));
    if (job !== current && priority !== job.priority) {
      queue.splice(queue.indexOf(job), 1);
      job.priority = priority;
      enqueue(job);
    }
    return;
  }
  if (job === current) {
    job.controller.abort();
    return;
  }
  queue.splice(queue.indexOf(job), 1);
  post({
    kind: "result",
    id,
    ok: false,
    error: packageError("cancelled", "the call was cancelled before it started").toJSON(),
  });
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

let serving = false;

/** What this worker holds and has done (see {@link WorkerStats}). */
export function workerStats(): WorkerStats {
  return { started, handles: handles.size, queued: queue.length };
}

/**
 * Serves the worker client's protocol in this worker, with `ext`'s methods added. Call it once,
 * at the top of the worker's module.
 */
export function serve(ext?: ServerExtension): void {
  if (serving) throw packageError("invalid-argument", "serve() was already called in this worker");
  if (ext !== undefined) {
    const clash = Object.keys(ext.methods).find((m) => BUILT_IN.has(m));
    if (clash !== undefined)
      throw packageError("invalid-argument", `an extension can't redefine the built-in method ${clash}`);
    extension = ext;
    if (ext.saveBudget !== undefined) {
      if (!(Number.isInteger(ext.saveBudget) && ext.saveBudget > 0))
        throw packageError("invalid-argument", "saveBudget must be a positive whole number of work units");
      saveBudget = ext.saveBudget;
    }
  }
  serving = true;
  scope.addEventListener("message", (event) => {
    const message = event.data;
    switch (message.kind) {
      case "hello":
        void hello(message);
        return;
      case "cancel":
        cancel((message as Cancel).id);
        return;
      case "call":
        // Calls before the handshake finished wait for it (hello() drains them).
        call(message);
        return;
    }
  });
}
