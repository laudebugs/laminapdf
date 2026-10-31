// The RenderBackend contract in a browser (Chromium, Firefox, WebKit), with C2b's PDFium goldens.
// The goldens are native PDFium (pdfium-binaries chromium/7881); this is EmbedPDF's WASM build of
// a different revision, so the bar is lower than the native test's 0.99 against itself.
import wasmUrl from "@embedpdf/pdfium/pdfium.wasm?url";
import { decodePng, fetcher } from "../../../render/test/browser-harness.js";
import { renderContract } from "../../../render/test/contract.js";
import { initPdfium, type PdfiumBackend } from "../../src/index.js";

const urls = import.meta.glob<string>(
  [
    "../../../../fixtures/corpus/*.pdf",
    "../../../../fixtures/corpus/expected.json",
    "../../../../fixtures/encrypted/r4-aes-128-user.pdf",
    "../../../../fixtures/golden/render/pdfium/*.png",
  ],
  { query: "?url", import: "default", eager: true },
);

let backend: PdfiumBackend | undefined;

renderContract({
  backend: async () => {
    backend ??= await initPdfium(wasmUrl);
    return backend;
  },
  read: fetcher(urls, "../../../../"),
  goldens: { decode: decodePng, minSsim: 0.98 },
  memory: () => backend?.memory() ?? 0,
});
