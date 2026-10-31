// The one document a viewer worker holds (ADR 0009 §2: one worker per document): lamina's
// Document for the model (page sizes, warnings, later text and structure) and a RenderDocument
// for pixels, both opened from the same bytes. The password is used here and kept nowhere else.
import { Document } from "@lamina/pdf";
import { emit } from "@lamina/pdf/worker/server";
import { bytesSource, type RenderBackend, type RenderDocument } from "@lamina/render";
import { initHayro } from "@lamina/render-hayro";
import { initPdfium } from "@lamina/render-pdfium";
import type { BackendInit, OpenedDocument, SourceInit, ViewerOpenOptions } from "../protocol.js";
import { readAll, sourceOf } from "../source.js";

let backend: RenderBackend | undefined;
let loading: Promise<RenderBackend> | undefined;

interface Open {
  readonly model: Document;
  readonly render: RenderDocument;
}
let open: Open | undefined;

/** Loads the backend once (the client asks as soon as the worker starts, before a file is chosen). */
export async function initBackend(init: BackendInit): Promise<{ name: string; version: string }> {
  loading ??= init.backend === "hayro" ? initHayro(init.wasm as never) : initPdfium(init.wasm as never);
  const loaded = await loading;
  backend = loaded;
  return { name: loaded.name, version: loaded.version };
}

export function currentBackend(): RenderBackend {
  if (backend === undefined) throw new Error("initBackend was not called");
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
  const bytes = await readAll(sourceOf(init), signal);
  closeOpen();
  const { password, limits, renderLimits } = options;
  const model = Document.open(bytes, {
    ...(password === undefined ? {} : { password }),
    ...(limits === undefined ? {} : { limits }),
  });
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
  open = { model, render };
  // ADR 0003 warnings (a repaired xref, a dropped object) for the UI.
  for (const warning of model.warnings) emit("warning", warning);
  return {
    pageCount: model.pageCount,
    pages: model.pages,
    needsPassword: password !== undefined && password !== "",
    backend: { name: engine.name, version: engine.version },
  };
}

export function closeDocument(): undefined {
  closeOpen();
  return undefined;
}

/** The open document (`disposed` when there is none). */
export function current(): Open {
  if (open === undefined) {
    throw { code: "disposed", kind: "state", message: "no document is open in this worker" };
  }
  return open;
}
