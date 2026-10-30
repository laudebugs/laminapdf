// The `browser`, `deno` and `default` entry (ADR 0016 §2): the modules are fetched from URLs
// next to the package.
import type { InitOptions, WasmSource } from "./init.js";
import { initWith, loadTextWith } from "./init.js";
import { locate } from "./loader/fetch.js";

/**
 * Loads the base WebAssembly module (fetched next to the package); with `{ text: true }`, the text
 * module too. Idempotent: later calls return at once (or load the text module). Everything else
 * in the package is synchronous and needs this promise to have resolved.
 *
 * @param input - where the base module comes from (a URL, `Response`, bytes or
 *   `WebAssembly.Module`), or options; omitted, the file next to the package.
 * @example
 * ```ts
 * await init(); // or: await init({ text: true })
 * ```
 */
export function init(input?: WasmSource | InitOptions): Promise<void> {
  return initWith(locate, input);
}

/**
 * Loads the text module (fonts, shaping and text layout; about 940 KB gzipped). Builders
 * created afterwards can draw text. Idempotent.
 *
 * @param source - where it comes from; omitted, the file next to the package.
 */
export function loadText(source?: WasmSource): Promise<void> {
  return loadTextWith(locate, source);
}

export * from "./api.js";
