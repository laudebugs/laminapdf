// The RenderBackend contract in Node, over the installed (hash-checked) pdfium.wasm.
import { readFileSync } from "node:fs";
import { renderContract } from "../../render/test/contract.js";
import { wasmPath } from "../scripts/verify-pdfium.mjs";
import { initPdfium, type PdfiumBackend } from "../src/index.js";

const root = new URL("../../../", import.meta.url);
let backend: PdfiumBackend | undefined;

renderContract({
  backend: async () => {
    backend ??= await initPdfium(new Uint8Array(readFileSync(wasmPath())));
    return backend;
  },
  read: async (path) => new Uint8Array(readFileSync(new URL(path, root))),
  memory: () => backend?.memory() ?? 0,
});
