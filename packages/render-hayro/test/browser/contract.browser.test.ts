// The RenderBackend contract in a browser (Chromium, Firefox, WebKit), with C2b's hayro goldens
// (native hayro 0.8.0; this is the same code compiled to wasm32).

import { decodePng, fetcher } from "@laminapdf/test-utils/browser-harness";
import { renderContract } from "@laminapdf/test-utils/render-contract";
import wasmUrl from "@laminapdf/wasm/lamina_pdf_render_hayro.wasm?url";
import { type HayroBackend, initHayro } from "../../src/index.js";

const urls = import.meta.glob<string>(
  [
    "../../../../fixtures/corpus/*.pdf",
    "../../../../fixtures/corpus/expected.json",
    "../../../../fixtures/encrypted/r4-aes-128-user.pdf",
    "../../../../fixtures/golden/render/hayro/*.png",
    "../../../../fixtures/appearance/*.pdf",
    "../../../../fixtures/golden/render-appearance/hayro/*.png",
  ],
  { query: "?url", import: "default", eager: true },
);

let backend: HayroBackend | undefined;

renderContract({
  backend: async () => {
    backend ??= await initHayro(wasmUrl);
    return backend;
  },
  read: fetcher(urls, "../../../../"),
  goldens: { decode: decodePng, minSsim: 0.99 },
  memory: () => backend?.memory() ?? 0,
});
