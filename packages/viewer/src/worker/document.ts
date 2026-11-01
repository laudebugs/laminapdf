// The one document a viewer worker holds (ADR 0009 §2: one worker per document): lamina's
// Document for the model (page sizes, warnings, later text and structure) and a RenderDocument
// for pixels, both opened from the same bytes. The password is used here and kept nowhere else.

import { bytesSource, type RenderBackend, type RenderDocument, RenderError } from "@laminapdf/render";
import { initHayro } from "@laminapdf/render-hayro";
import { initPdfium } from "@laminapdf/render-pdfium";
import { Document, isLaminaError } from "@laminapdf/sdk";
import { emit, yieldToMessages } from "@laminapdf/sdk/worker/server";
import type { BackendInit, OpenedDocument, SourceInit, ViewerOpenOptions } from "../protocol.js";
import { readAll, sourceOf } from "../source.js";

let backend: RenderBackend | undefined;
/** Backends loaded or loading, by name; a failed load is forgotten, so the next call retries. */
const loading = new Map<string, Promise<RenderBackend>>();

interface Open {
  readonly id: number;
  readonly model: Document;
  readonly render: RenderDocument;
}
let open: Open | undefined;
let nextDocument = 1;

/** Loads the backend (the client asks as soon as the worker starts, before a file is chosen). */
export async function initBackend(init: BackendInit): Promise<{ name: string; version: string }> {
  let pending = loading.get(init.backend);
  if (pending === undefined) {
    pending = init.backend === "hayro" ? initHayro(init.wasm as never) : initPdfium(init.wasm as never);
    loading.set(init.backend, pending);
    pending.catch(() => loading.delete(init.backend));
  }
  const loaded = await pending;
  backend = loaded;
  return { name: loaded.name, version: loaded.version };
}

function currentBackend(): RenderBackend {
  if (backend === undefined)
    throw new RenderError("render-backend-unavailable", "no backend is loaded: call initBackend first");
  return backend;
}

function closeOpen(): void {
  if (open === undefined) return;
  open.render.close();
  open.model.free();
  open = undefined;
}

/** Opens the worker's document (closing any previous one) and reports its warnings as events. */
export async function openDocument(
  init: SourceInit,
  options: ViewerOpenOptions,
  signal: AbortSignal,
): Promise<OpenedDocument> {
  const engine = currentBackend();
  // Bytes are used as they are; a Blob is read in slices.
  const bytes =
    init instanceof Blob
      ? await readAll(sourceOf(init), signal)
      : init instanceof Uint8Array
        ? init
        : new Uint8Array(init);
  const { password, limits, renderLimits } = options;
  const withLimits = limits === undefined ? {} : { limits };
  // Whether the file needs the password: a restarted worker can reopen it without one only if not.
  // Trying without it first is cheap: opening reads the cross-reference table, not the file.
  let needsPassword = false;
  let model: Document;
  try {
    model = Document.open(bytes, withLimits);
  } catch (e) {
    if (password === undefined || password === "" || !isLaminaError(e, "password-required")) throw e;
    model = Document.open(bytes, { ...withLimits, password });
    needsPassword = true;
  }
  let render: RenderDocument;
  try {
    render = engine.open(bytesSource(bytes), {
      ...(password === undefined ? {} : { password }),
      ...(renderLimits === undefined ? {} : { limits: renderLimits }),
    });
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
  open = { id: nextDocument++, model, render };
  // ADR 0003 warnings (a repaired xref, a dropped object) for the UI.
  for (const warning of model.warnings) emit("warning", warning);
  return {
    pageCount: model.pageCount,
    pages: model.pages,
    id: open.id,
    needsPassword,
    backend: { name: engine.name, version: engine.version },
  };
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
