// Shows a PDF in a web page: the viewer scrolls and zooms it, and a Web Worker opens and draws it
// with PDFium, away from the page's main thread. A bundler such as Vite gives PDFium's
// WebAssembly file a URL through the `?url` import.

import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import { isLaminaPdfError } from "@laminapdf/sdk";
import { createViewer, type Viewer, ViewerClient } from "@laminapdf/viewer";

/** Asks the reader for the file's password, saying whether the last one was wrong; `undefined` when they cancel. */
export type AskPassword = (wrong: boolean) => Promise<string | undefined>;

/** A viewer on the page and the Web Worker that draws for it. */
export class PdfView {
  readonly client: ViewerClient;
  readonly viewer: Viewer;
  #stopReopening = () => {};

  private constructor(client: ViewerClient, viewer: Viewer) {
    this.client = client;
    this.viewer = viewer;
  }

  /** Starts a viewer in `container`, which gives it its size. Start it before the reader picks a file. */
  static async start(container: HTMLElement): Promise<PdfView> {
    // The Web Worker starts and PDFium downloads now, so the first page draws sooner.
    const client = await ViewerClient.create({ backend: { backend: "pdfium", wasm: pdfiumWasm } });
    const viewer = createViewer({ client });
    viewer.mount(container);
    return new PdfView(client, viewer);
  }

  /**
   * Shows `file`. A file with a password asks for it, again after a wrong one, until it opens; it
   * resolves `false` when the reader cancels, and the file shown before stays.
   */
  async show(file: Blob, askPassword: AskPassword): Promise<boolean> {
    let password: string | undefined;
    for (;;) {
      try {
        this.viewer.open(await this.client.open(file, password === undefined ? {} : { password }));
        break;
      } catch (e) {
        if (!isLaminaPdfError(e, "wrong-password") && !isLaminaPdfError(e, "password-required")) throw e;
        // Any password that failed was wrong, an empty one included.
        password = await askPassword(password !== undefined);
        if (password === undefined) return false;
      }
    }
    // A Web Worker that crashed starts again, but the password died with it, so ask again.
    this.#stopReopening();
    this.#stopReopening = this.client.on("password-required", () => void this.show(file, askPassword));
    return true;
  }

  /** Takes the viewer off the page and ends its Web Worker. */
  close(): void {
    this.#stopReopening();
    this.viewer.destroy();
    this.client.terminate();
  }
}

/** Keeps `label` showing the page and the zoom, such as "Page 3 of 12 · 100 %". */
export function showPosition(viewer: Viewer, label: HTMLElement): () => void {
  const write = () => {
    const { currentPage, pageCount, percent } = viewer.getState();
    label.textContent = pageCount === 0 ? "" : `Page ${currentPage + 1} of ${pageCount} · ${Math.round(percent)} %`;
  };
  write();
  return viewer.subscribe(write);
}
