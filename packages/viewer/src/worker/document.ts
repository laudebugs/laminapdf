// The one document a viewer worker holds (ADR 0009 §2: one worker per document): Lamina PDF's
// Document for the model (page sizes, warnings, later text and structure) and a RenderDocument
// for pixels, both opened from the same bytes. The password is used here and kept nowhere else.
// A remote file is read through one HttpRangeByteSource: Lamina PDF opens it range by range and
// the backend reads its block cache, each fetching only what it needs (a render that meets bytes
// not fetched yet fills them and draws again).

import {
  bytesSource,
  type CloneableWasmSource,
  isRenderError,
  type RenderBackend,
  type RenderDocument,
  RenderError,
} from "@laminapdf/render";
import { initHayro } from "@laminapdf/render-hayro";
import { initPdfium } from "@laminapdf/render-pdfium";
import {
  Document,
  isLaminaPdfError,
  loadExtract,
  type OpenOptions,
  type RangeSource,
  wasmMemory,
} from "@laminapdf/sdk";
import { emit, yieldToMessages } from "@laminapdf/sdk/worker/server";
import type {
  BackendInit,
  OpenedDocument,
  RemoteSource,
  SourceInit,
  ViewerOpenOptions,
  WorkerMemory,
} from "../protocol.js";
import { HttpRangeByteSource } from "../source/http-range.js";
import { COPY_ABOVE, OpfsByteSource } from "../source/opfs.js";
import { blobSource, readAll } from "../source.js";

let backend: RenderBackend | undefined;
/** Backends loaded or loading, by name; a failed load is forgotten, so the next call retries. */
const loading = new Map<string, Promise<RenderBackend>>();

interface Open {
  readonly id: number;
  readonly model: Document;
  readonly render: RenderDocument;
  /** Where a remote document's bytes come from; undefined when they are all in memory. */
  readonly remote: HttpRangeByteSource | undefined;
  /** A large local file's OPFS copy it is read from; undefined when it is in memory. */
  readonly local: OpfsByteSource | undefined;
  /**
   * How to open it again as a text-capable copy (its bytes in memory, the options it opened
   * with): kept only while that copy may be needed, since the options hold the password.
   */
  reopen: { readonly bytes: Uint8Array | undefined; readonly options: ViewerOpenOptions } | undefined;
  /** The copy opened in the extract module, for a document opened before that module loaded. */
  text?: Promise<Document> | undefined;
  /** What its open answered, and the page's number for that open (`currentDocument`). */
  answer?: { readonly document: OpenedDocument; readonly ticket: number | undefined };
}

/**
 * Rounds of fetching a call may need before it gives up with its last `data-pending`: the SDK's
 * default for the same loop over Lamina PDF's reads (`Document.openRange`'s `maxRounds`).
 */
const MAX_FILL_ROUNDS = 1024;
let open: Open | undefined;
let nextDocument = 1;

/** PDFium needs its `.wasm` from the app (hayro finds its own). */
function loadPdfium(wasm: CloneableWasmSource | undefined): Promise<RenderBackend> {
  if (wasm === undefined) {
    return Promise.reject(
      new RenderError("render-backend-unavailable", "PDFium needs its .wasm: pass BackendInit.wasm"),
    );
  }
  return initPdfium(wasm);
}

/** Loads the backend (the client asks as soon as the worker starts, before a file is chosen). */
export async function initBackend(init: BackendInit): Promise<{ name: string; version: string }> {
  let pending = loading.get(init.backend);
  if (pending === undefined) {
    pending = init.backend === "hayro" ? initHayro(init.wasm) : loadPdfium(init.wasm);
    loading.set(init.backend, pending);
    pending.catch(() => loading.delete(init.backend));
  }
  const loaded = await pending;
  backend = loaded;
  return { name: loaded.name, version: loaded.version };
}

/** Whether a render has started the extract module's load (once per worker). */
let warmed = false;

/**
 * Starts the text layer's module once a page has been drawn (render.ts calls it after each
 * render; only the first call starts anything). Before that its download would compete with the
 * document's bytes and the backend for the first page; text asked for earlier loads it at once.
 */
export function warmExtract(): void {
  if (warmed) return;
  warmed = true;
  void extractModule().catch(() => {
    // Tried once here; text asked for later tries again, never another render.
  });
}

/** The extract module's load, started after the backend's. */
let extract: Promise<void> | undefined;

/** The extract module's load: the one under way, or a new one after a failed one. */
function extractModule(): Promise<void> {
  if (extract === undefined) {
    const load = loadExtract();
    extract = load;
    // A failed load is forgotten (the next call tries again), while its callers still see it fail.
    load.catch(() => {
      if (extract === load) extract = undefined;
    });
  }
  return extract;
}

/**
 * The open document as the extract module reads it, for its text and structure: itself when it
 * opened in that module, else a copy opened from the same bytes with the same options once the
 * module has loaded (a document opened before it: a link opened as the reader starts).
 */
export async function textModel(signal: AbortSignal): Promise<Document> {
  const opened = current();
  if (opened.model.canExtractText) return opened.model;
  // The copy, opened or opening; else none can be opened (its bytes were let go), and the base
  // document answers `extract-module-not-loaded` itself.
  if (opened.text !== undefined) return untilAborted(opened.text, signal);
  if (opened.reopen === undefined) return opened.model;
  const { bytes, options } = opened.reopen;
  const ranged = opened.remote ?? opened.local;
  if (bytes === undefined && ranged === undefined) return opened.model;
  // One copy for every caller, opened with no caller's signal: a page that scrolls away aborts
  // its own wait, never the copy the other pages and the structure tree wait for.
  opened.text = (async () => {
    await extractModule();
    const { model } =
      bytes !== undefined
        ? await openModel((o) => Document.open(bytes, o), options)
        : await openModel((o) => Document.openRange(ranged as RangeSource, o), options);
    if (!model.canExtractText) {
      model.free();
      throw new RenderError("render-failed", "the extract module didn't load");
    }
    // The copy is open: the password and bytes it needed are no longer kept.
    opened.reopen = undefined;
    return model;
  })();
  opened.text.catch(() => {
    opened.text = undefined;
  });
  return untilAborted(opened.text, signal);
}

/**
 * What a text-capable copy of `model` would open from: nothing for a document that can already
 * give its text, since the options hold the password and are kept only while needed.
 */
function reopenFor(model: Document, bytes: Uint8Array | undefined, options: ViewerOpenOptions): Open["reopen"] {
  return model.canExtractText ? undefined : { bytes, options };
}

/** `promise`, or a rejection as soon as `signal` aborts, whichever comes first. */
function untilAborted<T>(promise: Promise<T>, signal: AbortSignal): Promise<T> {
  signal.throwIfAborted();
  return new Promise<T>((resolve, reject) => {
    const abort = () => reject(signal.reason);
    signal.addEventListener("abort", abort, { once: true });
    promise.then(resolve, reject).finally(() => signal.removeEventListener("abort", abort));
  });
}

function currentBackend(): RenderBackend {
  // Only a viewer bug gets here (the client loads the backend before anything else), so it stays
  // an Error: the worker reports it as worker-failed, kind internal.
  if (backend === undefined) throw new Error("initBackend was not called");
  return backend;
}

function closeOpen(): void {
  if (open === undefined) return;
  open.render.close();
  open.model.free();
  open.local?.close();
  // A text-capable copy goes with it, once opened (a failed one has nothing to free).
  open.text?.then(
    (copy) => copy.free(),
    () => {},
  );
  open = undefined;
}

function isRemote(init: SourceInit): init is RemoteSource {
  return typeof (init as Partial<RemoteSource>).url === "string" && !(init instanceof Blob);
}

/** A file's bytes, in memory: bytes as they are, a Blob read in slices. */
async function bytesOf(init: Exclude<SourceInit, RemoteSource>, signal: AbortSignal): Promise<Uint8Array> {
  if (init instanceof Blob) return readAll(blobSource(init), signal);
  return init instanceof Uint8Array ? init : new Uint8Array(init);
}

/**
 * Runs `f` until it stops throwing `data-pending`, filling the ranges it names from `remote`
 * between tries (`onFetch` runs before each fill). A round that asks for exactly what the last
 * one asked made no progress. The render backend's twin of the SDK's loop over Lamina PDF's
 * reads (`Document.ensure`), which catches the SDK's own error type and fills its own source.
 *
 * PDFium is opened over a source that refuses missing reads rather than through its
 * availability API: on a well-formed file its open asks for the header, the xref and the
 * trailer, a few blocks; a damaged file it rebuilds asks for all of it, as repairing needs.
 */
export async function filling<T>(
  remote: HttpRangeByteSource | undefined,
  f: () => T,
  signal: AbortSignal,
  onFetch?: () => void,
): Promise<T> {
  let previous: string | undefined;
  for (let round = 0; ; round++) {
    try {
      return f();
    } catch (e) {
      if (remote === undefined || !isRenderError(e) || e.code !== "data-pending") throw e;
      const ranges = e.ranges ?? [];
      const key = JSON.stringify(ranges);
      if (round >= MAX_FILL_ROUNDS || ranges.length === 0 || key === previous) throw e;
      previous = key;
      onFetch?.();
      await remote.fill(ranges, signal);
      signal.throwIfAborted();
    }
  }
}

/**
 * Lamina PDF's document: opened without the password first, so `needsPassword` says whether a
 * restarted worker must ask again (trying is cheap: opening reads the xref, not the file).
 */
async function openModel(
  open: (options: OpenOptions) => Promise<Document> | Document,
  options: ViewerOpenOptions,
): Promise<{ model: Document; needsPassword: boolean }> {
  const { password, limits } = options;
  const withLimits = limits === undefined ? {} : { limits };
  try {
    return { model: await open(withLimits), needsPassword: false };
  } catch (e) {
    if (password === undefined || password === "" || !isLaminaPdfError(e, "password-required")) throw e;
    return { model: await open({ ...withLimits, password }), needsPassword: true };
  }
}

/** Where an opening document's bytes are: in memory, at a remote site, or in an OPFS copy. */
type DocumentBytes =
  | { readonly kind: "memory"; readonly bytes: Uint8Array }
  | { readonly kind: "remote"; readonly source: HttpRangeByteSource }
  | { readonly kind: "copy"; readonly source: OpfsByteSource };

/** Where `init`'s bytes will be read from. */
async function bytesFor(init: SourceInit, options: ViewerOpenOptions, signal: AbortSignal): Promise<DocumentBytes> {
  if (isRemote(init)) {
    const remote = await HttpRangeByteSource.open(init.url, { signal, onProgress: (p) => emit("progress", p) });
    // A server without range support sent the whole file: read it as a local one.
    return remote.whole === undefined ? { kind: "remote", source: remote } : { kind: "memory", bytes: remote.whole };
  }
  // A large file is read from its OPFS copy where the browser can keep one, else from memory.
  if (init instanceof File && init.size > (options.copyAbove ?? COPY_ABOVE)) {
    const copy = await OpfsByteSource.open(init, {
      signal,
      onProgress: (loaded, total) => emit("progress", { loaded, total }),
      keep: options.keepCopy ?? true,
    });
    if (copy !== undefined) return { kind: "copy", source: copy };
  }
  return { kind: "memory", bytes: await bytesOf(init, signal) };
}

/** Opens the worker's document (closing any previous one) and reports its warnings as events. */
export async function openDocument(
  init: SourceInit,
  options: ViewerOpenOptions,
  signal: AbortSignal,
  ticket?: number,
): Promise<OpenedDocument> {
  const engine = currentBackend();
  const from = await bytesFor(init, options, signal);
  try {
    const document = await openFrom(engine, from, options, signal);
    if (open !== undefined) open.answer = { document, ticket };
    return document;
  } catch (e) {
    if (from.kind === "copy") from.source.close();
    throw e;
  }
}

/** Opens Lamina PDF's document and the backend's over `from`. */
async function openFrom(
  engine: RenderBackend,
  from: DocumentBytes,
  options: ViewerOpenOptions,
  signal: AbortSignal,
): Promise<OpenedDocument> {
  const { password, renderLimits } = options;
  const { model, needsPassword } =
    from.kind === "memory"
      ? await openModel((o) => Document.open(from.bytes, o), options)
      : await openModel((o) => Document.openRange(from.source, { ...o, signal }), options);
  const remote = from.kind === "remote" ? from.source : undefined;
  let render: RenderDocument;
  let pages: OpenedDocument["pages"];
  let title: string | undefined;
  try {
    const source = from.kind === "memory" ? bytesSource(from.bytes) : from.source;
    const renderOptions = {
      ...(password === undefined ? {} : { password }),
      ...(renderLimits === undefined ? {} : { limits: renderLimits }),
    };
    render = await filling(remote, () => engine.open(source, renderOptions), signal);
    try {
      pages = await model.ensure(() => model.pages, { signal });
      title = (await model.ensure(() => model.info, { signal })).title?.trim();
    } catch (e) {
      render.close();
      throw e;
    }
  } catch (e) {
    model.free();
    throw e;
  }
  // A cancel sent while the synchronous opens ran is only read now: an aborted open must not
  // replace the document the page still thinks is open.
  await yieldToMessages();
  if (signal.aborted) {
    render.close();
    model.free();
    throw new RenderError("cancelled", "the open was cancelled");
  }
  // The new pair is open: only now is the previous document closed, so a failed open (a wrong
  // password, a broken file) leaves it working.
  closeOpen();
  open = {
    id: nextDocument++,
    model,
    render,
    remote,
    local: from.kind === "copy" ? from.source : undefined,
    reopen: reopenFor(model, from.kind === "memory" ? from.bytes : undefined, options),
  };
  // ADR 0003 warnings (a repaired xref, a dropped object) for the UI.
  for (const warning of model.warnings) emit("warning", warning);
  return {
    pageCount: pages.length,
    pages,
    id: open.id,
    needsPassword,
    backend: { name: engine.name, version: engine.version },
    ...(title === undefined || title === "" ? {} : { title }),
  };
}

/** The worker's WebAssembly memory in bytes: the backend's (0 before one loads) and Lamina PDF's. */
export function workerMemory(): WorkerMemory {
  const sdk = Object.values(wasmMemory()).reduce((sum, bytes) => sum + bytes, 0);
  return { backend: backend?.memory() ?? 0, sdk };
}

/**
 * The open document's answer and the page's number for the open that made it: after a cancelled
 * open, the page asks, since the cancel can cross an open this worker had already committed.
 */
export function currentDocument():
  | { readonly document: OpenedDocument; readonly ticket: number | undefined }
  | undefined {
  return open?.answer;
}

export function closeDocument(): undefined {
  closeOpen();
  return undefined;
}

/** The open document (`disposed` when there is none, or when `id` names a document since replaced). */
export function current(id?: number): Open {
  if (open === undefined || (id !== undefined && id !== open.id)) {
    throw new RenderError("disposed", "that document is not open in this worker");
  }
  return open;
}
