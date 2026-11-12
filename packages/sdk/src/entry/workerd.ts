// The `workerd` entry (Cloudflare Workers): the modules are imported as `WebAssembly.Module`s.
import type { InitOptions, WasmSource } from "../init.js";
import { initWith, loadExtractWith, loadTextWith } from "../init.js";
import { locate } from "../loader/workerd.js";

/**
 * Loads the base WebAssembly module (imported with the Worker's bundle); with `{ text: true }`, the text
 * module too. Idempotent: later calls return at once (or load the text module). Everything else
 * in the package is synchronous and needs this promise to have resolved.
 *
 * @param input - where the base module comes from (a URL, `Response`, bytes or
 *   `WebAssembly.Module`), or options; omitted, the file next to the package.
 * @example
 * ```ts
 * await init(); // or: await init({ text: true })
 * ```
 *
 * @since 0.2.0
 */
export function init(input?: WasmSource | InitOptions): Promise<void> {
  return initWith(locate, input);
}

/**
 * Loads the text module (fonts, shaping and text layout; about 940 KB gzipped). Builders
 * created afterwards can draw text. Idempotent.
 *
 * @param source - where it comes from; omitted, the file next to the package.
 *
 * @since 0.2.0
 */
export function loadText(source?: WasmSource): Promise<void> {
  return loadTextWith(locate, source);
}

/**
 * Loads the extract module (reading plus text extraction). Documents opened afterwards can
 * extract text with `page.text()`. Idempotent; not needed when the text module is loaded.
 *
 * @param source - where it comes from; omitted, the file next to the package.
 *
 * @since 0.2.0
 */
export function loadExtract(source?: WasmSource): Promise<void> {
  return loadExtractWith(locate, source);
}

export * from "../api.js";
