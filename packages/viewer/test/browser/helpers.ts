// Fixtures and clients for the viewer's browser tests (C3a).
import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import {
  type BackendName,
  ViewerClient,
  type ViewerEvent,
  type ViewerEvents,
  type ViewerOptions,
} from "@lamina/viewer";

const urls = import.meta.glob<string>(
  [
    "../../../../fixtures/corpus/*.pdf",
    "../../../../fixtures/text/encrypted/*.pdf",
    "../../../../fixtures/corpus/expected.json",
  ],
  { query: "?url", import: "default", eager: true },
);

/** The repository-relative paths of the corpus PDFs. */
export const CORPUS = Object.keys(urls)
  .filter((k) => k.includes("/corpus/") && k.endsWith(".pdf"))
  .map((k) => k.replace("../../../../", ""));

/** An AES-256 file whose user password is `user` (fixtures/README.md). */
export const ENCRYPTED = "fixtures/text/encrypted/text-shaped-aes-256.pdf";
export const PASSWORD = "user";

export async function read(path: string): Promise<Uint8Array> {
  const url = urls[`../../../../${path}`];
  if (url === undefined) throw new Error(`${path} is not served`);
  return new Uint8Array(await (await fetch(url)).arrayBuffer());
}

export async function expected(): Promise<Record<string, { page_count: number; first_page_mediabox: number[] }>> {
  return JSON.parse(new TextDecoder().decode(await read("fixtures/corpus/expected.json")));
}

export function viewer(backend: BackendName = "pdfium", options: Partial<ViewerOptions> = {}): Promise<ViewerClient> {
  return ViewerClient.create({
    backend: backend === "pdfium" ? { backend, wasm: pdfiumWasm } : { backend },
    ...options,
  });
}

export async function rejection(p: Promise<unknown>): Promise<unknown> {
  try {
    await p;
  } catch (e) {
    return e;
  }
  throw new Error("expected a rejection");
}

/** Resolves on the client's next `event`. */
export function next<E extends ViewerEvent>(client: ViewerClient, event: E): Promise<ViewerEvents[E]> {
  return new Promise((resolve) => {
    const off = client.on(event, (data) => {
      off();
      resolve(data);
    });
  });
}
