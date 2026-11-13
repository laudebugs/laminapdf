// Page export in the document worker: a page drawn at the resolution asked for and encoded as
// PNG or JPEG by the canvas, with the resolution written into the file (PNG `pHYs`, JFIF
// density) as the Rust export writes it, so the image prints at the page's size. One page per
// call: the pixels of the page before are released before the next is drawn.

import { DEFAULT_LIMITS, type RenderedTile, type RenderRequest } from "@laminapdf/render";
import type { ExportedPage, ExportPageOptions } from "../protocol.js";
import { current, filling } from "./document.js";

/**
 * The most pixels one exported page holds, and the longest side: the renderers' own limits
 * (about 16.8 million pixels, Letter at about 424 dpi; 16,384 pixels a side), since a page is
 * drawn as one tile.
 */
const MAX_EXPORT_PIXELS = DEFAULT_LIMITS.maxRenderPixels;
const MAX_EXPORT_SIDE = DEFAULT_LIMITS.maxRenderSide;

/** JPEG quality when none is given, on the 1–100 scale. */
const DEFAULT_QUALITY = 90;

/**
 * Page `index` drawn at `options.dpi` (lowered when the page would pass the renderers' limits;
 * `dpi` says what it was drawn at), on white, and encoded as `options.format`.
 */
export async function exportPage(
  index: number,
  options: ExportPageOptions,
  signal: AbortSignal,
): Promise<ExportedPage> {
  const { tile, dpi } = await drawForExport(index, options.dpi, signal);
  const canvas = new OffscreenCanvas(tile.width, tile.height);
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("no 2D context for exporting");
  context.putImageData(new ImageData(tile.pixels, tile.width, tile.height), 0, 0);
  const jpeg = options.format === "jpeg";
  const quality = Math.min(100, Math.max(1, Math.round(options.quality ?? DEFAULT_QUALITY)));
  const blob = await canvas.convertToBlob(
    jpeg ? { type: "image/jpeg", quality: quality / 100 } : { type: "image/png" },
  );
  signal.throwIfAborted();
  const bytes = new Uint8Array(await blob.arrayBuffer());
  const image = jpeg ? withJpegDensity(bytes, dpi) : withPngDensity(bytes, dpi);
  return { image, width: tile.width, height: tile.height, dpi };
}

/**
 * The resolution page `index` is exported at when `wanted` is asked for: see {@link fittedDpi}.
 */
export function exportDpi(index: number, wanted: number): number {
  if (!(wanted > 0 && Number.isFinite(wanted))) throw new RangeError("dpi must be a positive number");
  const page = current().render.page(index);
  return fittedDpi(page.width, page.height, wanted);
}

/**
 * `wanted`, lowered when a `width` × `height` point page drawn at it would pass the renderers'
 * pixel or side limit. The renderers size a page as each side times {@link scaleFor}'s scale,
 * rounded down, and that scale can round up: the result is checked as they will draw it, and
 * lowered a hair at a time until it fits.
 */
export function fittedDpi(width: number, height: number, wanted: number): number {
  const area = Math.max(1, width * height);
  const side = Math.max(1, width, height);
  let dpi = Math.min(wanted, 72 * Math.sqrt(MAX_EXPORT_PIXELS / area), (72 * MAX_EXPORT_SIDE) / side);
  for (let tries = 0; tries < 64 && !fits(width, height, dpi); tries++) dpi *= 1 - 1e-6;
  return dpi;
}

/** Whether a `width` × `height` point page drawn at `dpi` stays within the renderers' limits. */
function fits(width: number, height: number, dpi: number): boolean {
  const scale = scaleFor(dpi);
  const w = Math.floor(width * scale);
  const h = Math.floor(height * scale);
  return w * h <= MAX_EXPORT_PIXELS && w <= MAX_EXPORT_SIDE && h <= MAX_EXPORT_SIDE;
}

/** Page `index` drawn on white at {@link exportDpi}`(index, wanted)`, and that resolution. */
export async function drawForExport(
  index: number,
  wanted: number,
  signal: AbortSignal,
): Promise<{ tile: RenderedTile; dpi: number }> {
  const dpi = exportDpi(index, wanted);
  const doc = current();
  const request: RenderRequest = { page: index, scale: scaleFor(dpi) };
  const tile = await filling(doc.remote, () => doc.render.render(request, { signal }), signal);
  signal.throwIfAborted();
  return { tile, dpi };
}

/**
 * Pixels per point for `dpi`, as the renderers take it: the nearest 32-bit float, nudged up when
 * it lands below `dpi / 72`. A render's size is the page's size times the scale, rounded down,
 * so a scale a hair low would cost a page that is a whole number of pixels its last column
 * (Letter at 150 dpi: 1,274, not 1,275).
 */
export function scaleFor(dpi: number): number {
  const exact = dpi / 72;
  const rounded = Math.fround(exact);
  if (rounded >= exact) return rounded;
  const bits = new Uint32Array(new Float32Array([rounded]).buffer);
  bits[0] = (bits[0] ?? 0) + 1;
  return new Float32Array(bits.buffer)[0] ?? exact;
}

/**
 * `png` with its resolution `dpi` (as pixels per metre) in a `pHYs` chunk: the encoder's own one
 * rewritten, or a new one after the `IHDR`.
 */
export function withPngDensity(png: Uint8Array, dpi: number): Uint8Array<ArrayBuffer> {
  const perMetre = Math.round(dpi / 0.0254);
  const own = chunkAt(png, "pHYs");
  if (own !== undefined) {
    const out = png.slice();
    const view = new DataView(out.buffer);
    view.setUint32(own + 8, perMetre);
    view.setUint32(own + 12, perMetre);
    out[own + 16] = 1;
    view.setUint32(own + 17, crc32(out.subarray(own + 4, own + 17)));
    return out;
  }
  // Signature (8) and IHDR (length 4, type 4, data 13, CRC 4).
  const at = 8 + 25;
  const chunk = new Uint8Array(21);
  const view = new DataView(chunk.buffer);
  view.setUint32(0, 9);
  chunk.set([0x70, 0x48, 0x59, 0x73], 4); // "pHYs"
  view.setUint32(8, perMetre);
  view.setUint32(12, perMetre);
  chunk[16] = 1; // unit: the metre
  view.setUint32(17, crc32(chunk.subarray(4, 17)));
  const out = new Uint8Array(png.length + chunk.length);
  out.set(png.subarray(0, at));
  out.set(chunk, at);
  out.set(png.subarray(at), at + chunk.length);
  return out;
}

/** `jpeg` with its JFIF density set to `dpi`: the canvas's own APP0 segment, or a new one. */
export function withJpegDensity(jpeg: Uint8Array, dpi: number): Uint8Array<ArrayBuffer> {
  const density = Math.min(0xffff, Math.max(1, Math.round(dpi)));
  const isJfif = jpeg[2] === 0xff && jpeg[3] === 0xe0 && String.fromCharCode(...jpeg.subarray(6, 11)) === "JFIF\0";
  if (isJfif) {
    const out = jpeg.slice();
    const view = new DataView(out.buffer);
    out[13] = 1; // units: dots per inch
    view.setUint16(14, density);
    view.setUint16(16, density);
    return out;
  }
  // SOI, then an APP0 (18 bytes with its marker; its length field counts 16): "JFIF\0", version 1.1, dots per inch, the density, no thumbnail.
  const app0 = new Uint8Array([0xff, 0xe0, 0, 16, 0x4a, 0x46, 0x49, 0x46, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0]);
  const view = new DataView(app0.buffer);
  // In the segment: marker 0–1, length 2–3, "JFIF\0" 4–8, version 9–10, units 11, density 12–15.
  view.setUint16(12, density);
  view.setUint16(14, density);
  const out = new Uint8Array(jpeg.length + app0.length);
  out.set(jpeg.subarray(0, 2));
  out.set(app0, 2);
  out.set(jpeg.subarray(2), 2 + app0.length);
  return out;
}

/** Where the first chunk of `type` starts in `png` (its length field), before the image data. */
function chunkAt(png: Uint8Array, type: string): number | undefined {
  const view = new DataView(png.buffer, png.byteOffset, png.byteLength);
  for (let at = 8; at + 8 <= png.length; ) {
    const name = String.fromCharCode(...png.subarray(at + 4, at + 8));
    if (name === type) return at;
    if (name === "IDAT" || name === "IEND") return undefined;
    at += 12 + view.getUint32(at);
  }
  return undefined;
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();

/** The CRC-32 a PNG chunk carries, over its type and data. */
function crc32(bytes: Uint8Array): number {
  let c = 0xffffffff;
  for (const byte of bytes) c = (CRC_TABLE[(c ^ byte) & 0xff] ?? 0) ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
