// The RenderBackend contract in Node, over the installed (hash-checked) pdfium.wasm.
import { readFileSync } from "node:fs";
import { renderContract } from "@laminapdf/test-utils/render-contract";
import { fromRoot } from "@laminapdf/test-utils/root";
import { describe, expect, it } from "vitest";
import { wasmPath } from "../scripts/verify-pdfium.mjs";
import { initPdfium, type PdfiumBackend } from "../src/index.js";

let backend: PdfiumBackend | undefined;

renderContract({
  backend: async () => {
    backend ??= await initPdfium(new Uint8Array(readFileSync(wasmPath())));
    return backend;
  },
  read: async (path) => new Uint8Array(readFileSync(fromRoot(path))),
  memory: () => backend?.memory() ?? 0,
});

describe("initPdfium", () => {
  const bytes = (): Uint8Array<ArrayBuffer> => new Uint8Array(readFileSync(wasmPath()));

  it("loads from an ArrayBuffer (the glue's wasmBinary)", async () => {
    expect((await initPdfium(bytes().buffer)).memory()).toBeGreaterThan(0);
  });

  it("loads from a DataView of the bytes, as from any view", async () => {
    expect((await initPdfium(new DataView(bytes().buffer))).memory()).toBeGreaterThan(0);
  });
});
