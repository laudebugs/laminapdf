// ADR 0004's second web renderer and CI oracle (todo C2c): lamina-render's hayro adapter, built by
// `cargo xtask wasm` as lamina_render_hayro (crates/lamina-render-wasm) into @lamina/wasm, behind
// @lamina/render's RenderBackend. hayro reads the whole file, so `open` fills the module's input
// buffer from the source in one pass, chunk by chunk (every missing range in one `data-pending`),
// and the module borrows it. Requests are validated by @lamina/render's planner (the Rust one,
// ported), so both web adapters refuse the same requests; Rust re-checks them.

import {
  flagsOf,
  isRenderError,
  isRenderErrorCode,
  limitsOf,
  planTile,
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
import initGlue, { HayroDocument, type InitInput, InputBuffer } from "@lamina/wasm/render-hayro";

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

/** Set once the module trapped: its state may be half-updated, so nothing else runs in it. */
let broken = false;

/**
 * Converts what the module threw: its errors (`code`, `kind`, `ranges`) into RenderError, and a
 * WebAssembly trap (a hayro panic: the module is built with `panic = "abort"`) into
 * `render-failed`, after which the instance is broken and the worker must be restarted
 * (ADR 0009 §3.6).
 */
function rethrow(e: unknown): never {
  if (e instanceof WebAssembly.RuntimeError) {
    broken = true;
    throw new RenderError("render-failed", "the hayro backend failed on this document (the module must be reloaded)");
  }
  // Only the module's own errors (a known render code): anything else is passed on unchanged.
  if (e instanceof Error && isRenderErrorCode((e as { code?: unknown }).code)) {
    const { code, ranges } = e as unknown as { code: RenderErrorCode; ranges?: [number, number][] };
    throw new RenderError(code, e.message, ranges);
  }
  throw e;
}

function healthy(): void {
  if (broken) {
    throw new RenderError(
      "render-backend-unavailable",
      "the hayro module failed earlier and must be reloaded: restart the worker",
    );
  }
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
    healthy();
    const limits = limitsOf(options.limits);
    try {
      const input = fill(source, limits);
      const doc = HayroDocument.open(
        input,
        options.password,
        limits.maxRenderPixels,
        limits.maxRenderSide,
        limits.maxInputBytes,
      );
      return new HayroRenderDocument(doc, limits);
    } catch (e) {
      if (e instanceof RenderError) throw e;
      rethrow(e);
    }
  }
}

/**
 * The module's input buffer, filled straight from `source` in 64 KiB chunks (never a whole
 * copy in JS), or `data-pending` with every range the source lacks.
 */
function fill(source: RenderSource, limits: RenderLimits): InputBuffer {
  if (source.size > limits.maxInputBytes || source.size > 0xffff_ffff) {
    throw new RenderError(
      "render-too-large",
      `the input is ${source.size} bytes, over the render limit of ${Math.min(limits.maxInputBytes, 0xffff_ffff)} bytes`,
    );
  }
  const input = new InputBuffer(source.size);
  const pending: [number, number][] = [];
  try {
    for (let at = 0; at < source.size; at += CHUNK) {
      const length = Math.min(CHUNK, source.size - at);
      let chunk: Uint8Array | undefined;
      try {
        chunk = source.readSync(at, length);
      } catch (e) {
        // As PDFium's adapter: a throwing source is a failed read (or its own RenderError).
        throw isRenderError(e)
          ? e
          : new RenderError("source", `bytes ${at}..+${length} of the input could not be read`);
      }
      if (chunk === undefined) pending.push([at, length]);
      else if (chunk.length !== length) {
        throw new RenderError("source", `bytes ${at}..+${length} of the input could not be read`);
      } else if (pending.length === 0) input.write(at, chunk);
    }
  } catch (e) {
    input.free();
    throw e;
  }
  if (pending.length > 0) {
    input.free();
    throw new RenderError(
      "data-pending",
      `${pending.length} byte range(s) of the input are not available yet`,
      pending,
    );
  }
  return input;
}

/** The validated request as the module's flat arguments. */
function flat(
  request: RenderRequest,
  plan: TilePlan,
): [number, number, number, number, number, number, number, number, number | undefined] {
  const f = flagsOf(request.flags);
  const bits = (f.annotations ? 1 : 0) | (f.formWidgets ? 2 : 0) | (f.print ? 4 : 0) | (f.grayscale ? 8 : 0);
  const c = request.background === undefined ? WHITE : request.background;
  const background = c === null ? undefined : ((c.r << 24) | (c.g << 16) | (c.b << 8) | c.a) >>> 0;
  const { x, y, width, height } = plan.tile;
  return [request.page, request.scale, x, y, width, height, request.rotation ?? 0, bits, background];
}

class HayroRenderDocument implements RenderDocument {
  readonly pageCount: number;
  #doc: HayroDocument | undefined;
  readonly #limits: RenderLimits;
  readonly #pages = new Map<number, RenderPage>();

  constructor(doc: HayroDocument, limits: RenderLimits) {
    this.#doc = doc;
    this.#limits = limits;
    this.pageCount = doc.pageCount();
  }

  #live(): HayroDocument {
    if (this.#doc === undefined) throw new RenderError("disposed", "the document was closed");
    healthy();
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
    const cached = this.#pages.get(index);
    if (cached !== undefined) return cached;
    try {
      const [width = 0, height = 0, rotate = 0] = doc.page(index);
      const page = { width, height, rotate };
      this.#pages.set(index, page);
      return page;
    } catch (e) {
      rethrow(e);
    }
  }

  /** The shared planner (Rust's, ported): both web adapters refuse the same requests. */
  plan(request: RenderRequest): TilePlan {
    return planTile(this.page(request.page), request, this.#limits);
  }

  render(request: RenderRequest, options: { readonly signal?: AbortSignal } = {}): RenderedTile {
    throwIfAborted(options.signal);
    const doc = this.#live();
    const plan = this.plan(request);
    const pixels = new Uint8ClampedArray(plan.byteLength);
    try {
      // wasm-bindgen copies `&mut [u8]` back into this array after the call.
      doc.render(...flat(request, plan), new Uint8Array(pixels.buffer));
    } catch (e) {
      rethrow(e);
    }
    throwIfAborted(options.signal);
    return { width: plan.tile.width, height: plan.tile.height, pixels };
  }

  close(): void {
    // A broken instance can't run `free`; dropping the handle is all that's left.
    if (!broken) this.#doc?.free();
    this.#doc = undefined;
  }

  [Symbol.dispose](): void {
    this.close();
  }
}
