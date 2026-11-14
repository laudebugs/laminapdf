---
title: "Show a PDF in a web page"
description: "Put a scrolling, zoomable PDF viewer in your page with @laminapdf/viewer, drawn by PDFium in a Web Worker."
version: "0.3.0"
url: "https://docs.laminapdf.com/guides/show-a-pdf/"
---

# Show a PDF in a web page

Put a scrolling, zoomable PDF viewer in your page with @laminapdf/viewer, drawn by PDFium in a Web Worker.

You'll put a PDF on your page that people can scroll and zoom, with text they can select and links within the document they can follow. The viewer draws only the pages near the scroll position, so a 1,000-page file scrolls as smoothly as a short one.

> **Preview:** `@laminapdf/viewer` is a preview. It is the core of [the Lamina PDF reader](https://app.laminapdf.com), and its API can still change between releases while the reader grows.

Install the viewer and the PDFium build it draws with:

```sh
npm install @laminapdf/viewer @embedpdf/pdfium@2.15.1
```

This example shows a file the user picked, in a container on your page:

`packages/viewer/examples/show-a-pdf.ts`:

```ts
// Shows a PDF in a web page. The viewer keeps only the PDF's pages near the scroll position on
// screen and draws them with PDFium, in a Web Worker so the web page stays responsive. Pass the
// address your bundler serves PDFium's WebAssembly at; with Vite:
//
//   import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
//
// The viewer is a preview: its API can still change between releases.

import { createViewer, ViewerClient } from "@laminapdf/viewer";

// Some Safari releases have no Symbol.dispose. The code TypeScript and esbuild compile from `using`
// then looks for Symbol.for("Symbol.dispose"), so the shown PDF answers to that key there.
const dispose: typeof Symbol.dispose = (Symbol.dispose ?? Symbol.for("Symbol.dispose")) as typeof Symbol.dispose;

/** A PDF on screen. Close it, or declare it with `using`, to take it down and end its Web Worker. */
export interface ShownPdf extends Disposable {
  readonly pageCount: number;
  /** Takes the PDF down and ends its Web Worker; closing it again does nothing. */
  close(): void;
}

/** Shows `file` in `container`, which must have a height, drawn by PDFium from `pdfiumWasm`. */
export async function showPdf(container: HTMLElement, file: Blob, pdfiumWasm: string): Promise<ShownPdf> {
  // The client starts the document's Web Worker and loads PDFium into it.
  const client = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
  const viewer = createViewer({ client });
  const close = () => {
    viewer.destroy();
    client.terminate();
  };
  try {
    viewer.mount(container);
    // The file goes to the Web Worker by reference; the web page never reads its bytes.
    const opened = await client.open(file);
    viewer.open(opened);
    return { pageCount: opened.pages.length, close, [dispose]: close };
  } catch (error) {
    // A file that won't open, a damaged one or a locked one, still ends the Web Worker.
    close();
    throw error;
  }
}
```

Call it with an element that has a height, the file from an `<input type="file">` or a drop, and the address your bundler serves PDFium's WebAssembly at:

```ts
using shown = await showPdf(container, file, pdfiumWasm);
```

`using` takes the PDF down when the block it's declared in ends. When the viewer should stay on the page longer, keep what `showPdf` returns and, as [Take it down](#take-it-down) explains, call its `close()` when you're done.

## What runs where

The viewer splits the work between your page and a Web Worker, one per document:

- **The Web Worker** opens the file with Lamina PDF and draws pages with PDFium. A slow PDF page never blocks scrolling or typing on your site.
- **Your web page** holds the view: a shell for each page near the scroll position, the canvases they paint into, and the text and link layers over them.

The file stays in the Web Worker. A `File` goes to the Web Worker by reference, so your web page never reads its bytes. To show a file from a server instead, pass `client.open` the file's address as `{ url }` where the example passes `file`. The Web Worker then reads it with HTTP range requests, fetching only the parts the visible pages need.

## Give the container a size

The viewer fills its container and scrolls inside it, so the container needs a height: a fixed one, or `flex: 1` in a column that fills the window. A container with no height shows nothing.

## Take it down

When the PDF leaves the page, call `close()` on what `showPdf` returns, or let a `using` declaration close it at the end of its block. Closing stops the PDF's renders, frees its canvases and ends its Web Worker; a second `close()` does nothing. If the file won't open, because it's damaged or locked with a password, `showPdf` ends its Web Worker before it throws.

`using` works in Safari too. The code TypeScript and esbuild compile from `using` looks for `Symbol.dispose`, or for `Symbol.for("Symbol.dispose")` where the browser has none, and `showPdf` puts `close()` under the key that code looks for.

## Next

[How a page is drawn](../../pages/concepts/drawing-pages.md) explains why a renderer draws the pixels while Lamina PDF reads the text. To draw pages to images instead of a scrolling view, see [render pages to images](../../pages/guides/render-pages.md).
