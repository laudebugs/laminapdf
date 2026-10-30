// The `workerd` condition (Cloudflare Workers): Workers can't compile WebAssembly at run time,
// so the modules are imported statically; wrangler turns a `.wasm` import into a
// `WebAssembly.Module` (ADR 0016 §2). Both modules are part of the Worker's bundle.
import base from "@lamina/wasm/lamina_base.wasm";
import extract from "@lamina/wasm/lamina_extract.wasm";
import text from "@lamina/wasm/lamina_text.wasm";
import type { Locate } from "../init.js";

export const locate: Locate = (module) => (module === "base" ? base : module === "text" ? text : extract);
