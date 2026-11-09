// The viewer's methods on the document worker (ADR 0009 §3). They extend `@laminapdf/sdk/worker`:
// one protocol, one queue and one cancellation path for both, with the viewer's methods
// added through `serve({ methods })` in the worker and `client.request` on the page. This module
// holds what both sides agree on: the method map (params and results), the priorities and the
// coalescing key.

import { type CloneableWasmSource, flagsOf, type RenderLimits, type RenderRequest } from "@laminapdf/render";
import type { Limits, OutlineNode, PageInfo, StructureNode, Target } from "@laminapdf/sdk";

/** Which engine draws the pages (ADR 0004: PDFium by default, hayro the CI oracle). */
export type BackendName = "pdfium" | "hayro";

/** What the worker loads a backend from. */
export interface BackendInit {
  readonly backend: BackendName;
  /** The engine's `.wasm`: required for PDFium (its URL from your bundler); hayro finds its own. */
  readonly wasm?: CloneableWasmSource;
}

/**
 * A remote file the worker fetches itself, with HTTP Range requests (`HttpRangeByteSource`): its
 * bytes never pass through the page.
 */
export interface RemoteSource {
  readonly url: string;
}

/** What a document is opened from: sent to the worker, which reads it there. */
export type SourceInit = Blob | ArrayBuffer | Uint8Array | RemoteSource;

/** `open`'s options. The password crosses to the worker once and is kept only there. */
export interface ViewerOpenOptions {
  readonly password?: string;
  /** Lamina PDF's reading limits (ADR 0003). */
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
  /** The title in the file's metadata, for the window's title; absent when the file has none. */
  readonly title?: string;
}

/** A rendered tile on the page: an ImageBitmap transferred from the worker. Close it when done. */
export interface RenderedBitmap {
  readonly bitmap: ImageBitmap;
  readonly width: number;
  readonly height: number;
}

/**
 * The bytes of the worker's WebAssembly memories: the render backend's and Lamina PDF's (every
 * module loaded). Memories only grow, so this is also their peak.
 */
export interface WorkerMemory {
  readonly backend: number;
  readonly sdk: number;
}

/** The viewer's methods: params and result per method. `client.call` is typed by it. */
export interface Methods {
  initBackend: { params: [BackendInit]; result: { name: BackendName; version: string } };
  openDocument: { params: [source: SourceInit, options: ViewerOpenOptions]; result: OpenedDocument };
  closeDocument: { params: []; result: undefined };
  pageInfo: { params: [index: number]; result: PageInfo };
  render: { params: [request: RenderRequest, document: number]; result: RenderedBitmap };
  memory: { params: []; result: WorkerMemory };
  pageText: { params: [index: number]; result: PageTextData };
  structure: { params: []; result: StructureData };
  outline: { params: []; result: OutlineNode[] };
  namedDestination: { params: [name: string]; result: Target | undefined };
}

/** One text run of a page: glyphs of one font, size and direction on one baseline, in logical order. */
export interface TextRunData {
  /** Its text in logical order (document text; an `/ActualText` replacement when `actualText`). */
  readonly text: string;
  /** Font size in points, the text matrix applied. */
  readonly size: number;
  /** Vertical writing. */
  readonly vertical: boolean;
  /** The text replaces its glyphs (`/ActualText`). */
  readonly actualText: boolean;
  /** Drawn invisibly (`Tr` 3 or 7: an OCR layer, hidden text); selectable all the same. */
  readonly invisible: boolean;
  /** Its glyphs: `[start, end)` into `quads` (8 numbers each), as the SDK's `PageTextRun.glyphRange`. */
  readonly glyphRange: readonly [number, number];
  /** The marked-content ID it is in, which links it to the structure tree. */
  readonly mcid?: number;
}

/**
 * A page's text: its runs, and every glyph's quad as 8 numbers (bottom-left, bottom-right,
 * top-right, top-left, each `x, y`) in page space (ADR 0027: points from the displayed page's
 * top-left, y down).
 */
export interface PageTextData {
  readonly runs: readonly TextRunData[];
  readonly quads: Float32Array;
}

/** Whether the document says it is tagged, and its structure tree when it has one. */
export interface StructureData {
  readonly tagged: boolean;
  readonly tree?: readonly StructureNode[];
}

/** How much of a remote file the worker has read: the `progress` event. */
export interface LoadProgress {
  readonly loaded: number;
  readonly total: number;
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

/** The fields of a render key that say which pixels it holds (see {@link renderKey}). */
export interface RenderKeyFields {
  readonly scale: number;
  readonly rotation: number;
}

/** Reads back the scale and rotation from a {@link renderKey}. */
export function renderKeyFields(key: string): RenderKeyFields {
  const fields = JSON.parse(key) as unknown[];
  return { scale: fields[2] as number, rotation: fields[4] as number };
}

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
