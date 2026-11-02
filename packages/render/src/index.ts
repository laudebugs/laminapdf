// The web render contract of ADR 0004 (todo C2c): the interface both web adapters implement
// (@laminapdf/render-pdfium, @laminapdf/render-hayro), the tile planner they share, and RenderError.
// It mirrors laminapdf-render's Rust trait: bounded tiles of a page at a scale, straight-alpha RGBA8
// rows of width × 4 bytes, validated once by planTile so both adapters agree on sizes.

/**
 * The bytes of a document, read synchronously (PDFium's file access can't await). A read
 * returns `undefined` when those bytes haven't arrived yet: the call then throws `data-pending`
 * with the ranges to fetch (ADR 0004 §1, the web form of `Pending`).
 */
export interface RenderSource {
  readonly size: number;
  /** `length` bytes at `offset` (fewer only at the end), or `undefined` while they haven't arrived. */
  readSync(offset: number, length: number): Uint8Array | undefined;
}

/** A source over bytes already in memory. */
export function bytesSource(bytes: Uint8Array): RenderSource {
  return {
    size: bytes.length,
    readSync: (offset, length) => bytes.subarray(offset, Math.min(bytes.length, offset + length)),
  };
}

/** Bounds on one render, checked before anything is allocated (`render-too-large`). */
export interface RenderLimits {
  /** Most pixels in one tile. Default 16,777,216 (4096 × 4096), iOS Safari's per-canvas cap. */
  readonly maxRenderPixels: number;
  /** Most pixels on either side of one tile. Default 16,384. */
  readonly maxRenderSide: number;
  /** Most input bytes a backend that reads the whole file (hayro) reads. Default 1 GiB. */
  readonly maxInputBytes: number;
}

/** The render limits when none are given (ADR 0004): 16.7 MP, 16,384 px a side, 1 GiB of input. */
export const DEFAULT_LIMITS: RenderLimits = Object.freeze({
  maxRenderPixels: 16_777_216,
  maxRenderSide: 16_384,
  maxInputBytes: 2 ** 30,
});

/** A view rotation, clockwise, added to the page's own `/Rotate`. */
export type ViewRotation = 0 | 90 | 180 | 270;

/** Whether `rotation` turns the page a quarter (90° or 270°), which swaps its width and height. */
export function isQuarterTurn(rotation: ViewRotation): boolean {
  return rotation === 90 || rotation === 270;
}

/** What to draw besides the page content. */
export interface RenderFlags {
  /** Annotation appearance streams. Default on. */
  readonly annotations: boolean;
  /** Form widgets, read-only; drawn only when `annotations` is on too. Default on. */
  readonly formWidgets: boolean;
  /** Print intent: honours annotation `Print`/`NoView` flags. Default off. */
  readonly print: boolean;
  /** Grayscale content (the background keeps its colour). Default off. */
  readonly grayscale: boolean;
}

/** The render flags when none are given: annotations and form widgets drawn, screen colours. */
export const DEFAULT_FLAGS: RenderFlags = Object.freeze({
  annotations: true,
  formWidgets: true,
  print: false,
  grayscale: false,
});

/** A rectangle in the whole page's output pixels (after scale and view rotation), y down. */
export interface PixelRect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** A straight-alpha sRGB colour, each channel 0–255. */
export interface Rgba8 {
  readonly r: number;
  readonly g: number;
  readonly b: number;
  readonly a: number;
}

/** Opaque white, the default background. */
export const WHITE: Rgba8 = Object.freeze({ r: 255, g: 255, b: 255, a: 255 });

/** One render: a page, a scale, an optional tile, a view rotation, flags and a background. */
export interface RenderRequest {
  /** 0-based (ADR 0027). */
  readonly page: number;
  /** Output pixels per point, device pixel ratio applied (1 is 72 DPI). */
  readonly scale: number;
  /** The tile; omitted is the whole page (still bounded by the limits). */
  readonly clip?: PixelRect;
  readonly rotation?: ViewRotation;
  readonly flags?: Partial<RenderFlags>;
  /** Omitted is white; `null` is transparent. */
  readonly background?: Rgba8 | null;
}

/** A page as a backend reports it: displayed size in points (`/Rotate` applied) and `/Rotate`. */
export interface RenderPage {
  readonly width: number;
  readonly height: number;
  readonly rotate: number;
}

/** The geometry of one validated request. */
export interface TilePlan {
  /** The whole page's pixel size at the scale, after the view rotation. */
  readonly pageWidth: number;
  readonly pageHeight: number;
  readonly tile: PixelRect;
  /** `tile.width × tile.height × 4`. */
  readonly byteLength: number;
}

/** A rendered tile: straight-alpha RGBA8, ready for `new ImageData(pixels, width, height)`. */
export interface RenderedTile {
  readonly width: number;
  readonly height: number;
  readonly pixels: Uint8ClampedArray<ArrayBuffer>;
}

/** Options for opening a document in a backend. */
export interface RenderOpenOptions {
  /** Used by `open` only; the backend keeps no copy (ADR 0004 §1). */
  readonly password?: string;
  readonly limits?: Partial<RenderLimits>;
}

/** One open document. One call at a time (renders are synchronous). */
export interface RenderDocument extends Disposable {
  readonly pageCount: number;
  /** @throws RenderError `page-index-out-of-range`, `data-pending`, `render-failed`. */
  page(index: number): RenderPage;
  /** Validates `request` for this document. */
  plan(request: RenderRequest): TilePlan;
  /**
   * Renders `request`. `signal` is checked before and after the engine call: an aborted
   * request throws a `DOMException` named `AbortError`, and nothing it allocated survives.
   */
  render(request: RenderRequest, options?: { readonly signal?: AbortSignal }): RenderedTile;
  /** Frees the engine's document. Idempotent. */
  close(): void;
}

/** A rendering engine. */
export interface RenderBackend {
  readonly name: "pdfium" | "hayro";
  readonly version: string;
  /** The size of the engine's WebAssembly memory, in bytes (it only grows). */
  memory(): number;
  /** @throws RenderError `password-required`, `wrong-password`, `data-pending`, `render-failed`. */
  open(source: RenderSource, options?: RenderOpenOptions): RenderDocument;
}

/**
 * The codes a render backend throws (ADR 0026). They are defined in `fixtures/codes.json`
 * (`renderErrors` and the groups whose codes it reuses); test/codes.test.ts checks this union
 * and its kinds against that file, so a drift fails the tests.
 */
export type RenderErrorCode =
  | "render-too-large"
  | "render-failed"
  | "render-backend-unavailable"
  | "invalid-argument"
  | "page-index-out-of-range"
  | "password-required"
  | "wrong-password"
  | "data-pending"
  | "source"
  | "cancelled"
  | "disposed";

/** What kind of failure a render code is (the ADR 0026 kinds its codes use). */
export type RenderErrorKind = "limit" | "input" | "io" | "usage" | "security" | "state" | "cancelled";

/** A byte range of the input: its first byte and its length. */
export type ByteRange = readonly [offset: number, length: number];

const KINDS: Record<RenderErrorCode, RenderErrorKind> = {
  "render-too-large": "limit",
  "render-failed": "input",
  "render-backend-unavailable": "io",
  "invalid-argument": "usage",
  "page-index-out-of-range": "usage",
  "password-required": "security",
  "wrong-password": "security",
  "data-pending": "state",
  source: "io",
  cancelled: "cancelled",
  disposed: "state",
};

/** A render backend's error, in the ADR 0026 contract. Messages carry no document text. */
export class RenderError extends Error {
  override readonly name = "RenderError";
  readonly code: RenderErrorCode;
  readonly kind: RenderErrorKind;
  /** For `data-pending`: the ranges to fetch before calling again. */
  readonly ranges: readonly ByteRange[] | undefined;

  constructor(code: RenderErrorCode, message: string, ranges?: readonly ByteRange[]) {
    super(message);
    this.code = code;
    this.kind = KINDS[code];
    this.ranges = ranges;
  }

  /**
   * The ADR 0026 §1 object: what crosses a worker boundary (structured clone keeps neither a
   * custom error name nor own properties). `@laminapdf/sdk`'s worker posts it, and the page gets
   * a `LaminaPdfError` with the same code, message and `ranges`. A `data-pending` lists its ranges
   * as `[offset, length]` pairs in `data.ranges`, as the Rust core does. It carries no hint.
   */
  toDiagnostic(): RenderDiagnostic {
    return {
      code: this.code,
      kind: this.kind,
      message: this.message,
      ...(this.ranges === undefined ? {} : { data: { ranges: this.ranges } }),
      docsUrl: `${DOCS_URL.base}${this.code}${DOCS_URL.suffix}`,
    };
  }
}

/**
 * Where each code's page is (ADR 0026 §6), as `fixtures/codes.json`'s `docsUrl` says;
 * test/codes.test.ts checks that `toDiagnostic()`'s URLs agree. Hand-copied, like KINDS, until
 * this package's code tables are generated.
 */
const DOCS_URL = { base: "https://github.com/laudebugs/laminapdf/blob/main/docs/errors/", suffix: ".md" } as const;

/** A RenderError as the ADR 0026 §1 object (see {@link RenderError.toDiagnostic}). */
export interface RenderDiagnostic {
  readonly code: RenderErrorCode;
  readonly kind: RenderErrorKind;
  readonly message: string;
  readonly data?: { readonly ranges: readonly ByteRange[] };
  readonly docsUrl: string;
}

/** Whether `e` is a RenderError (with `code`, if given). */
export function isRenderError<C extends RenderErrorCode>(e: unknown, code?: C): e is RenderError & { code: C } {
  // Structural, not instanceof: module copies give one class per copy (ADR 0009 §3).
  if (typeof e !== "object" || e === null || (e as { name?: unknown }).name !== "RenderError") return false;
  const actual = (e as { code?: unknown }).code;
  return typeof actual === "string" && (code === undefined || actual === code);
}

/** Whether `code` is one of the codes a render backend throws. */
export function isRenderErrorCode(code: unknown): code is RenderErrorCode {
  return typeof code === "string" && Object.hasOwn(KINDS, code);
}

/**
 * Merges limits over the defaults. Sides are clamped to 2^32 − 1 (a u32 in the Rust planner),
 * so both planners see the same limit.
 */
export function limitsOf(limits?: Partial<RenderLimits>): RenderLimits {
  const merged = { ...DEFAULT_LIMITS, ...limits };
  return { ...merged, maxRenderSide: Math.min(Math.max(0, Math.floor(merged.maxRenderSide)), 0xffff_ffff) };
}

/** Merges flags over the defaults. */
export function flagsOf(flags?: Partial<RenderFlags>): RenderFlags {
  return { ...DEFAULT_FLAGS, ...flags };
}

/**
 * Validates `request` against `page` and `limits` (the port of Rust's `TilePlan::new`): the
 * whole page is `max(1, floor(points × scale))` pixels per side after the view rotation, and the
 * tile must lie inside it and within the limits.
 *
 * @throws RenderError `invalid-argument` or `render-too-large`.
 */
export function planTile(page: RenderPage, request: RenderRequest, limits: RenderLimits): TilePlan {
  checkScale(request.scale);
  checkPage(page);
  checkBackground(request.background);
  const rotation = request.rotation ?? 0;
  if (![0, 90, 180, 270].includes(rotation)) {
    throw new RenderError("invalid-argument", "invalid render request: rotation must be 0, 90, 180 or 270");
  }
  const quarter = isQuarterTurn(rotation);
  const pageWidth = wholePixels(quarter ? page.height : page.width, request.scale);
  const pageHeight = wholePixels(quarter ? page.width : page.height, request.scale);
  const tile = request.clip ?? { x: 0, y: 0, width: pageWidth, height: pageHeight };
  checkTile(tile, pageWidth, pageHeight);
  checkTileLimits(tile, limits);
  return { pageWidth, pageHeight, tile, byteLength: tile.width * tile.height * 4 };
}

function checkScale(scale: number): void {
  // The Rust planner sees the scale as an f32: check that value, so both agree.
  if (!Number.isFinite(Math.fround(scale)) || Math.fround(scale) <= 0) {
    throw new RenderError("invalid-argument", "invalid render request: scale must be finite and positive");
  }
}

function checkPage(page: RenderPage): void {
  if (!(page.width > 0 && page.height > 0 && Number.isFinite(page.width) && Number.isFinite(page.height))) {
    throw new RenderError("invalid-argument", "invalid render request: the page has no drawable area");
  }
}

function checkBackground(background: Rgba8 | null | undefined): void {
  if (background === undefined || background === null) return;
  const channels = [background.r, background.g, background.b, background.a];
  if (!channels.every((c) => Number.isInteger(c) && c >= 0 && c <= 255)) {
    throw new RenderError("invalid-argument", "invalid render request: colour channels must be integers 0–255");
  }
}

/** One side of the whole page in pixels: `max(1, floor(points × scale))`. */
function wholePixels(points: number, scale: number): number {
  // f32 like the Rust planner, so both agree on the floor at every scale.
  const v = Math.max(1, Math.floor(Math.fround(points) * Math.fround(scale)));
  if (v > 2_147_483_647) {
    throw new RenderError("invalid-argument", "invalid render request: the page is too large to address at this scale");
  }
  return v;
}

/** The tile is whole pixels, not empty, and inside the whole page. */
function checkTile(tile: PixelRect, pageWidth: number, pageHeight: number): void {
  const ints = [tile.x, tile.y, tile.width, tile.height];
  if (!ints.every((v) => Number.isInteger(v) && v >= 0)) {
    throw new RenderError("invalid-argument", "invalid render request: the tile must be whole pixels");
  }
  if (tile.width === 0 || tile.height === 0) {
    throw new RenderError("invalid-argument", "invalid render request: the tile is empty");
  }
  if (tile.x + tile.width > pageWidth || tile.y + tile.height > pageHeight) {
    throw new RenderError("invalid-argument", "invalid render request: the tile extends past the page");
  }
}

function checkTileLimits(tile: PixelRect, limits: RenderLimits): void {
  if (tile.width > limits.maxRenderSide || tile.height > limits.maxRenderSide) {
    throw new RenderError(
      "render-too-large",
      `a ${tile.width}×${tile.height} pixel tile exceeds the render limit of ${limits.maxRenderSide} pixels per side`,
    );
  }
  if (tile.width * tile.height > limits.maxRenderPixels) {
    throw new RenderError(
      "render-too-large",
      `a ${tile.width}×${tile.height} pixel tile exceeds the render limit of ${limits.maxRenderPixels} pixels`,
    );
  }
}

/**
 * The affine matrix `[a, b, c, d, e, f]` from the displayed page space (points, top-left origin,
 * y down, `/Rotate` applied) to the tile's pixels: Rust's `TilePlan::page_to_tile`.
 */
export function pageToTile(plan: TilePlan, page: RenderPage, rotation: ViewRotation = 0): readonly number[] {
  const quarter = isQuarterTurn(rotation);
  const w0 = quarter ? plan.pageHeight : plan.pageWidth;
  const h0 = quarter ? plan.pageWidth : plan.pageHeight;
  const sx = w0 / page.width;
  const sy = h0 / page.height;
  const { x: tx, y: ty } = plan.tile;
  switch (rotation) {
    case 0:
      return [sx, 0, 0, sy, -tx, -ty];
    case 90:
      return [0, sx, -sy, 0, h0 - tx, -ty];
    case 180:
      return [-sx, 0, 0, -sy, w0 - tx, h0 - ty];
    case 270:
      return [0, -sx, sy, 0, -tx, w0 - ty];
  }
}

/** Throws the `AbortError` a cancelled render throws, if `signal` is aborted. */
export function throwIfAborted(signal: AbortSignal | undefined): void {
  if (signal?.aborted) throw new DOMException("the render was aborted", "AbortError");
}

/**
 * Where an engine's `.wasm` comes from: a URL (or URL string) to fetch, a `Response` (or a
 * promise of one), the bytes, or a compiled `WebAssembly.Module`. The app controls caching and
 * lazy loading (ADR 0009 §2).
 */
export type WasmSource =
  | URL
  | string
  | Response
  | PromiseLike<Response>
  | ArrayBuffer
  | ArrayBufferView<ArrayBuffer>
  | WebAssembly.Module;

/** The {@link WasmSource} forms that `postMessage` can send to a worker. */
export type CloneableWasmSource = string | ArrayBuffer | ArrayBufferView<ArrayBuffer> | WebAssembly.Module;

// For the adapters (@laminapdf/render-hayro, @laminapdf/render-pdfium): what both share, so they
// check pages, reads and sizes the same way. Apps don't need these.

/** The most input bytes a wasm32 engine can address: its file access and buffers take 32-bit sizes. */
export const MAX_WASM_INPUT = 0xffff_ffff;

/** Throws `render-too-large` when an input of `size` bytes is over `most`. */
export function checkInputSize(size: number, most: number): void {
  if (size > most) {
    throw new RenderError("render-too-large", `the input is ${size} bytes, over the render limit of ${most} bytes`);
  }
}

/** Throws `page-index-out-of-range` unless `index` is a page of a `pageCount`-page document. */
export function checkPageIndex(index: number, pageCount: number): void {
  if (!Number.isInteger(index) || index < 0 || index >= pageCount) {
    throw new RenderError(
      "page-index-out-of-range",
      `page index ${index} is out of range for a document of ${pageCount} pages`,
    );
  }
}

/** Throws `disposed` once the document is closed. */
export function checkOpen(open: boolean): asserts open {
  if (!open) throw new RenderError("disposed", "the document was closed");
}

/** The error of a failed read: the source's own RenderError, else `source`. */
export function readFailure(offset: number, length: number, cause?: unknown): RenderError {
  return isRenderError(cause)
    ? cause
    : new RenderError("source", `bytes ${offset}..+${length} of the input could not be read`);
}

/**
 * Reads `length` bytes at `offset` from `source`: the bytes, or `undefined` while they haven't
 * arrived.
 *
 * @throws RenderError `source` when the source throws or returns fewer bytes, or the source's own
 * RenderError.
 */
export function readRange(source: RenderSource, offset: number, length: number): Uint8Array | undefined {
  let bytes: Uint8Array | undefined;
  try {
    bytes = source.readSync(offset, length);
  } catch (e) {
    throw readFailure(offset, length, e);
  }
  if (bytes !== undefined && bytes.length !== length) throw readFailure(offset, length);
  return bytes;
}

/** `ranges` sorted, with overlapping and touching ones merged (an engine may ask for one window many times). */
export function mergeRanges(ranges: readonly ByteRange[]): [offset: number, length: number][] {
  const merged: [number, number][] = [];
  for (const [offset, length] of [...ranges].sort((a, b) => a[0] - b[0] || a[1] - b[1])) {
    const last = merged.at(-1);
    if (last !== undefined && offset <= last[0] + last[1]) {
      last[1] = Math.max(last[1], offset + length - last[0]);
    } else {
      merged.push([offset, length]);
    }
  }
  return merged;
}

/** `data-pending` with the missing `ranges`, merged. */
export function pendingError(ranges: readonly ByteRange[]): RenderError {
  const merged = mergeRanges(ranges);
  return new RenderError("data-pending", `${merged.length} byte range(s) of the input are not available yet`, merged);
}
