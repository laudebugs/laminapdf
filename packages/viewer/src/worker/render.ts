// Rendering in the document worker (C3a, ADR 0009 §4): the backend draws straight-alpha RGBA, the
// worker makes an ImageBitmap of it and transfers it, so the page never copies pixels. A
// coalesced render is drawn once; each caller gets its own bitmap of the same pixels.

import { emit } from "@lamina/pdf/worker/server";
import type { RenderedTile, RenderRequest } from "@lamina/render";
import type { RenderedBitmap } from "../protocol.js";
import { current } from "./document.js";

/** Test hook: every render's scale, in start order (the priority tests read it). */
export const renderLog: number[] = [];

export function render(request: RenderRequest, signal: AbortSignal): RenderedTile {
  renderLog.push(request.scale);
  // For the perf harness (C3c) and the tests: which render the worker is now busy with.
  emit("render-start", { page: request.page, scale: request.scale });
  return current().render.render(request, { signal });
}

/** One caller's bitmap of `tile`. */
export async function reply(tile: RenderedTile): Promise<[RenderedBitmap, Transferable[]]> {
  const { width, height, pixels } = tile;
  // A fresh copy per caller: ImageData takes ownership of nothing, but each bitmap is separate.
  const bitmap = await createImageBitmap(new ImageData(new Uint8ClampedArray(pixels), width, height));
  return [{ bitmap, width, height }, [bitmap]];
}
