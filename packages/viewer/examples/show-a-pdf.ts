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
const dispose = (Symbol.dispose ?? Symbol.for("Symbol.dispose")) as typeof Symbol.dispose;

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
