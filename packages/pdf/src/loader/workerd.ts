// The `workerd` condition (Cloudflare Workers): Workers can't compile WebAssembly at run time,
// so the modules are imported statically; wrangler turns a `.wasm` import into a
// `WebAssembly.Module` (ADR 0016 §2). Both modules are part of the Worker's bundle.
import base from "@lamina/wasm/lamina_base.wasm";
import text from "@lamina/wasm/lamina_text.wasm";
import { packageError } from "../errors.js";
import type { Locate } from "../init.js";

// The extract module isn't imported here: a static import would put it in every Worker's bundle
// (about 526 KB gzip) whether or not it extracts text. A Worker that does passes it itself.
export const locate: Locate = (module) => {
  if (module === "extract") {
    throw packageError(
      "extract-module-not-loaded",
      'in workerd, pass the extract module: import extract from "@lamina/wasm/lamina_extract.wasm"; await loadExtract(extract)',
    );
  }
  return module === "base" ? base : text;
};
