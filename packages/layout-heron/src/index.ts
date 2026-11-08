// Docling's Heron layout model (RT-DETRv2, its official ONNX export) for @laminapdf/sdk's
// extractJob, run by the onnxruntime-web module the app passes in. The weights are the app's to
// fetch (tools/models/models.toml pins them); `HeronLayout.load` refuses any other file.
import type { LayoutRegion, ModelInfo } from "@laminapdf/sdk";
import { type PageSize, regions } from "./post.js";
import { chw640, SIDE } from "./pre.js";

export { label } from "./post.js";

/** The pinned weights' SHA-256, as tools/models/models.toml lists it; {@link HeronLayout.load} checks against this, not {@link HERON}. */
const PINNED_SHA256 = "59c81a3a2923042d85034ffc487f8f47e4854117e879aef89b2b9f728fb4922a";

/** The pinned weights, as tools/models/models.toml lists them (frozen). */
export const HERON: ModelInfo = Object.freeze({
  name: "docling-layout-heron",
  version: "40bde044036bb181c130ddf6c51792187268748f",
  sha256: PINNED_SHA256,
  licence: "Apache-2.0",
});

/** The part of onnxruntime-web this package uses: pass `await import("onnxruntime-web")`. */
export interface OnnxRuntime {
  readonly InferenceSession: {
    create(model: Uint8Array): Promise<OnnxSession>;
  };
  readonly Tensor: new (type: "uint8" | "int64", data: Uint8Array | BigInt64Array, dims: readonly number[]) => unknown;
}

/** An ONNX Runtime session, as {@link OnnxRuntime} creates it. */
export interface OnnxSession {
  run(feeds: Record<string, unknown>): Promise<Record<string, { readonly data: unknown }>>;
}

/** A rendered page: straight RGBA rows, and its size in points. */
export interface Raster {
  /** `width × height × 4` bytes. */
  readonly pixels: Uint8Array | Uint8ClampedArray;
  readonly width: number;
  readonly height: number;
  /** The page's displayed width in points. */
  readonly widthPt: number;
  /** The page's displayed height in points. */
  readonly heightPt: number;
}

/** The checks a native `PageRaster` makes (laminapdf-extract): a raster that fails them would give wrong boxes, not an error. */
function checkRaster({ pixels, width, height, widthPt, heightPt }: Raster): void {
  const whole = (n: number) => Number.isInteger(n) && n > 0;
  const finite = (n: number) => Number.isFinite(n) && n > 0;
  if (!whole(width) || !whole(height) || !finite(widthPt) || !finite(heightPt)) {
    throw new Error(`the raster's sizes must be positive: ${width} × ${height} px, ${widthPt} × ${heightPt} pt`);
  }
  if (pixels.length !== width * height * 4) {
    throw new Error(`the raster has ${pixels.length} bytes, not ${width} × ${height} × 4`);
  }
}

/** The SHA-256 of `bytes` in lowercase hex (WebCrypto: browsers, Node, Deno, workers). */
async function sha256(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", bytes as Uint8Array<ArrayBuffer>);
  return Array.from(new Uint8Array(digest), (b) => b.toString(16).padStart(2, "0")).join("");
}

/** Heron, loaded and ready to run. */
export class HeronLayout {
  /** The score a region needs unless {@link HeronLayout.load} is told otherwise, as Docling uses. */
  static readonly DEFAULT_THRESHOLD = 0.3;

  readonly #ort: OnnxRuntime;
  readonly #session: OnnxSession;
  readonly #threshold: number;

  private constructor(ort: OnnxRuntime, session: OnnxSession, threshold: number) {
    this.#ort = ort;
    this.#session = session;
    this.#threshold = threshold;
  }

  /**
   * Checks `weights` against the pinned SHA-256 and opens them with `ort`.
   *
   * @throws Error when the weights aren't the pinned file, or ONNX Runtime can't open them.
   */
  static async load(
    weights: Uint8Array,
    ort: OnnxRuntime,
    { threshold = HeronLayout.DEFAULT_THRESHOLD }: { readonly threshold?: number } = {},
  ): Promise<HeronLayout> {
    const found = await sha256(weights);
    if (found !== PINNED_SHA256) {
      throw new Error(`the weights' SHA-256 is ${found}, not the pinned ${PINNED_SHA256}`);
    }
    return new HeronLayout(ort, await ort.InferenceSession.create(weights), threshold);
  }

  /** The model as the output's `generator.models` names it: pass it in `options.models.layout`. */
  get info(): ModelInfo {
    return HERON;
  }

  /**
   * The regions on `page`, in top-left points, best first.
   *
   * @throws Error when `page`'s sizes aren't positive (whole pixels, finite points) or its pixels
   *   aren't `width × height × 4` bytes; when the model's outputs aren't the three it gives.
   */
  async detect(page: Raster): Promise<LayoutRegion[]> {
    checkRaster(page);
    const images = new this.#ort.Tensor("uint8", chw640(page.pixels, page.width, page.height), [1, 3, SIDE, SIDE]);
    const sizes = new this.#ort.Tensor("int64", BigInt64Array.from([BigInt(page.width), BigInt(page.height)]), [1, 2]);
    const out = await this.#session.run({ images, orig_target_sizes: sizes });
    const pick = (name: string): ArrayLike<number | bigint> => {
      const data = out[name]?.data;
      if (!ArrayBuffer.isView(data) && !Array.isArray(data))
        throw new Error(`Heron gave no numeric \`${name}\` output`);
      return data as ArrayLike<number | bigint>;
    };
    const size: PageSize = {
      widthPx: page.width,
      heightPx: page.height,
      widthPt: page.widthPt,
      heightPt: page.heightPt,
    };
    return regions(
      {
        labels: pick("labels"),
        boxes: pick("boxes") as ArrayLike<number>,
        scores: pick("scores") as ArrayLike<number>,
      },
      size,
      this.#threshold,
    );
  }
}
