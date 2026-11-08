// The resize, the label map, the box filter and the session wiring, with a fake ONNX Runtime
// standing in for onnxruntime-web (tools/heron-web runs the real model).
import { describe, expect, it } from "vitest";
import { HERON, HeronLayout, label, type OnnxRuntime } from "../src/index.ts";
import { regions } from "../src/post.ts";
import { chw640, SIDE } from "../src/pre.ts";

function flat(width: number, height: number, rgba: readonly number[]): Uint8Array {
  const out = new Uint8Array(width * height * 4);
  for (let i = 0; i < width * height; i++) out.set(rgba, i * 4);
  return out;
}

describe("chw640", () => {
  it("keeps a flat colour flat, one plane per channel", () => {
    const out = chw640(flat(10, 20, [10, 20, 30, 255]), 10, 20);
    expect(out.length).toBe(3 * SIDE * SIDE);
    expect(new Set(out.subarray(0, SIDE * SIDE))).toEqual(new Set([10]));
    expect(new Set(out.subarray(2 * SIDE * SIDE))).toEqual(new Set([30]));
  });

  it("reads a transparent page as white", () => {
    expect(new Set(chw640(flat(1240, 1754, [0, 0, 0, 0]), 1240, 1754))).toEqual(new Set([255]));
  });

  it("keeps a one-pixel stroke through the reduction", () => {
    const pixels = flat(1280, 64, [255, 255, 255, 255]);
    for (let y = 0; y < 64; y++) pixels.set([0, 0, 0], (y * 1280 + 641) * 4);
    const out = chw640(pixels, 1280, 64);
    expect(Array.from(out.subarray(0, SIDE)).some((v) => v < 200)).toBe(true);
  });
});

describe("regions", () => {
  // 1240 × 1754 pixels of an A4 page at 150 dpi.
  const page = { widthPx: 1240, heightPx: 1754, widthPt: 595.2, heightPt: 841.92 };

  it("drops low scores and keeps one label per box, in points", () => {
    const out = regions(
      {
        labels: [7, 10, 9, 8],
        boxes: [116, 114, 359, 166, 116, 114, 359, 166, 116, 308, 1124, 534, 200, 600, 300, 700],
        scores: [0.62, 0.59, 0.71, 0.2],
      },
      page,
      0.3,
    );
    expect(out.map((r) => r.label)).toEqual(["text", "section-header"]);
    // 116 px at 150 dpi is 55.68 pt.
    expect(out[1]?.bbox[0]).toBeCloseTo(55.68, 2);
  });

  it("names Heron's labels as the extraction does", () => {
    expect([label(0), label(5), label(10), label(12)]).toEqual(["caption", "page-header", "title", "other"]);
  });
});

/** A fake runtime whose session returns one title box, recording what it was fed. */
function fakeOrt(seen: Record<string, unknown>[]): OnnxRuntime {
  return {
    InferenceSession: {
      create: async () => ({
        run: async (feeds) => {
          seen.push(feeds);
          return { labels: { data: [10n] }, boxes: { data: [0, 0, 620, 100] }, scores: { data: [0.8] } };
        },
      }),
    },
    Tensor: class {
      readonly type: string;
      readonly data: unknown;
      readonly dims: readonly number[];
      constructor(type: string, data: unknown, dims: readonly number[]) {
        this.type = type;
        this.data = data;
        this.dims = dims;
      }
    },
  };
}

describe("HeronLayout", () => {
  it("refuses weights other than the pinned file", async () => {
    await expect(HeronLayout.load(new Uint8Array([1, 2, 3]), fakeOrt([]))).rejects.toThrow(/not the pinned/);
    expect(HERON.licence).toBe("Apache-2.0");
  });

  it("keeps its pin out of reach", () => {
    expect(Object.isFrozen(HERON)).toBe(true);
    expect(() => {
      (HERON as { sha256: string }).sha256 = "0".repeat(64);
    }).toThrow(TypeError);
  });
});

describe("HeronLayout.detect, past the hash check", () => {
  it("feeds the model 640 × 640 bytes and the page's size, and maps its boxes to points", async () => {
    const seen: Record<string, unknown>[] = [];
    // A layout built past the hash check, as `load` builds it.
    const ctor = HeronLayout as unknown as new (o: OnnxRuntime, s: unknown, t: number) => HeronLayout;
    const ort = fakeOrt(seen);
    const heron = new ctor(ort, await ort.InferenceSession.create(new Uint8Array()), 0.3);
    const out = await heron.detect({
      pixels: flat(1240, 200, [255, 255, 255, 255]),
      width: 1240,
      height: 200,
      widthPt: 620,
      heightPt: 100,
    });
    expect(out).toEqual([{ label: "title", bbox: [0, 0, 310, 50], score: 0.8 }]);
    const feeds = seen[0] as { images: { dims: number[] }; orig_target_sizes: { data: BigInt64Array } };
    expect(feeds.images.dims).toEqual([1, 3, 640, 640]);
    expect(Array.from(feeds.orig_target_sizes.data)).toEqual([1240n, 200n]);
  });

  it("refuses a raster whose bytes or sizes don't match", async () => {
    const ctor = HeronLayout as unknown as new (o: OnnxRuntime, s: unknown, t: number) => HeronLayout;
    const ort = fakeOrt([]);
    const heron = new ctor(ort, await ort.InferenceSession.create(new Uint8Array()), 0.3);
    const page = { pixels: flat(10, 10, [255, 255, 255, 255]), width: 10, height: 10, widthPt: 10, heightPt: 10 };
    await expect(heron.detect({ ...page, width: 12 })).rejects.toThrow(/not 12 × 10 × 4/);
    await expect(heron.detect({ ...page, width: 0, pixels: new Uint8Array() })).rejects.toThrow(/must be positive/);
    await expect(heron.detect({ ...page, width: 2.5 })).rejects.toThrow(/must be positive/);
  });
});
