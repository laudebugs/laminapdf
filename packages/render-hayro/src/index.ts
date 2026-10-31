// ADR 0004's second web renderer and CI oracle (todo C2c): lamina-render's hayro adapter, built by
// `cargo xtask wasm` as lamina_render_hayro (crates/lamina-render-wasm) into @lamina/wasm, behind
// @lamina/render's RenderBackend. hayro reads the whole file, so `open` reads the source in one
// pass (every missing range in one `data-pending`) and moves the bytes into the module, which
// borrows them (no second copy inside). Tile geometry and validation are the Rust planner's.

import {
  flagsOf,
  limitsOf,
  type RenderBackend,
  type RenderDocument,
  RenderError,
  type RenderErrorCode,
  type RenderedTile,
  type RenderLimits,
  type RenderOpenOptions,
  type RenderPage,
  type RenderRequest,
  type RenderSource,
  type TilePlan,
  throwIfAborted,
  WHITE,
} from "@lamina/render";
import initGlue, { HayroDocument, type InitInput } from "@lamina/wasm/render-hayro";

/** The pinned hayro version (the workspace's `=0.8.0`). */
export const HAYRO_VERSION = "0.8.0";

/** Reads per call, as lamina-cos reads windows. */
const CHUNK = 64 * 1024;

let memory: WebAssembly.Memory | undefined;

/**
 * Loads the module: a URL, `Response`, bytes or compiled module; omitted, the glue fetches
 * `lamina_render_hayro.wasm` next to itself. Call it inside the document worker (ADR 0009 §2).
 *
 * @throws RenderError `render-backend-unavailable` when the module can't be loaded.
 */
export async function initHayro(wasm?: InitInput | Promise<InitInput>): Promise<HayroBackend> {
  try {
    const output = await initGlue(wasm === undefined ? undefined : { module_or_path: wasm });
    memory = output.memory;
  } catch {
    throw new RenderError("render-backend-unavailable", "the hayro module could not be loaded");
  }
  return new HayroBackend();
}

/** Converts the module's thrown errors (`code`, `kind`, `ranges`) into RenderError. */
function rethrow(e: unknown): never {
  if (e instanceof Error && typeof (e as { code?: unknown }).code === "string") {
    const { code, ranges } = e as unknown as { code: RenderErrorCode; ranges?: [number, number][] };
    throw new RenderError(code, e.message, ranges);
  }
  throw e;
}

/** hayro as a RenderBackend. */
export class HayroBackend implements RenderBackend {
  readonly name = "hayro";
  readonly version = HAYRO_VERSION;

  /** The size of the module's WebAssembly memory, in bytes. */
  memory(): number {
    return memory?.buffer.byteLength ?? 0;
  }

  open(source: RenderSource, options: RenderOpenOptions = {}): RenderDocument {
    const limits = limitsOf(options.limits);
    const bytes = readAll(source, limits);
    try {
      const doc = HayroDocument.open(bytes, options.password, limits.maxRenderPixels, limits.maxRenderSide);
      return new HayroRenderDocument(doc, limits);
    } catch (e) {
      rethrow(e);
    }
  }
}

/** All of `source`, or `data-pending` with every range it lacks. */
function readAll(source: RenderSource, limits: RenderLimits): Uint8Array {
  if (source.size > limits.maxInputBytes) {
    throw new RenderError(
      "render-too-large",
      `the input is ${source.size} bytes, over the render limit of ${limits.maxInputBytes} bytes`,
    );
  }
  const out = new Uint8Array(source.size);
  const pending: [number, number][] = [];
  for (let at = 0; at < source.size; at += CHUNK) {
    const length = Math.min(CHUNK, source.size - at);
    const chunk = source.readSync(at, length);
    if (chunk === undefined) pending.push([at, length]);
    else if (chunk.length !== length) {
      throw new RenderError("source", `bytes ${at}..+${length} of the input could not be read`);
    } else out.set(chunk, at);
  }
  if (pending.length > 0) {
    throw new RenderError(
      "data-pending",
      `${pending.length} byte range(s) of the input are not available yet`,
      pending,
    );
  }
  return out;
}

/** The request as the module's flat arguments. */
function flat(
  request: RenderRequest,
): [number, number, number, number, number, number, number, number, number | undefined] {
  const f = flagsOf(request.flags);
  const bits = (f.annotations ? 1 : 0) | (f.formWidgets ? 2 : 0) | (f.print ? 4 : 0) | (f.grayscale ? 8 : 0);
  const c = request.background === undefined ? WHITE : request.background;
  const background = c === null ? undefined : ((c.r << 24) | (c.g << 16) | (c.b << 8) | c.a) >>> 0;
  const clip = request.clip ?? { x: 0, y: 0, width: 0, height: 0 };
  return [
    request.page,
    request.scale,
    clip.x,
    clip.y,
    clip.width,
    clip.height,
    request.rotation ?? 0,
    bits,
    background,
  ];
}

class HayroRenderDocument implements RenderDocument {
  readonly pageCount: number;
  #doc: HayroDocument | undefined;
  readonly #limits: RenderLimits;

  constructor(doc: HayroDocument, limits: RenderLimits) {
    this.#doc = doc;
    this.#limits = limits;
    this.pageCount = doc.pageCount();
  }

  #live(): HayroDocument {
    if (this.#doc === undefined) throw new RenderError("disposed", "the document was closed");
    return this.#doc;
  }

  page(index: number): RenderPage {
    const doc = this.#live();
    if (!Number.isInteger(index) || index < 0 || index >= this.pageCount) {
      throw new RenderError(
        "page-index-out-of-range",
        `page index ${index} is out of range for a document of ${this.pageCount} pages`,
      );
    }
    try {
      const [width = 0, height = 0, rotate = 0] = doc.page(index);
      return { width, height, rotate };
    } catch (e) {
      rethrow(e);
    }
  }

  plan(request: RenderRequest): TilePlan {
    const doc = this.#live();
    this.page(request.page);
    try {
      const [pageWidth = 0, pageHeight = 0, x = 0, y = 0, width = 0, height = 0] = doc.plan(...flat(request));
      return { pageWidth, pageHeight, tile: { x, y, width, height }, byteLength: width * height * 4 };
    } catch (e) {
      rethrow(e);
    }
  }

  render(request: RenderRequest, options: { readonly signal?: AbortSignal } = {}): RenderedTile {
    throwIfAborted(options.signal);
    const doc = this.#live();
    const plan = this.plan(request);
    if (plan.byteLength > this.#limits.maxRenderPixels * 4) {
      throw new RenderError("render-too-large", "the tile exceeds the render limit");
    }
    const pixels = new Uint8ClampedArray(plan.byteLength);
    try {
      // wasm-bindgen copies `&mut [u8]` back into this array after the call.
      doc.render(...flat(request), new Uint8Array(pixels.buffer));
    } catch (e) {
      rethrow(e);
    }
    throwIfAborted(options.signal);
    return { width: plan.tile.width, height: plan.tile.height, pixels };
  }

  close(): void {
    this.#doc?.free();
    this.#doc = undefined;
  }

  [Symbol.dispose](): void {
    this.close();
  }
}
