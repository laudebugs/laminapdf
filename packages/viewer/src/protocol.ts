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
  AddLink,
  ArticleThread,
  AttachmentFile,
  Collection,
  ContentsListing,
  ContentsPage,
  ContentsPlacement,
  DocumentAttachment,
  DocumentMetadata,
  DocumentProperties,
  ExplicitDestination,
  FieldEdit,
  InitialView,
  Limits,
  MergeOptions,
  NamedDestination,
  Op,
  OutlineNode,
  OutlineTarget,
  PageAnnotation,
  PageBoxesRead,
  PageInfo,
  PageLabelRange,
  PageLink,
  PlannedSave,
  ReferenceEdit,
  Reflow,
  SaveOptions,
  SaveReport,
  StructureNode,
  Target,
  TargetView,
  Violation,
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
  /**
   * Every page in its new order, with its size, when the call inserted, deleted or moved pages:
   * a viewer follows the pages by ID. Absent when the page list stayed as it was.
   */
  readonly order?: readonly PageInfo[];
  /**
   * Every page in order, with its new label, when the call changed the page labels but not the
   * page list. Labels draw nothing on the page, so no page is drawn again.
   */
  readonly relabelled?: readonly PageInfo[];
  /** Each step an undo or redo took back or made again, in order, with how it went (none for an edit). */
  readonly reverted?: readonly RevertedStep[];
  /** The IDs of the bookmarks the edit added, in order (none for an undo or redo). */
  readonly createdOutlineItems?: readonly string[];
  /** The IDs of the links the edit added, in order (none for an undo or redo). */
  readonly createdAnnotations?: readonly string[];
  /** The IDs of the files the edit attached, in order (none for an undo or redo). */
  readonly createdAttachments?: readonly string[];
  /** The edit found the outline's links broken and rebuilt them. */
  readonly outlineRepaired?: boolean;
  /** What a page deletion did to the references to the deleted pages (none for an undo or redo). */
  readonly references?: readonly ReferenceEdit[];
  /** What a page duplication or copy did to form fields (none for an undo or redo). */
  readonly fields?: readonly FieldEdit[];
  /** The contents page a `createContentsPage` step inserted, which a restarted worker inserts again. */
  readonly contents?: ContentsInsert;
}

/** The most steps an edit session keeps: the oldest go after this many (ADR 0006 §4). */
export const HISTORY_DEPTH = 500;

/** One step of an edit session: a transaction, as the history lists it. */
export interface EditStep {
  /** Unique in the session, and increasing. */
  readonly id: number;
  /** What it did, in the words its caller gave ("Rotate page 5"). */
  readonly label: string;
  /**
   * The pages it touched (0-based), in order, where they are when the history was read: a page
   * moved since is named at its new place, and one deleted since is left out.
   */
  readonly pages: readonly number[];
  /** The IDs of the pages it touched, which stay theirs when pages move. */
  readonly pageIds: readonly string[];
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

/**
 * A copy of the document with ops applied that the document itself never takes
 * (`copyWithEdit`): what a save with them would write. `history` is the session's after it, which
 * loses its undone steps (applying the ops cleared them in the model).
 */
export interface EditedCopy {
  readonly bytes: Uint8Array;
  readonly report: SaveReport;
  readonly history: EditHistory;
}

/**
 * A contents page laid out from the bookmarks and how it was placed (`createContentsPage`): the
 * step a restarted worker inserts again, since a page's text can't travel as ops.
 */
export interface ContentsInsert {
  readonly placement: ContentsPlacement;
  readonly page: ContentsPage;
}

/**
 * What a contents page would list, for a preview: the listing, and with a font the number of
 * pages it takes laid out in it, so `entries[i].labels[pages - 1]` is the label each entry prints.
 */
export interface ContentsPreview extends ContentsListing {
  /** The pages it takes; 1 when it wasn't laid out (no font given). */
  readonly pages: number;
}

/**
 * The ops of one step and its label, as a restarted worker replays them (`restoreEdits`); a
 * contents page's step has no ops but its `contents`.
 */
export interface ReplayedStep {
  readonly ops: readonly Op[];
  readonly contents?: ContentsInsert;
  readonly label: string;
  readonly at: number;
}

/** A file the open document took as a source document: its ID (the SHA-256 of its bytes) and its page IDs, in order. */
export interface AddedSource {
  readonly id: string;
  readonly pages: readonly string[];
}

/** What a dialog shows of a file before its pages come in (`previewSource`). */
export interface SourcePreview {
  /** What its pages are drawn by (`renderSource`) until `closeSource`. */
  readonly key: number;
  /** Each page's displayed size in points (`/Rotate` applied) and rotation. */
  readonly pages: readonly { readonly width: number; readonly height: number; readonly rotate: number }[];
  /** Whether it is encrypted: its pages come in unencrypted. */
  readonly encrypted: boolean;
  /** The PDF/A part and conformance its metadata claims (`"2B"`), if any. */
  readonly pdfa?: string;
  /** The PDF/UA part it claims, if any. */
  readonly pdfua?: number;
  /** Its title, if its metadata has one. */
  readonly title?: string;
}

/** How `writeParts` hands back the files it made. */
export interface PartsOutput {
  /** One zip of them, under these names and dated `modified` (ms since the epoch); else each file on its own. */
  readonly zip?: { readonly names: readonly string[]; readonly modified: number };
}

/** The files `writeParts` made: each one, or one zip of them. */
export type WrittenParts = { readonly files: readonly Uint8Array[] } | { readonly zip: Uint8Array };

/** What `writeParts` reports after each file. */
export interface PartsProgress {
  readonly done: number;
  readonly total: number;
}

/** A merge into a new document (`mergeFiles`). */
export interface MergeFilesRequest {
  /** The open document, first, under this title, when it takes part: its pages as its edits leave them. */
  readonly open?: { readonly id: number; readonly title: string };
  /** The other files, in order. */
  readonly files: readonly { readonly bytes: Uint8Array; readonly password?: string; readonly title?: string }[];
  readonly options?: MergeOptions;
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
  /** The files document `document` carries, each with what holds it, as its edits leave them. */
  attachments: { params: [document: number]; result: DocumentAttachment[] };
  /** The bytes of attachment `id` of document `document`, exactly as attached, and its checksum's verdict. */
  attachmentBytes: { params: [id: string, document: number]; result: AttachmentFile };
  /** The largest file, in bytes, document `document` can read back out and so attach. */
  attachmentLimit: { params: [document: number]; result: number };
  /** The portfolio document `document` is, or `undefined`. */
  collection: { params: [document: number]; result: Collection | undefined };
  /** Keeps `bytes` in document `document`'s session for the ops that name them (`attachFile`); answers their ID. */
  addBlob: { params: [bytes: Uint8Array, document: number]; result: string };
  /** Page `index`'s boxes in user space, as document `document`'s edits leave them. */
  pageBoxes: { params: [index: number, document: number]; result: PageBoxesRead };
  /** Document `document`'s page label ranges, as its edits leave them. */
  labelRanges: { params: [document: number]; result: PageLabelRange[] };
  /** What a contents page placed as `placement` would list in document `document`, for a preview. */
  contentsListing: {
    params: [placement: ContentsPlacement, document: number, font?: Uint8Array];
    result: ContentsPreview;
  };
  /**
   * Lays out a contents page of document `document`'s bookmarks in `font` (TrueType or OpenType
   * bytes; the text module loads for it) and inserts it as one step; the result carries the page
   * (`contents`) so a restarted worker can insert it again.
   */
  createContentsPage: {
    params: [placement: ContentsPlacement, font: Uint8Array, document: number, label?: string];
    result: EditResult;
  };
  planSave: { params: [options: SaveOptions, document: number]; result: PlannedSave };
  /** What saving document `document` with `ops` applied would break, before they apply: each signature restriction they violate. */
  wouldViolate: { params: [ops: readonly Op[], document: number]; result: Violation[] };
  copyEdits: { params: [options: SaveOptions, document: number]; result: SavedCopy };
  /** A copy of document `document` as a save with `options` writes it with `ops` applied, which leaves the document as it was. */
  copyWithEdit: { params: [ops: readonly Op[], options: SaveOptions, document: number]; result: EditedCopy };
  adoptEdits: { params: [token: number, document: number]; result: EditHistory };
  /**
   * A restarted worker's document takes the edits the old one had: `steps` applied again, one
   * transaction each, then the newest `undone` of them undone, and drawn once.
   */
  restoreEdits: {
    params: [
      steps: readonly ReplayedStep[],
      undone: number,
      document: number,
      sources?: readonly Uint8Array[],
      blobs?: readonly Uint8Array[],
    ];
    result: EditResult;
  };
  /** Opens a file to preview before its pages come in: its pages, claims and title. */
  previewSource: { params: [bytes: Uint8Array, password?: string]; result: SourcePreview };
  /** Page `page` of preview `key`, drawn at `scale`. */
  renderSource: { params: [key: number, page: number, scale: number]; result: RenderedBitmap };
  /** Closes preview `key`. */
  closeSource: { params: [key: number]; result: undefined };
  /** Makes a file a source document of document `document`, for the ops that copy its pages. */
  addSource: { params: [bytes: Uint8Array, password: string | undefined, document: number]; result: AddedSource };
  /** One new file per list of page IDs of document `document`, as `output` says; progress as `PartsProgress`. */
  writeParts: {
    params: [ranges: readonly (readonly string[])[], output: PartsOutput, document: number];
    result: WrittenParts;
  };
  /** One new document merged from files (and the open document, when `request.open` names it). */
  mergeFiles: { params: [request: MergeFilesRequest]; result: Uint8Array };
  pageInfo: { params: [index: number]; result: PageInfo };
  render: { params: [request: RenderRequest, document: number]; result: RenderedBitmap };
  memory: { params: []; result: WorkerMemory };
  pageText: { params: [index: number]; result: PageTextData };
  structure: { params: []; result: StructureData };
  reflow: { params: []; result: Reflow };
  networkLog: { params: []; result: WorkerNetworkLog };
  outline: { params: []; result: OutlineNode[] };
  namedDestination: { params: [name: string]; result: Target | undefined };
  namedDestinations: { params: []; result: NamedDestination[] };
  destinationAt: { params: [page: number, view: TargetView]; result: ExplicitDestination };
  linkOnArea: {
    params: [page: number, area: readonly [number, number, number, number], target: OutlineTarget];
    result: AddLink;
  };
  linkOnText: {
    params: [page: number, lines: readonly (readonly [number, number, number, number])[], target: OutlineTarget];
    result: AddLink;
  };
  initialView: { params: []; result: InitialView };
  threads: { params: []; result: ArticleThread[] };
  pageLinks: { params: [index: number]; result: readonly PageLink[] };
  pageAnnotations: { params: [index: number]; result: readonly PageAnnotation[] };
  properties: { params: []; result: DocumentProperties };
  printPage: { params: [index: number, options: PrintOptions]; result: PrintedPage };
  exportPage: { params: [index: number, options: ExportPageOptions]; result: ExportedPage };
  exportTiff: { params: [indexes: readonly number[], options: ExportTiffOptions]; result: ExportedTiff };
  fileBytes: { params: []; result: Uint8Array };
  extractText: { params: [options: ExtractTextOptions]; result: ExtractedText };
  extractPictures: { params: [options: ExtractPicturesOptions]; result: ExtractedPictures };
  exportSummary: { params: []; result: ExportSummary };
  linksAndBookmarks: { params: [format: "json" | "csv"]; result: LinksAndBookmarks };
  imagesToPdf: {
    params: [images: readonly ImageToPdf[], options: ImagesToPdfOptions];
    result: Uint8Array<ArrayBuffer>;
  };
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
  /** Draw comments and filled-in form fields, as the page shows them on screen (default `true`). */
  readonly annotations?: boolean;
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
  /** Draw comments and filled-in form fields, as the page shows them on screen (default `true`). */
  readonly annotations?: boolean;
}

/** Pages exported as TIFF. */
export interface ExportedTiff {
  /** One file, multi-page, or one file per page, in page order. */
  readonly files: readonly Uint8Array<ArrayBuffer>[];
  /** Each page's resolution, in page order: below the one asked for when a page was too large. */
  readonly dpis: readonly number[];
}

/** How the open document's text is exported. */
export interface ExtractTextOptions {
  /**
   * Plain text, Markdown (headings, lists and tables), or the `laminapdf.extract/1` JSON, which
   * gives every block its box (`bbox`) and page.
   */
  readonly format: "text" | "markdown" | "json";
  /** The pages (0-based) in this order; every page when absent. */
  readonly pages?: readonly number[];
  /**
   * Where the reading order comes from: `auto` follows the structure tree when it looks right,
   * else the layout; `tagged` follows the structure tree wherever the file has one.
   */
  readonly readingOrder: "auto" | "tagged";
}

/** The open document's text, as {@link ExtractTextOptions} asked. */
export interface ExtractedText {
  /** The text, the Markdown, or the JSON (indented two spaces). */
  readonly text: string;
  /**
   * The file says it is tagged (`/MarkInfo /Marked true`): its headings and lists come from its
   * tags, not guessed from the layout.
   */
  readonly tagged: boolean;
}

/** How the open document's pictures are exported. */
export interface ExtractPicturesOptions {
  /** The pages (0-based) to read; every page when absent. */
  readonly pages?: readonly number[];
  /**
   * Keep JPEG and JPEG 2000 pictures byte for byte, as the file stores them. When false, JPEGs
   * are decoded and written as PNG (where the worker has `createImageBitmap` and
   * `OffscreenCanvas`); JPEG 2000 stays as it is, since browsers can't decode it.
   */
  readonly keepOriginals: boolean;
  /** Leave out pictures smaller than this many pixels on their longer side (0 keeps them all). */
  readonly minimumSize: number;
}

/** One picture of the open document, as a file. */
export interface ExtractedPicture {
  /** The first page (0-based) that draws it: a picture drawn on several pages comes once. */
  readonly page: number;
  /** A file name unique among the pictures: `page-3-picture-2.png` (page and picture 1-based). */
  readonly name: string;
  /** The file: PNG, JPEG (`.jpg`) or JPEG 2000 (`.jp2`), as `name`'s extension says. */
  readonly bytes: Uint8Array<ArrayBuffer>;
  /** Its size in pixels. */
  readonly width: number;
  readonly height: number;
}

/** The open document's pictures, page by page. */
export interface ExtractedPictures {
  readonly pictures: readonly ExtractedPicture[];
  /**
   * The pictures left out: those that can't be decoded (JBIG2, an unsupported colour space) and
   * those under `minimumSize`.
   */
  readonly skipped: number;
}

/** What the open document has to export, for an export dialog to offer. */
export interface ExportSummary {
  /** Its links, on every page. */
  readonly links: number;
  /** Its bookmarks (outline items), at every level. */
  readonly bookmarks: number;
  /** The file says it is tagged (`/MarkInfo /Marked true`). */
  readonly tagged: boolean;
}

/** The open document's links and bookmarks, as files. */
export interface LinksAndBookmarks {
  /** `laminapdf.links/1` JSON, or CSV with a header row. */
  readonly links: string;
  /** `laminapdf.outline/1` JSON, or CSV with a header row and each item's depth in `level`. */
  readonly bookmarks: string;
}

/** One picture for {@link ImagesToPdfOptions | imagesToPdf}. */
export interface ImageToPdf {
  readonly bytes: Uint8Array<ArrayBuffer>;
  /** Its MIME type: `image/jpeg` (embedded as it is) or `image/png`. Others are refused. */
  readonly type: string;
}

/** How imagesToPdf lays its pictures out, one a page. */
export interface ImagesToPdfOptions {
  /** `image`: each page the picture's size; `letter` or `a4`: the picture fitted and centred, the page turned its way. */
  readonly pageSize: "image" | "letter" | "a4";
  /** The space around each picture, in points. */
  readonly margin: number;
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
 * RenderRequest (page, scale, clip, rotation, flags, background, appearance and the annotations
 * it hides), so two requests share a run only when their pixels are the same (ADR 0009 §3.3).
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

/** `key` (a {@link viewKey}) for the same picture of a page now at index `page`. */
export function viewKeyAt(key: string, page: number): string {
  const fields = JSON.parse(key) as unknown[];
  fields[1] = page;
  return JSON.stringify(fields);
}

function keyFields(request: RenderRequest, id: number): unknown[] {
  const { page, scale, clip, rotation = 0, flags, background, appearance, hiddenAnnotations = [] } = request;
  // Only a render that hides annotations names them, so every other key stays as it was.
  const hidden = hiddenAnnotations.length === 0 ? [] : [[...new Set(hiddenAnnotations)].sort((a, b) => a - b)];
  return [
    id,
    page,
    scale,
    clip === undefined ? null : [clip.x, clip.y, clip.width, clip.height],
    rotation,
    flagsOf(flags),
    background === undefined ? "white" : background,
    appearanceKey(appearance),
    ...hidden,
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
