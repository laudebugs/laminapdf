// Loading the WebAssembly modules (ADR 0016 §2, §3): `init()` compiles and instantiates the base
// module once; the text module is loaded by `init({ text: true })` or `loadText()`. After that,
// every call is synchronous. Each export condition supplies its own `Locate` (src/loader/), so
// this file has no environment detection and no runtime-specific API.
import * as baseGlue from "@lamina/wasm/base";
import { packageError } from "./errors.js";

/** The base module's wasm-bindgen glue. */
export type BaseGlue = typeof baseGlue;
/** The text module's wasm-bindgen glue (a superset of the base module's API). */
export type TextGlue = typeof import("@lamina/wasm/text");

/**
 * Where a module comes from: a URL (or URL string) to fetch, a `Response` (or a promise of one),
 * the bytes, or a compiled `WebAssembly.Module` (Cloudflare Workers import one).
 */
export type WasmSource =
  | URL
  | string
  | Response
  | PromiseLike<Response>
  | ArrayBuffer
  | ArrayBufferView
  | WebAssembly.Module;

/** Options for {@link init}. */
export interface InitOptions {
  /** The base module (default: the `lamina_base.wasm` next to the package, per export condition). */
  readonly module?: WasmSource;
  /**
   * Also load the text module (fonts and text): `true` for the default file, or where it comes
   * from. Without it, only builders created after a later {@link loadText} can draw text.
   */
  readonly text?: boolean | WasmSource;
}

/**
 * How an export condition finds a module's default file; `undefined` lets the wasm-bindgen glue
 * fetch the `.wasm` next to itself.
 */
export type Locate = (module: "base" | "text") => WasmSource | undefined | Promise<WasmSource | undefined>;

/** The `memory` export every wasm-bindgen module has. */
interface Exports {
  readonly memory: WebAssembly.Memory;
}

let base: { readonly glue: BaseGlue; readonly exports: Exports } | undefined;
let basePending: Promise<void> | undefined;
let text: { readonly glue: TextGlue; readonly exports: Exports } | undefined;
let textPending: Promise<void> | undefined;

function isOptions(input: unknown): input is InitOptions {
  if (typeof input !== "object" || input === null || "then" in input) return false;
  const proto: unknown = Object.getPrototypeOf(input);
  return proto === Object.prototype || proto === null;
}

async function fromResponse(response: Response): Promise<WebAssembly.Module> {
  if (!response.ok) {
    throw new Error(`fetching the lamina WebAssembly module failed: HTTP ${response.status}`);
  }
  const type = response.headers.get("content-type") ?? "";
  // compileStreaming needs the application/wasm type; file: URLs and some servers don't send it.
  if (typeof WebAssembly.compileStreaming === "function" && type.startsWith("application/wasm")) {
    return WebAssembly.compileStreaming(response);
  }
  return WebAssembly.compile(await response.arrayBuffer());
}

/** Compiles `source` (once: a `WebAssembly.Module` is used as it is). */
async function compile(source: WasmSource): Promise<WebAssembly.Module> {
  if (source instanceof WebAssembly.Module) return source;
  if (typeof source === "string" || source instanceof URL) return fromResponse(await fetch(source));
  if (source instanceof ArrayBuffer) return WebAssembly.compile(source);
  if (ArrayBuffer.isView(source)) {
    const bytes = new Uint8Array(source.buffer, source.byteOffset, source.byteLength);
    return WebAssembly.compile(bytes.slice());
  }
  return fromResponse(await source);
}

async function instantiate<G extends { default: (input?: { module_or_path: WebAssembly.Module }) => Promise<unknown> }>(
  glue: G,
  source: WasmSource | undefined,
): Promise<{ readonly glue: G; readonly exports: Exports }> {
  const input = source === undefined ? undefined : { module_or_path: await compile(source) };
  const exports = (await glue.default(input)) as Exports;
  return { glue, exports };
}

/** Loads the base module (once; a failed load can be retried). */
function loadBase(locate: Locate, source: WasmSource | undefined): Promise<void> {
  basePending ??= (async () => {
    base = await instantiate(baseGlue, source ?? (await locate("base")));
  })().catch((e: unknown) => {
    basePending = undefined;
    throw e;
  });
  return basePending;
}

/** Loads the text module (once; a failed load can be retried). */
export function loadTextWith(locate: Locate, source?: WasmSource): Promise<void> {
  textPending ??= (async () => {
    const glue = await import("@lamina/wasm/text");
    text = await instantiate(glue, source ?? (await locate("text")));
  })().catch((e: unknown) => {
    textPending = undefined;
    throw e;
  });
  return textPending;
}

/** `init` for an export condition's `locate`. */
export async function initWith(locate: Locate, input?: WasmSource | InitOptions): Promise<void> {
  const options: InitOptions = input === undefined ? {} : isOptions(input) ? input : { module: input };
  const loads = [loadBase(locate, options.module)];
  if (options.text !== undefined && options.text !== false) {
    loads.push(loadTextWith(locate, options.text === true ? undefined : options.text));
  }
  await Promise.all(loads);
}

/** The base module's glue. @throws {@link LaminaError} `not-initialized` before `init()`. */
export function baseModule(): BaseGlue {
  if (base === undefined) {
    throw packageError("not-initialized", "init() has not finished loading the WebAssembly module");
  }
  return base.glue;
}

/** The text module's glue, when loaded. */
export function textModule(): TextGlue | undefined {
  return text?.glue;
}

/** Whether `init()` has finished. */
export function isInitialized(): boolean {
  return base !== undefined;
}

/** Whether the text module is loaded (builders created now can draw text). */
export function isTextLoaded(): boolean {
  return text !== undefined;
}

/** The size of each loaded module's linear memory, in bytes. */
export interface WasmMemory {
  /** The base module's (absent before `init()`). */
  readonly base?: number;
  /** The text module's (absent until it is loaded). */
  readonly text?: number;
}

/**
 * The size of each loaded module's WebAssembly memory. Memory grows and never shrinks; freed
 * documents and builders leave room that later ones reuse.
 *
 * @returns the byte length of each loaded module's memory.
 */
export function wasmMemory(): WasmMemory {
  return {
    ...(base === undefined ? {} : { base: base.exports.memory.buffer.byteLength }),
    ...(text === undefined ? {} : { text: text.exports.memory.buffer.byteLength }),
  };
}
