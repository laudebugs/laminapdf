// The viewer's methods on the document worker (C3a, ADR 0009 §3). They extend `@laminapdf/sdk/worker`
// (L2a): one protocol, one queue and one cancellation path for both, with the viewer's methods
// added through `serve({ methods })` in the worker and `client.request` on the page. This module
// holds what both sides agree on: the method map (params and results), the priorities and the
// coalescing key.

import { flagsOf, type RenderLimits, type RenderRequest } from "@laminapdf/render";
import type { Limits, PageInfo } from "@laminapdf/sdk";

/** Which engine draws the pages (ADR 0004: PDFium by default, hayro the CI oracle). */
export type BackendName = "pdfium" | "hayro";

/** What the worker loads a backend from. */
export interface BackendInit {
  readonly backend: BackendName;
  /** The engine's `.wasm`: required for PDFium (its URL from your bundler); hayro finds its own. */
  readonly wasm?: string | ArrayBuffer | Uint8Array | WebAssembly.Module;
}

/** What a document is opened from: sent to the worker, which reads it there. */
export type SourceInit = Blob | ArrayBuffer | Uint8Array;

/** `open`'s options. The password crosses to the worker once and is kept only there. */
export interface ViewerOpenOptions {
  readonly password?: string;
  /** lamina's reading limits (ADR 0003). */
  readonly limits?: Partial<Limits>;
  /** The backend's render limits (ADR 0004). */
  readonly renderLimits?: Partial<RenderLimits>;
}

/** An open document, as layout needs it up front. */
export interface OpenedDocument {
  /** Which open of this worker it is: renders name it, so one queued for a replaced document fails. */
  readonly id: number;
  readonly pageCount: number;
  /** Every page's displayed size in points (`/Rotate` applied), rotation, ID and label. */
  readonly pages: readonly PageInfo[];
  /** Whether the file can't be opened without its password, so a restarted worker must ask again. */
  readonly needsPassword: boolean;
  readonly backend: { readonly name: BackendName; readonly version: string };
}

/** A rendered tile on the page: an ImageBitmap transferred from the worker. Close it when done. */
export interface RenderedBitmap {
  readonly bitmap: ImageBitmap;
  readonly width: number;
  readonly height: number;
}

/** The viewer's methods: params and result per method. `client.call` is typed by it. */
export interface Methods {
  initBackend: { params: [BackendInit]; result: { name: BackendName; version: string } };
  openDocument: { params: [source: SourceInit, options: ViewerOpenOptions]; result: OpenedDocument };
  closeDocument: { params: []; result: undefined };
  pageInfo: { params: [index: number]; result: PageInfo };
  render: { params: [request: RenderRequest, document: number]; result: RenderedBitmap };
  /** Test-only: blocks the worker synchronously for `ms`. */
  __busy: { params: [ms: number]; result: undefined };
  /** Test-only: every render's `scale` in the order they started. */
  __renderLog: { params: []; result: number[] };
  /** Test-only: an uncaught error in the worker (a crash). */
  __crash: { params: []; result: never };
}

/** A viewer method's name. */
export type MethodName = keyof Methods;
/** Method `M`'s parameters. */
export type Params<M extends MethodName> = Methods[M]["params"];
/** Method `M`'s result. */
export type Result<M extends MethodName> = Methods[M]["result"];

/**
 * Who is waiting (ADR 0009 §3.3): `visible` tiles first, then `prefetch` (just off screen), then
 * `background` (thumbnails, search, text). Document calls run above all three.
 */
export type Priority = "visible" | "prefetch" | "background";

/** The worker queue's numbers for each priority (higher first). */
export const PRIORITY: { readonly [P in Priority | "document"]: number } = Object.freeze({
  document: 4,
  visible: 3,
  prefetch: 2,
  background: 1,
});

/**
 * The coalescing key of a render on document `id`: the document and the whole normalised
 * RenderRequest (page, scale, clip, rotation, flags and background), so two requests share a run
 * only when their pixels are the same (ADR 0009 §3.3).
 */
export function renderKey(request: RenderRequest, id: number): string {
  const { page, scale, clip, rotation = 0, flags, background } = request;
  return JSON.stringify([
    id,
    page,
    scale,
    clip === undefined ? null : [clip.x, clip.y, clip.width, clip.height],
    rotation,
    flagsOf(flags),
    background === undefined ? "white" : background,
  ]);
}
