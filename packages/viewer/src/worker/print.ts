// Printing in the document worker (R09): a page drawn at print resolution with the print
// intent (annotations as their print flags say), encoded as PNG here, so the page that prints
// it only ever holds one small file per page, never its pixels. One page per call, so the
// pixels of the page before are released before the next is drawn.

import type { RenderRequest } from "@laminapdf/render";
import type { PrintedPage, PrintOptions } from "../protocol.js";
import { readAll } from "../source.js";
import { current, filling } from "./document.js";

/** The most pixels one printed page holds: about 16 million, a 300 dpi page of 11 × 17 inches. */
const MAX_PRINT_PIXELS = 16_000_000;

/**
 * Page `index` drawn for printing at `options.dpi` (lowered when the page would pass
 * MAX_PRINT_PIXELS; `dpi` says what it was drawn at), as a PNG.
 */
export async function printPage(index: number, options: PrintOptions, signal: AbortSignal): Promise<PrintedPage> {
  const doc = current();
  const page = doc.render.page(index);
  const wanted = options.dpi / 72;
  const area = page.width * page.height;
  const scale = Math.min(wanted, Math.sqrt(MAX_PRINT_PIXELS / Math.max(1, area)));
  const request: RenderRequest = {
    page: index,
    scale,
    flags: { annotations: options.annotations, formWidgets: options.annotations, print: true, grayscale: false },
  };
  const tile = await filling(doc.remote, () => doc.render.render(request, { signal }), signal);
  signal.throwIfAborted();
  const canvas = new OffscreenCanvas(tile.width, tile.height);
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("no 2D context for printing");
  context.putImageData(new ImageData(tile.pixels, tile.width, tile.height), 0, 0);
  const image = await canvas.convertToBlob({ type: "image/png" });
  return { image, width: page.width, height: page.height, dpi: Math.round(scale * 72) };
}

/**
 * Every byte of the open file as it was opened: a remote file's from its block cache and its
 * host, unchanged. A local file's bytes stay where the page has them.
 */
export function fileBytes(signal: AbortSignal): Promise<Uint8Array> {
  const { remote } = current();
  if (remote === undefined) return Promise.reject(new Error("the open file isn't remote: read it where it was opened"));
  return remote.whole === undefined ? readAll(remote, signal) : Promise.resolve(remote.whole);
}

/** One caller's answer: its own copy of the bytes, transferred. */
export function replyBytes(bytes: Uint8Array): [Uint8Array, Transferable[]] {
  const copy = bytes.slice();
  return [copy, [copy.buffer]];
}
