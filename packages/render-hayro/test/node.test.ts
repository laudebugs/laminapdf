// The RenderBackend contract in Node, over the built lamina_render_hayro.wasm.
import { readFileSync } from "node:fs";
import { renderContract } from "../../render/test/contract.js";
import { type HayroBackend, initHayro } from "../src/index.js";

const root = new URL("../../../", import.meta.url);
let backend: HayroBackend | undefined;

renderContract({
  backend: async () => {
    backend ??= await initHayro(readFileSync(new URL("packages/wasm/dist/lamina_render_hayro.wasm", root)));
    return backend;
  },
  read: async (path) => new Uint8Array(readFileSync(new URL(path, root))),
  memory: () => backend?.memory() ?? 0,
});
