// Renders one page of a PDF to an image at the width you ask for. In a web page the image is a
// canvas: the renderer gives straight RGBA pixels, which `putImageData` copies in unchanged.
// Load a renderer once and reuse it for every page:
//
//   import wasmUrl from "@embedpdf/pdfium/pdfium.wasm?url";
//   const renderer = await initPdfium(wasmUrl); // from @laminapdf/render-pdfium
//
// or `await initHayro()` from `@laminapdf/render-hayro`.

import { bytesSource, type RenderBackend } from "@laminapdf/render";

/** Draws page `index` of `bytes` onto `canvas` (one on the page, or an `OffscreenCanvas` in a Web Worker), `width` pixels wide and as tall as the page needs. */
export function renderPage(
  renderer: RenderBackend,
  bytes: Uint8Array,
  index: number,
  width: number,
  canvas: HTMLCanvasElement | OffscreenCanvas,
): void {
  using doc = renderer.open(bytesSource(bytes));
  // The page's size in points as it's displayed, its own rotation applied.
  const page = doc.page(index);
  // A page's size in pixels rounds down, so ask for half a pixel more to get `width` exactly.
  const image = doc.render({ page: index, scale: (width + 0.5) / page.width });
  canvas.width = image.width;
  canvas.height = image.height;
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("the canvas has no 2D context");
  context.putImageData(new ImageData(image.pixels, image.width, image.height), 0, 0);
}
