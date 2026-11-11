// Fixtures and clients for the viewer's browser tests (C3a).
import pdfiumWasm from "@embedpdf/pdfium/pdfium.wasm?url";
import {
  type BackendName,
  ViewerClient,
  type ViewerEvent,
  type ViewerEvents,
  type ViewerOptions,
} from "@laminapdf/viewer";

const urls = import.meta.glob<string>(
  [
    "../../../../fixtures/corpus/*.pdf",
    "../../../../fixtures/text/encrypted/*.pdf",
    "../../../../fixtures/corpus/expected.json",
    "../../../../fixtures/golden/text-*.pdf",
    "../../../../fixtures/golden/build-text-multiscript.pdf",
    "../../../../fixtures/golden/tagged-report.pdf",
    "../../../../fixtures/golden/links-and-outlines.pdf",
    "../../../../fixtures/links/links.pdf",
    "../../../../fixtures/golden/examples/hello.pdf",
    "../../../../fixtures/text/extracted.json",
    "../../../../fixtures/reading-order/*.pdf",
    "../../../../fixtures/reading-order/*.json",
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

/** Where the test server serves `path`, an address the document worker can fetch with Range requests. */
export function addressOf(path: string): string {
  const url = urls[`../../../../${path}`];
  if (url === undefined) throw new Error(`${path} is not served`);
  return new URL(url, location.href).href;
}

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

/** The RPC tests' worker: the viewer's methods plus test hooks (test-worker.ts). */
function testWorker(): Worker {
  return new Worker(new URL("./test-worker.ts", import.meta.url), { type: "module", name: "lamina-viewer-test" });
}

/** A client on the test worker, for tests that drive the queue with its hooks (see {@link hook}). */
export function testViewer(
  backend: BackendName = "pdfium",
  options: Partial<ViewerOptions> = {},
): Promise<ViewerClient> {
  return viewer(backend, { worker: testWorker, ...options });
}

/**
 * Calls one of the test worker's hooks: `__busy` (ms; answers the wall clock when the hold ends),
 * `__renderLog`, `__renderDocument` (a number per render document the worker draws from),
 * `__failNextReopen`, `__failNextUndo` or `__crash`.
 */
export function hook(
  client: ViewerClient,
  method: "__busy" | "__renderLog" | "__renderDocument" | "__failNextReopen" | "__failNextUndo" | "__crash",
  params: unknown[] = [],
) {
  return (client.call as (m: string, p: unknown[]) => Promise<unknown>).call(client, method, params);
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
