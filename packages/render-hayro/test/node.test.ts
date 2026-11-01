// The RenderBackend contract in Node, over the built lamina_pdf_render_hayro.wasm.
import { readFileSync } from "node:fs";
import { renderContract } from "@laminapdf/test-utils/render-contract";
import { fromRoot } from "@laminapdf/test-utils/root";
import { type HayroBackend, initHayro } from "../src/index.js";

let backend: HayroBackend | undefined;

renderContract({
  backend: async () => {
    backend ??= await initHayro(readFileSync(fromRoot("packages/wasm/dist/lamina_pdf_render_hayro.wasm")));
    return backend;
  },
  read: async (path) => new Uint8Array(readFileSync(fromRoot(path))),
  memory: () => backend?.memory() ?? 0,
});
