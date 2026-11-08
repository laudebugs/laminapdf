// Docling's Heron layout model (RT-DETRv2, its official ONNX export) for @laminapdf/sdk's
// extractJob, run by the onnxruntime-web module the app passes in. The weights are the app's to
// fetch (tools/models/models.toml pins them); `HeronLayout.load` refuses any other file.
import type { LayoutRegion, ModelInfo } from "@laminapdf/sdk";
import { type PageSize, regions } from "./post.js";
import { chw640, SIDE } from "./pre.js";

export { label } from "./post.js";

/** The pinned weights, as tools/models/models.toml lists them. */
export const HERON: ModelInfo = {
  name: "docling-layout-heron",
  version: "40bde044036bb181c130ddf6c51792187268748f",
  sha256: "59c81a3a2923042d85034ffc487f8f47e4854117e879aef89b2b9f728fb4922a",
  licence: "Apache-2.0",
};

/** The part of onnxruntime-web this package uses: pass `await import("onnxruntime-web")`. */
export interface OnnxRuntime {
  readonly InferenceSession: {
    create(model: Uint8Array): Promise<OnnxSession>;
  };
  readonly Tensor: new (type: "uint8" | "int64", data: Uint8Array | BigInt64Array, dims: readonly number[]) => unknown;
}

/** An ONNX Runtime session, as {@link OnnxRuntime} creates it. */
export interface OnnxSession {
  run(feeds: Record<string, unknown>): Promise<Record<string, { readonly data: ArrayLike<number | bigint> }>>;
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
    if (found !== HERON.sha256) {
      throw new Error(`the weights' SHA-256 is ${found}, not the pinned ${HERON.sha256}`);
    }
    return new HeronLayout(ort, await ort.InferenceSession.create(weights), threshold);
  }

  /** The model as the output's `generator.models` names it: pass it in `options.models.layout`. */
  get info(): ModelInfo {
    return HERON;
  }

  /** The regions on `page`, in top-left points, best first. */
  async detect(page: Raster): Promise<LayoutRegion[]> {
    const images = new this.#ort.Tensor("uint8", chw640(page.pixels, page.width, page.height), [1, 3, SIDE, SIDE]);
    const sizes = new this.#ort.Tensor("int64", BigInt64Array.from([BigInt(page.width), BigInt(page.height)]), [1, 2]);
    const out = await this.#session.run({ images, orig_target_sizes: sizes });
    const pick = (name: string) => {
      const value = out[name];
      if (value === undefined) throw new Error(`Heron gave no \`${name}\` output`);
      return value.data;
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
