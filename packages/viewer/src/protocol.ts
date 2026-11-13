// The viewer's methods on the document worker (ADR 0009 §3). They extend `@laminapdf/sdk/worker`:
// one protocol, one queue and one cancellation path for both, with the viewer's methods
// added through `serve({ methods })` in the worker and `client.request` on the page. This module
// holds what both sides agree on: the method map (params and results), the priorities and the
// coalescing key.

import {
  type CloneableWasmSource,
  flagsOf,
  type PageAppearance,
  type RenderLimits,
  type RenderRequest,
} from "@laminapdf/render";
import type {
  DocumentMetadata,
  DocumentProperties,
  Limits,
  Op,
  OutlineNode,
  PageInfo,
  PageLink,
  PlannedSave,
  Reflow,
  SaveOptions,
  SaveReport,
  StructureNode,
  Target,
  XmpText,
} from "@laminapdf/sdk";
import type { FindOptions } from "./find/fold.js";
import type { FindSnippet } from "./find/page.js";

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
  /**
   * A local file larger than this many bytes is copied into the origin-private file system and
   * read from there, not held in memory (default 200 MiB; tests lower it).
   */
  readonly copyAbove?: number;
  /** Keep that copy once the document closes, for next time (default true); false removes it then. */
  readonly keepCopy?: boolean;
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
  /**
   * A hash of the file's first revision (`"fp1:"` and a SHA-256 in hex): the same for a renamed
   * copy and after an incremental update, so a reading position kept under it follows the file.
   * Absent when it couldn't be computed (no Web Crypto, a failed read): the document still opens.
   */
  readonly fingerprint?: string;
  readonly backend: { readonly name: BackendName; readonly version: string };
  /** The title in the file's metadata, for the window's title; absent when the file has none. */
  readonly title?: string;
  /**
   * Whether the file is encrypted: it can't take an edit until saving can encrypt again (E11),
   * since its edits are written into memory to be drawn, and that save is refused (`encrypted-base`).
   */
  readonly encrypted: boolean;
}

/**
 * What an edit, an undo or a redo did, for the view to redraw (`applyEdit`, `undoEdit`,
 * `redoEdit`, `goToStep`): the pages it touched, as they are after it, and the edit session's
 * history now. Each page's size is its displayed size in points, so a quarter turn swaps its width
 * and height. An edit that touched no page (the document's metadata) leaves `pages` empty.
 */
export interface EditResult {
  readonly pages: readonly PageInfo[];
  readonly history: EditHistory;
  /** Each step an undo or redo took back or made again, in order, with how it went (none for an edit). */
  readonly reverted?: readonly RevertedStep[];
}

/** The most steps an edit session keeps: the oldest go after this many (ADR 0006 §4). */
export const HISTORY_DEPTH = 500;

/** One step of an edit session: a transaction, as the history lists it. */
export interface EditStep {
  /** Unique in the session, and increasing. */
  readonly id: number;
  /** What it did, in the words its caller gave ("Rotate page 5"). */
  readonly label: string;
  /** The pages it touched (0-based), in order. */
  readonly pages: readonly number[];
  /** When it was made, in ms since the epoch. */
  readonly at: number;
  /**
   * Its last undo (`"undo"`) or redo (`"redo"`) restored only part of it (`UndoOutcome::Partial`):
   * a later change replaced some of what it changed, which keeps the later value.
   */
  readonly partial?: "undo" | "redo";
}

/**
 * An edit session's history (the worker holds the op log; this is its summary): the steps kept,
 * oldest first, of which the first `position` are done and the rest undone, ready to redo.
 */
export interface EditHistory {
  readonly steps: readonly EditStep[];
  readonly position: number;
  /** How many of the oldest steps the depth limit dropped ({@link HISTORY_DEPTH}). */
  readonly dropped: number;
  /**
   * Transactions between the document now and its last save, or as opened (ADR 0006 §7, the
   * semantic level): 0 when they match, as after undoing every step since.
   */
  readonly unsaved: number;
  /** An edit failed and couldn't be undone: nothing more can be edited until the document opens again. */
  readonly diverged: boolean;
}

/** How an undo or redo of one step went. */
export interface RevertedStep {
  readonly step: EditStep;
  readonly way: "undo" | "redo";
  /**
   * `partial`: some of it was left alone, because a later change replaced it; `skipped`: nothing
   * could be restored, and the step is gone from the history.
   */
  readonly outcome: "applied" | "partial" | "skipped";
}

/**
 * A copy of the document as a save writes it (`copyEdits`), for the page to write where the file
 * lives; `token` names it to `adoptEdits` once written.
 */
export interface SavedCopy {
  readonly token: number;
  readonly bytes: Uint8Array;
  readonly report: SaveReport;
}

/** The ops of one step and its label, as a restarted worker replays them (`restoreEdits`). */
export interface ReplayedStep {
  readonly ops: readonly Op[];
  readonly label: string;
  readonly at: number;
}

/** An edit session with nothing in it. */
export const EMPTY_HISTORY: EditHistory = { steps: [], position: 0, dropped: 0, unsaved: 0, diverged: false };

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
  /** `ticket`: the page's number for this open, which `currentDocument` gives back. */
  openDocument: { params: [source: SourceInit, options: ViewerOpenOptions, ticket?: number]; result: OpenedDocument };
  /** The open document's answer and its open's ticket; asked after a cancelled open. */
  currentDocument: {
    params: [];
    result: { readonly document: OpenedDocument; readonly ticket: number | undefined } | undefined;
  };
  closeDocument: { params: []; result: undefined };
  /**
   * Applies `ops` to document `document` as one transaction, then redraws from the result: the
   * pages it touched are drawn from the edited file from then on.
   */
  applyEdit: { params: [ops: readonly Op[], document: number, label?: string]; result: EditResult };
  /** Undoes the newest done step of document `document`'s edit session, and redraws. */
  undoEdit: { params: [document: number]; result: EditResult };
  /** Redoes the oldest undone step, and redraws. */
  redoEdit: { params: [document: number]; result: EditResult };
  /** Undoes or redoes steps until `position` of them are done, and redraws once. */
  goToStep: { params: [position: number, document: number]; result: EditResult };
  /** The edit session's history. */
  editHistory: { params: [document: number]; result: EditHistory };
  /** Document `document`'s metadata as its edits leave it. */
  metadata: { params: [document: number]; result: DocumentMetadata };
  /** Document `document`'s XMP metadata as text, as its edits leave it. */
  xmpPacket: { params: [document: number]; result: XmpText };
  planSave: { params: [options: SaveOptions, document: number]; result: PlannedSave };
  copyEdits: { params: [options: SaveOptions, document: number]; result: SavedCopy };
  adoptEdits: { params: [token: number, document: number]; result: EditHistory };
  /**
   * A restarted worker's document takes the edits the old one had: `steps` applied again, one
   * transaction each, then the newest `undone` of them undone, and drawn once.
   */
  restoreEdits: { params: [steps: readonly ReplayedStep[], undone: number, document: number]; result: EditResult };
  pageInfo: { params: [index: number]; result: PageInfo };
  render: { params: [request: RenderRequest, document: number]; result: RenderedBitmap };
  memory: { params: []; result: WorkerMemory };
  pageText: { params: [index: number]; result: PageTextData };
  structure: { params: []; result: StructureData };
  reflow: { params: []; result: Reflow };
  networkLog: { params: []; result: WorkerNetworkLog };
  outline: { params: []; result: OutlineNode[] };
  namedDestination: { params: [name: string]; result: Target | undefined };
  pageLinks: { params: [index: number]; result: readonly PageLink[] };
  properties: { params: []; result: DocumentProperties };
  printPage: { params: [index: number, options: PrintOptions]; result: PrintedPage };
  exportPage: { params: [index: number, options: ExportPageOptions]; result: ExportedPage };
  exportTiff: { params: [indexes: readonly number[], options: ExportTiffOptions]; result: ExportedTiff };
  fileBytes: { params: []; result: Uint8Array };
  find: { params: [query: string, options: FindOptions, pages: readonly number[]]; result: FindPageResult[] };
  readingOrder: { params: [index: number]; result: ReadingBlock[] };
}

/**
 * One block of a page's text in reading order, as Lamina PDF's extraction finds it (ADR 0030):
 * where it is, in page space (points from the displayed page's top-left, y down), and what kind
 * of block it is. Copy as paragraph joins the selected text block by block in this order.
 */
export interface ReadingBlock {
  /** Its box: `[x, y, width, height]`. */
  readonly bbox: readonly [number, number, number, number];
  /** `paragraph`, `heading`, `list-item`, `caption`, `footnote`, `table` or `other`. */
  readonly type: string;
}

/** How a page is drawn for printing. */
export interface PrintOptions {
  /** Dots per inch: 150 (standard) or 300 (high). Lowered for a page too large to hold. */
  readonly dpi: number;
  /** Draw annotations as their print flags say (comments, highlights, stamps); off leaves them out. */
  readonly annotations: boolean;
}

/** One page drawn for printing. */
export interface PrintedPage {
  /** The page as a PNG. */
  readonly image: Blob;
  /** Its displayed size in points (`/Rotate` applied), which the printed page takes. */
  readonly width: number;
  readonly height: number;
  /** The resolution it was drawn at: below the one asked for when the page was too large. */
  readonly dpi: number;
}

/** How a page is exported as an image. */
export interface ExportPageOptions {
  /** PNG (lossless) or JPEG (smaller, no transparency). */
  readonly format: "png" | "jpeg";
  /** Dots per inch. Lowered for a page too large to hold. */
  readonly dpi: number;
  /** JPEG quality, 1–100 (default 90). */
  readonly quality?: number;
}

/** One page exported as an image. */
export interface ExportedPage {
  /** The image file's bytes, its resolution recorded (PNG `pHYs`, JFIF density). */
  readonly image: Uint8Array<ArrayBuffer>;
  /** Its size in pixels. */
  readonly width: number;
  readonly height: number;
  /** The resolution it was drawn at: below the one asked for when the page was too large. */
  readonly dpi: number;
}

/** How pages are exported as TIFF. */
export interface ExportTiffOptions {
  /** Dots per inch. Lowered for a page too large to hold. */
  readonly dpi: number;
  /**
   * Every page in one file. Pages whose pixels pass the export's memory budget (256 MiB) still
   * come as a file each.
   */
  readonly multiPage: boolean;
}

/** Pages exported as TIFF. */
export interface ExportedTiff {
  /** One file, multi-page, or one file per page, in page order. */
  readonly files: readonly Uint8Array<ArrayBuffer>[];
  /** Each page's resolution, in page order: below the one asked for when a page was too large. */
  readonly dpis: readonly number[];
}

/** One page's find results: each match as its quads (8 numbers per run it touches), in page space. */
export interface FindPageResult {
  readonly page: number;
  readonly matches: readonly (readonly number[])[];
  /** Each match in its line, in the same order. */
  readonly snippets: readonly FindSnippet[];
  /** The page has no text to search (a scan without an OCR layer). */
  readonly noText?: true;
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

/** Which JavaScript context made a request: the page, or the document worker. */
export type NetworkSource = "page" | "worker";

/** One network request a context made, as `watchNetwork` reports it. */
export interface NetworkRecord {
  readonly url: string;
  readonly method: string;
  /**
   * What made it: `fetch`, `xmlhttprequest`, `websocket` and `beacon` for wrapped calls, else the
   * Resource Timing `initiatorType` (`script`, `link`, `css`, `img`, `navigation` …).
   */
  readonly initiator: string;
  /** Bytes received, when known (0 otherwise). */
  readonly bytes: number;
  /** The call failed or was refused (a CORS refusal, a network error, a beacon not queued). */
  readonly failed: boolean;
  readonly source: NetworkSource;
  /** `Date.now()` when it was reported. */
  readonly at: number;
}

/** A request the Content Security Policy blocked: nothing was sent. */
export interface PolicyViolation {
  readonly directive: string;
  readonly blockedUri: string;
  readonly source: NetworkSource;
  readonly at: number;
}

/** The document worker's network log: every request and blocked request since it started. */
export interface WorkerNetworkLog {
  /** Different for each worker the client starts, so a restarted worker's records stay apart. */
  readonly worker: string;
  readonly requests: readonly NetworkRecord[];
  readonly violations: readonly PolicyViolation[];
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
 * RenderRequest (page, scale, clip, rotation, flags, background and appearance), so two requests
 * share a run only when their pixels are the same (ADR 0009 §3.3).
 */
export function renderKey(request: RenderRequest, id: number): string {
  return JSON.stringify(keyFields(request, id));
}

/**
 * The view's cache key of a render: its {@link renderKey}'s fields and the page's `revision`,
 * which each edit of the page bumps, so a bitmap of the page before an edit never answers for it
 * after. {@link renderKeyFields} reads both kinds of key.
 */
export function viewKey(request: RenderRequest, id: number, revision: number): string {
  return JSON.stringify([...keyFields(request, id), revision]);
}

function keyFields(request: RenderRequest, id: number): unknown[] {
  const { page, scale, clip, rotation = 0, flags, background, appearance } = request;
  return [
    id,
    page,
    scale,
    clip === undefined ? null : [clip.x, clip.y, clip.width, clip.height],
    rotation,
    flagsOf(flags),
    background === undefined ? "white" : background,
    appearanceKey(appearance),
  ];
}

/** The request field for `appearance`: none for the pages' own colours, as requests were before page appearances. */
export function appearanceField(appearance: PageAppearance): { appearance?: PageAppearance } {
  return appearance === "original" ? {} : { appearance };
}

/** A page appearance as a string: equal for the same appearance, whatever the object. */
export function appearanceKey(appearance: PageAppearance | undefined): string {
  if (appearance === undefined || typeof appearance === "string") return appearance ?? "original";
  const { ink: i, paper: p } = appearance;
  return `high-contrast:${i.r},${i.g},${i.b},${i.a}:${p.r},${p.g},${p.b},${p.a}`;
}
