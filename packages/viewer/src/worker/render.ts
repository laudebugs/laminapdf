// Rendering in the document worker (ADR 0009 §4): the backend draws straight-alpha RGBA, the
// worker makes an ImageBitmap of it and transfers it, so the page never copies pixels. A
// coalesced render is drawn once; each caller gets its own bitmap of the same pixels.

import type { RenderedTile, RenderRequest } from "@laminapdf/render";
import { emit } from "@laminapdf/sdk/worker/server";
import { type RenderedBitmap, renderKey } from "../protocol.js";
import { current } from "./document.js";

/** Renders `request` on document `id` (a render queued for a document since replaced is `disposed`). */
export function render(request: RenderRequest, id: number, signal: AbortSignal): RenderedTile {
  const doc = current(id);
  // The client starts its timers now (not while the render waits), and a performance test can
  // see what the worker is busy with.
  emit("render-start", { page: request.page, scale: request.scale, key: renderKey(request, id) });
  return doc.render.render(request, { signal });
}

/** One caller's bitmap of `tile`. */
export async function reply(tile: RenderedTile): Promise<[RenderedBitmap, Transferable[]]> {
  const { width, height, pixels } = tile;
  // Each createImageBitmap takes its own snapshot of the pixels: no copy is needed per caller.
  const bitmap = await createImageBitmap(new ImageData(pixels, width, height));
  return [{ bitmap, width, height }, [bitmap]];
}
