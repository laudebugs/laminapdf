// Loading the WebAssembly modules (ADR 0016 §2, §3): `init()` compiles and instantiates the base
// module once; the text module is loaded by `init({ text: true })` or `loadText()`, the extract
// module (base + text extraction) by `init({ extract: true })` or `loadExtract()`. After
// that, every call is synchronous. `reset()` replaces each loaded module's instance
// (internal/instance.ts). Each export condition supplies its own `Locate` (src/loader/), so
// this file has no environment detection and no runtime-specific API.
import * as baseGlue from "@laminapdf/wasm/base";
import { packageError } from "./errors.js";
import { Instance } from "./internal/instance.js";

/** The base module's wasm-bindgen glue. */
export type BaseGlue = typeof baseGlue;
/** The text module's wasm-bindgen glue (a superset of the base module's API). */
export type TextGlue = typeof import("@laminapdf/wasm/text");
/** The extract module's wasm-bindgen glue (the base module's API plus `WasmPage.text`). */
type ExtractGlue = typeof import("@laminapdf/wasm/extract");

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
  /** The base module (default: the `lamina_pdf_base.wasm` next to the package, per export condition). */
  readonly module?: WasmSource;
  /**
   * Also load the text module (fonts and text): `true` for the default file, or where it comes
   * from. Without it, only builders created after a later {@link loadText} can draw text.
   */
  readonly text?: boolean | WasmSource;
  /**
   * Also load the extract module (reading plus text extraction, about half the text module's
   * size): `true` for the default file, or where it comes from. Documents opened afterwards can
   * extract text ({@link PageRef.text}). Not needed when the text module is loaded.
   */
  readonly extract?: boolean | WasmSource;
}

/**
 * How an export condition finds a module's default file; `undefined` lets the wasm-bindgen glue
 * fetch the `.wasm` next to itself.
 */
export type Locate = (module: "base" | "text" | "extract") => WasmSource | undefined | Promise<WasmSource | undefined>;

/** One module's state: what it was asked to load from, and the load (pending or done). */
interface Slot<G extends BaseGlue> {
  /** The source of the first call (`undefined`: the export condition's default file). */
  requested?: WasmSource | undefined;
  pending?: Promise<void> | undefined;
  loaded?: Instance<G> | undefined;
}

const base: Slot<BaseGlue> = {};
const text: Slot<TextGlue> = {};
const extract: Slot<ExtractGlue> = {};

function isOptions(input: unknown): input is InitOptions {
  if (typeof input !== "object" || input === null || "then" in input) return false;
  const proto: unknown = Object.getPrototypeOf(input);
  return proto === Object.prototype || proto === null;
}

/** The glue's default `init`: it fetches URLs, streams `Response`s (falling back when the MIME type isn't wasm) and instantiates bytes or a `WebAssembly.Module` as they are. */
type GlueInit = (input?: { module_or_path: unknown }) => Promise<unknown>;

/**
 * Loads `slot`'s module once with `glue` (a failed load can be retried). A later call naming a
 * different source than the first throws `already-initialized` instead of being ignored.
 */
function load<G extends BaseGlue & { default: GlueInit }>(
  slot: Slot<G>,
  name: "base" | "text" | "extract",
  glue: () => Promise<G>,
  locate: Locate,
  source: WasmSource | undefined,
): Promise<void> {
  if (slot.pending !== undefined) {
    if (source !== undefined && source !== slot.requested) {
      return Promise.reject(
        packageError(
          "already-initialized",
          `the ${name} module is already loaded or loading from another source; the new source is ignored`,
        ),
      );
    }
    return slot.pending;
  }
  slot.requested = source;
  slot.pending = (async () => {
    const g = await glue();
    const from = source ?? (await locate(name));
    const exports = (await g.default(from === undefined ? undefined : { module_or_path: from })) as {
      readonly memory: WebAssembly.Memory;
    };
    slot.loaded = new Instance(name, g, exports);
  })().catch((e: unknown) => {
    slot.pending = undefined;
    slot.requested = undefined;
    throw e;
  });
  return slot.pending;
}

/** Loads the text module (once; a failed load can be retried). */
export function loadTextWith(locate: Locate, source?: WasmSource): Promise<void> {
  return load(text, "text", () => import("@laminapdf/wasm/text"), locate, source);
}

/**
 * Loads the extract module (once; a failed load can be retried). Not when the text module is
 * loaded or loading: it extracts too, and documents open in it first, so the extract module
 * would never be used.
 */
export function loadExtractWith(locate: Locate, source?: WasmSource): Promise<void> {
  const loadIt = () => load(extract, "extract", () => import("@laminapdf/wasm/extract"), locate, source);
  // A source given is honoured as for any module; without one, a text module loaded or loading
  // makes the extract module unnecessary, unless that load fails.
  if (source !== undefined || text.pending === undefined) return loadIt();
  return text.pending.catch(loadIt);
}

/** `init` for an export condition's `locate`. */
export async function initWith(locate: Locate, input?: WasmSource | InitOptions): Promise<void> {
  const options: InitOptions = input === undefined ? {} : isOptions(input) ? input : { module: input };
  const loads = [load(base, "base", async () => baseGlue, locate, options.module)];
  if (options.text !== undefined && options.text !== false) {
    loads.push(loadTextWith(locate, options.text === true ? undefined : options.text));
  }
  // The text module extracts too: with it, the extract module would never be used.
  if (
    options.extract !== undefined &&
    options.extract !== false &&
    (options.text === undefined || options.text === false)
  ) {
    loads.push(loadExtractWith(locate, options.extract === true ? undefined : options.extract));
  }
  await Promise.all(loads);
}

/** The base module. @throws {@link LaminaPdfError} `not-initialized` before `init()`. */
export function baseModule(): Instance<BaseGlue> {
  if (base.loaded === undefined) {
    throw packageError("not-initialized", "init() has not finished loading the WebAssembly module");
  }
  return base.loaded;
}

/**
 * The module documents are opened in: the text module when loaded, else the extract module, else
 * the base one (each a superset of the base module's reading API; the first two extract text).
 * @throws {@link LaminaPdfError} `not-initialized` before `init()`.
 */
export function readingModule(): Instance<BaseGlue> {
  const base = baseModule();
  return (text.loaded ?? extract.loaded ?? base) as unknown as Instance<BaseGlue>;
}

/**
 * Whether documents opened or built from now on can extract text: the extract or the text module
 * is loaded. A document opened earlier stays in the module it was opened in; ask it with
 * {@link Document.canExtractText}.
 */
export function isExtractLoaded(): boolean {
  return extract.loaded !== undefined || text.loaded !== undefined;
}

/** Whether `doc` lives in a module that extracts text (the extract or the text module). */
export function extractsText(doc: object): boolean {
  return (
    (text.loaded !== undefined && doc instanceof text.loaded.glue.WasmDocument) ||
    (extract.loaded !== undefined && doc instanceof extract.loaded.glue.WasmDocument)
  );
}

/** The text module, when loaded. */
export function textModule(): Instance<TextGlue> | undefined {
  return text.loaded;
}

/** Whether `init()` has finished. */
export function isInitialized(): boolean {
  return base.loaded !== undefined;
}

/** Whether the text module is loaded (builders created now can draw text). */
export function isTextLoaded(): boolean {
  return text.loaded !== undefined;
}

/** The size of each loaded module's linear memory, in bytes. */
export interface WasmMemory {
  /** The base module's (absent before `init()`). */
  readonly base?: number;
  /** The text module's (absent until it is loaded). */
  readonly text?: number;
  /** The extract module's (absent until it is loaded). */
  readonly extract?: number;
}

/**
 * The size of each loaded module's WebAssembly memory. Memory grows and never shrinks; freed
 * documents and builders leave room that later ones reuse.
 *
 * @returns the byte length of each loaded module's memory.
 */
export function wasmMemory(): WasmMemory {
  return {
    ...(base.loaded === undefined ? {} : { base: base.loaded.memoryBytes }),
    ...(text.loaded === undefined ? {} : { text: text.loaded.memoryBytes }),
    ...(extract.loaded === undefined ? {} : { extract: extract.loaded.memoryBytes }),
  };
}

/**
 * Replaces each loaded module's instance with a fresh one, so its memory returns to the size it
 * had after `init()`: WebAssembly memory grows and never shrinks, and this gives it back. Every
 * document, page, builder and save job made before throws `instance-reset` afterwards, so call it
 * while none is in use, such as between jobs on a server. Images and fonts keep working.
 */
export function reset(): void {
  for (const slot of [base, text, extract]) slot.loaded?.replace();
}
