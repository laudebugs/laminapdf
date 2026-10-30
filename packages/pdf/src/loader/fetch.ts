// The `browser`, `deno` and `default` conditions: no source, so each wasm-bindgen glue module
// fetches its own `.wasm` with `new URL("lamina_<module>.wasm", import.meta.url)`, which bundlers
// (Vite, webpack, esbuild, Rollup) emit as an asset (ADR 0016 §2), compiling it with
// `instantiateStreaming` when it is served as `application/wasm`.
import type { Locate } from "../init.js";

export const locate: Locate = () => undefined;
