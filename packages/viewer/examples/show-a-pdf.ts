// Shows a PDF in a web page: the viewer scrolls and zooms it, and a Web Worker opens and draws it
// with PDFium, away from the page's main thread. A bundler such as Vite gives PDFium's
// WebAssembly file a URL through the `?url` import.

import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { isLaminaPdfError } from "@laminapdf/sdk";
import { createViewer, type Viewer, ViewerClient } from "@laminapdf/viewer";

/** A viewer on the page and the Web Worker that draws for it. */
export interface PdfView extends Disposable {
  readonly client: ViewerClient;
  readonly viewer: Viewer;
}

/** Asks the reader for the file's password, saying whether the last one was wrong; `undefined` when they cancel. */
export type AskPassword = (wrong: boolean) => Promise<string | undefined>;

/** Starts a viewer in `container`, which gives it its size. Start it before the reader picks a file. */
export async function startViewer(container: HTMLElement): Promise<PdfView> {
  // The Web Worker starts and PDFium downloads now, so the first page draws sooner.
  const client = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
  const viewer = createViewer({ client });
  viewer.mount(container);
  return {
    client,
    viewer,
    [Symbol.dispose]: () => {
      viewer.destroy();
      client.terminate();
    },
  };
}

/**
 * Shows `file` in `view`. A file with a password asks for it, again after a wrong one, until it
 * opens; it resolves `false` when the reader cancels, and the shown file stays.
 */
export async function showFile(view: PdfView, file: Blob, askPassword: AskPassword): Promise<boolean> {
  let password: string | undefined;
  for (;;) {
    try {
      view.viewer.open(await view.client.open(file, password === undefined ? {} : { password }));
      return true;
    } catch (e) {
      const wrong = isLaminaPdfError(e, "wrong-password");
      if (!wrong && !isLaminaPdfError(e, "password-required")) throw e;
      password = await askPassword(wrong);
      if (password === undefined) return false;
    }
  }
}

/** Keeps `label` showing the page and the zoom, such as "Page 3 of 12 · 100 %". */
export function showPosition(viewer: Viewer, label: HTMLElement): () => void {
  return viewer.subscribe(() => {
    const { currentPage, pageCount, percent } = viewer.getState();
    label.textContent = `Page ${currentPage + 1} of ${pageCount} · ${Math.round(percent)} %`;
  });
}
