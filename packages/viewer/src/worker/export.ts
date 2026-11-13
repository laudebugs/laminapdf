// Page export in the document worker: a page drawn at the resolution asked for and encoded as
// PNG or JPEG by the canvas, with the resolution written into the file (PNG `pHYs`, JFIF
// density) as the Rust export writes it, so the image prints at the page's size. One page per
// call: the pixels of the page before are released before the next is drawn.

import type { RenderRequest } from "@laminapdf/render";
import type { ExportedPage, ExportPageOptions } from "../protocol.js";
import { current, filling } from "./document.js";

/** The most pixels one exported page holds: about 64 million, a 600 dpi page of 11 × 17 inches. */
const MAX_EXPORT_PIXELS = 64_000_000;

/** JPEG quality when none is given, on the 1–100 scale. */
const DEFAULT_QUALITY = 90;

/**
 * Page `index` drawn at `options.dpi` (lowered when the page would pass MAX_EXPORT_PIXELS; `dpi`
 * says what it was drawn at), on white, and encoded as `options.format`.
 */
export async function exportPage(
  index: number,
  options: ExportPageOptions,
  signal: AbortSignal,
): Promise<ExportedPage> {
  if (!(options.dpi > 0 && Number.isFinite(options.dpi))) throw new RangeError("dpi must be a positive number");
  const doc = current();
  const page = doc.render.page(index);
  const area = page.width * page.height;
  const dpi = Math.min(options.dpi, 72 * Math.sqrt(MAX_EXPORT_PIXELS / Math.max(1, area)));
  const request: RenderRequest = { page: index, scale: scaleFor(dpi) };
  const tile = await filling(doc.remote, () => doc.render.render(request, { signal }), signal);
  signal.throwIfAborted();
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

/** `png` with a `pHYs` chunk after its `IHDR` giving `dpi` (as pixels per metre). */
export function withPngDensity(png: Uint8Array, dpi: number): Uint8Array<ArrayBuffer> {
  // Signature (8) and IHDR (length 4, type 4, data 13, CRC 4).
  const at = 8 + 25;
  const perMetre = Math.round(dpi / 0.0254);
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
  // SOI, then a 16-byte APP0: "JFIF\0", version 1.1, dots per inch, the density, no thumbnail.
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
