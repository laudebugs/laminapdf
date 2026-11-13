---
title: "Render pages to images"
description: "Draw a PDF's pages as pixels, for thumbnails, previews or tests, in JavaScript and from the command line."
version: "0.2.0"
url: "https://docs.laminapdf.com/guides/render-pages/"
---

# Render pages to images

Draw a PDF's pages as pixels, for thumbnails, previews or tests, in JavaScript and from the command line.

You'll draw a page as an image: a thumbnail for a file list, a preview in an email, or a picture to compare in a test. A renderer turns the page into straight RGBA pixels, and Lamina PDF reads the same page's text, so you can show both.

## In JavaScript

Two renderers implement the same interface, `RenderBackend` from `@laminapdf/render`:

| Package | Renderer | Best for |
| --- | --- | --- |
| `@laminapdf/render-hayro` | hayro, written in Rust and built to WebAssembly | servers, Cloudflare Workers and tests: small, and it needs no address |
| `@laminapdf/render-pdfium` | PDFium, the engine of Chrome's PDF viewer | browsers, where pages should look as they do in Chrome |

This example draws a page with hayro and reads its text with Lamina PDF:

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

To draw with PDFium instead, start it with `initPdfium(wasm)` from `@laminapdf/render-pdfium`, which takes PDFium's WebAssembly as `initHayro` takes hayro's. Then pass the backend it returns to `drawPage`; nothing else changes.

A scale of 2 draws 144 pixels per inch. Ask for a `clip` to draw only part of a page. The renderer refuses a tile wider or taller than 16,384 pixels, or larger than about 16.7 million pixels, with `render-too-large`, so draw a poster-sized page at a high scale in tiles. [How a page is drawn](../../pages/concepts/drawing-pages.md) explains the tile plan.

## From the command line

`laminapdf render` writes pages as PNG files, which suits scripts and batch jobs:

```console
$ mkdir -p thumbs
$ laminapdf render report.pdf --pages 1-3 --dpi 72 -o 'thumbs/{stem}-{n:03}.png'
page 1: 419 × 595 px → thumbs/report-001.png
page 2: 419 × 595 px → thumbs/report-002.png
page 3: 419 × 595 px → thumbs/report-003.png
```

[Extract text and render pages](../../pages/cli/extract-and-render.md#images-render) lists its options.

## In Rust

The Rust renderers are the `laminapdf-render` crate, with hayro behind its `hayro` feature, and `laminapdf-render-pdfium` for a native PDFium. Their API is at [docs.rs](https://docs.rs/laminapdf-render). The `laminapdf` crate doesn't draw pages yet, so this guide has no Rust example; the command line above runs the same renderers.

Next, see [how to compare pages in tests](../../pages/guides/test-generated-pdfs.md), which draws two PDFs and counts the pixels that differ.
