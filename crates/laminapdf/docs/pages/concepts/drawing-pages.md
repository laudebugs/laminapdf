---
title: "How a page is drawn"
description: "A renderer turns a page into pixels; Lamina PDF reads the page's text, links and selection itself, from the same bytes."
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/concepts/drawing-pages/"
---

# How a page is drawn

A renderer turns a page into pixels; Lamina PDF reads the page's text, links and selection itself, from the same bytes.

Two readers go through every page you show. A renderer, PDFium or hayro, turns the page into pixels. Lamina PDF's own model of the file gives you everything you can select, search or click: the text, the links and the selection boxes. Both read the same bytes.

*A renderer draws the pixels; Lamina PDF reads the text from the same bytes.*

Two readers use your PDF's bytes. To draw, a render request names the page, the scale, the tile to draw (its clip) and a rotation. The request becomes a tile plan, which fixes the size in pixels, and a renderer, PDFium or hayro, draws the tile as RGBA pixels. Text, links and selection never come from the renderer. Lamina PDF reads them from its own model of the same bytes, the model that also edits and saves the file.

## You ask for a tile, and get RGBA pixels back

A render request names the page, the scale in pixels per point, and optionally a clip and a rotation. The clip is the part of the page you want, as a rectangle in pixels: a viewer asks for tiles of a large page rather than the whole page at once, so it only draws what's on screen. A scale of 2 draws 144 pixels per inch, sharp on most screens.

Before it draws, the renderer turns the request into a tile plan: the whole page's size in pixels after the rotation, and the tile inside it. The renderer checks the tile against its limits first: by default, a tile wider or taller than 16,384 pixels, or larger than about 16.7 million pixels, fails with `render-too-large` instead of filling your memory. The renderer then draws the tile as straight RGBA, four bytes a pixel, row by row from the top-left corner.

This example draws a page with hayro and reads the same page's text:

`packages/sdk/examples/draw-page.ts`:

```ts
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
```

## The text never comes from the renderer

A renderer draws glyphs, but its idea of the text can differ from the file's: it may guess the order of letters or lose the characters behind a ligature. So Lamina PDF reads text, links and selection from its own model of the file, the same model that edits and saves it. What you select is what `page.text()` returns and what an edit changes.

The two readers can still disagree on where a glyph sits, since each reads the fonts its own way. Lamina PDF's tests compare each page's size and every character's position with PDFium's, and a difference becomes a bug to fix.

## PDFium in the browser, hayro on servers

PDFium, the engine behind Chrome's PDF viewer, draws the pages in the viewer. It runs in a Web Worker, away from your page, and only draws: it never changes the document. hayro is a renderer written in Rust. It's the lighter choice on a server or in a Cloudflare Worker, and it draws in the tests that compare pages. [What Lamina PDF writes and what it borrows](../../pages/concepts/dependencies.md) explains why each one is there.

Next, see [how to compare pages in tests](../../pages/guides/test-generated-pdfs.md), which draws two PDFs and counts the pixels that differ.
