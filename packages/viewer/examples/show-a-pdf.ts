// Shows a PDF in a web page. The viewer keeps only the pages near the scroll position in the page
// and draws them with PDFium, in a Web Worker so the page stays responsive. Pass the address your
// bundler serves PDFium's WebAssembly at; with Vite:
//
//   import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
//
// The viewer is a preview: its API can still change between releases.

import { createViewer, ViewerClient } from "@laminapdf/viewer";

/** A PDF on screen; dispose of it to take it down and end its Web Worker. */
export interface ShownPdf extends Disposable {
  readonly pageCount: number;
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
    // The Web Worker reads the file in slices; the page never holds its bytes.
    const opened = await client.open(file);
    viewer.open(opened);
    return { pageCount: opened.pages.length, [Symbol.dispose]: close };
  } catch (error) {
    // A file that won't open, a damaged one or a locked one, still ends the Web Worker.
    close();
    throw error;
  }
}
