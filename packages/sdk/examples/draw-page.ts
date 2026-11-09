// Draws one page to pixels and reads the same page's text, to show who does which job: a renderer
// turns the page into RGBA pixels, and Lamina PDF reads the text from its own model of the file.
// Both read the same bytes. In Node, load the hayro renderer with the module's bytes:
//
//   const wasm = fileURLToPath(import.meta.resolve("@laminapdf/wasm/lamina_pdf_render_hayro.wasm"));
//   const renderer = await initHayro(readFileSync(wasm));
//
// Text needs Lamina PDF's text or extract module: `await init({ text: true })`.

import { bytesSource, type RenderBackend } from "@laminapdf/render";
import { Document } from "@laminapdf/sdk";

/** Pixels per point: 2 draws at 144 dpi, sharp on most screens. */
const SCALE = 2;

/** One page as pixels, with its text beside them. */
export interface DrawnPage {
  readonly width: number;
  readonly height: number;
  /** Straight RGBA, four bytes a pixel, row by row from the top-left corner. */
  readonly pixels: Uint8ClampedArray;
  readonly text: string;
}

/** Draws page `index` of `bytes` with `renderer` and reads its text with Lamina PDF. */
export function drawPage(renderer: RenderBackend, bytes: Uint8Array, index: number): DrawnPage {
  // The renderer draws the page and nothing else: it never supplies text, links or selection.
  using drawing = renderer.open(bytesSource(bytes));
  const { width, height, pixels } = drawing.render({ page: index, scale: SCALE });
  // The text comes from Lamina PDF, which also edits and saves the file, so what you select is what
  // an edit changes.
  using doc = Document.open(bytes);
  const text = doc.page(index).text().text();
  return { width, height, pixels, text };
}
