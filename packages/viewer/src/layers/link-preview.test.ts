import type { Target } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { CSS_PX_PER_PT } from "../layout/page-layout.js";
import type { TextRunData } from "../protocol.js";
import {
  PAGE_PREVIEW,
  PREVIEW_MAX,
  PREVIEW_NO_TEXT_WIDTH,
  PREVIEW_PAD,
  type PreviewLine,
  previewLines,
  previewRegion,
  previewRender,
  previewText,
} from "./link-preview.js";

const LETTER = { width: 612, height: 792 };
const at = (page: number, x: number | undefined, y: number | undefined): Target => ({
  page,
  view: { kind: "at", ...(x === undefined ? {} : { x }), ...(y === undefined ? {} : { y }) },
});
const line = (left: number, top: number, right: number, text = "line"): PreviewLine => ({
  left,
  top,
  right,
  bottom: top + 10,
  text,
});

/** A two-column page: references at x 72–296 and 316–540, every 14 pt from the top. */
const TWO_COLUMNS: PreviewLine[] = Array.from({ length: 40 }, (_, i) => [
  line(72, 72 + i * 14, 290, `[${i + 1}] Left reference ${i + 1}`),
  line(316, 72 + i * 14, 534, `[${i + 41}] Right reference ${i + 41}`),
]).flat();

describe("previewRegion", () => {
  it("starts a region preview just above and left of the destination point", () => {
    const { clip, kind } = previewRegion(at(6, 72, 100), LETTER, TWO_COLUMNS, 100);
    expect({ kind, x: clip.x, y: clip.y }).toEqual({ kind: "region", x: 72 - PREVIEW_PAD, y: 100 - PREVIEW_PAD });
  });

  it("ends the crop at its column's right edge in the left column", () => {
    const { clip } = previewRegion(at(6, 72, 100), LETTER, TWO_COLUMNS, 100);
    expect(clip.x + clip.width).toBe(290 + PREVIEW_PAD);
  });

  it("shows the right column for a reference there", () => {
    const { clip } = previewRegion(at(6, 316, 100), LETTER, TWO_COLUMNS, 100);
    expect([clip.x, clip.x + clip.width]).toEqual([316 - PREVIEW_PAD, 534 + PREVIEW_PAD]);
  });

  it("is 360 pt wide without text lines", () => {
    const { clip } = previewRegion(at(0, 72, 100), LETTER, undefined, 100);
    expect(clip.width).toBe(PREVIEW_NO_TEXT_WIDTH + PREVIEW_PAD);
  });

  it("draws at the reader's zoom between 100 and 150 %", () => {
    const scales = [50, 120, 400].map((z) => previewRegion(at(0, 72, 100), LETTER, TWO_COLUMNS, z).scale);
    expect(scales.map((s) => Math.round((s / CSS_PX_PER_PT) * 100))).toEqual([100, 120, 150]);
  });

  it("lowers the scale to fit a wide region, never below 75 %", () => {
    const wide = [line(20, 100, 600)];
    const { scale, clip } = previewRegion(at(0, 20, 100), LETTER, wide, 150);
    expect([Math.round((scale / CSS_PX_PER_PT) * 100), Math.round(clip.width * scale)]).toEqual([
      75,
      PREVIEW_MAX.width,
    ]);
  });

  it("is never more than 180 CSS px high", () => {
    const { clip, scale } = previewRegion(at(0, 72, 100), LETTER, TWO_COLUMNS, 100);
    expect(clip.height * scale).toBeLessThanOrEqual(PREVIEW_MAX.height + 1e-9);
  });

  it("stops at the page's bottom", () => {
    const { clip } = previewRegion(at(0, 72, 780), LETTER, undefined, 100);
    expect(clip.y + clip.height).toBeLessThanOrEqual(LETTER.height);
  });

  it("previews a FitH destination's band from its leftmost line", () => {
    const target: Target = { page: 0, view: { kind: "fit-width", y: 100 } };
    const { kind, clip } = previewRegion(target, LETTER, TWO_COLUMNS, 100);
    expect({ kind, x: clip.x }).toEqual({ kind: "region", x: 72 - PREVIEW_PAD });
  });

  it("previews a FitR destination as its rectangle, fitted to the card", () => {
    const target: Target = { page: 0, view: { kind: "fit-rect", x0: 100, y0: 200, x1: 300, y1: 260 } };
    const { clip, scale } = previewRegion(target, LETTER, undefined, 100);
    expect({ x: clip.x, y: clip.y, fits: clip.width * scale <= 480 && clip.height * scale <= 180 }).toEqual({
      x: 100,
      y: 200,
      fits: true,
    });
  });

  it.each([
    ["Fit", { kind: "fit" } as const],
    ["FitV", { kind: "fit-height", x: 50 } as const],
    ["XYZ without a top", { kind: "at", x: 50 } as const],
  ])("previews the page's top for %s", (_, view) => {
    const { kind, clip, scale } = previewRegion({ page: 3, view }, LETTER, TWO_COLUMNS, 100);
    expect({ kind, y: clip.y, width: Math.round(clip.width * scale), height: Math.round(clip.height * scale) }).toEqual(
      {
        kind: "page",
        y: 0,
        width: PAGE_PREVIEW.width,
        height: PAGE_PREVIEW.height,
      },
    );
  });

  it("clamps a top above the page and a left past it", () => {
    const { clip } = previewRegion(at(0, 9000, -50), LETTER, undefined, 100);
    expect(clip.x >= 0 && clip.y === 0 && clip.x + clip.width <= LETTER.width).toBe(true);
  });

  it("clamps a FitR rectangle outside the page (a CropBox smaller than the MediaBox)", () => {
    const cropped = { width: 400, height: 500 };
    const target: Target = { page: 0, view: { kind: "fit-rect", x0: 350, y0: 450, x1: 700, y1: 900 } };
    const { clip } = previewRegion(target, cropped, undefined, 100);
    expect(clip.x + clip.width <= 400 && clip.y + clip.height <= 500).toBe(true);
  });

  it("survives NaN coordinates", () => {
    const { clip } = previewRegion(at(0, Number.NaN, Number.NaN), LETTER, TWO_COLUMNS, Number.NaN);
    expect(Object.values(clip).every(Number.isFinite)).toBe(true);
  });
});

describe("previewRender", () => {
  const region = { kind: "region", page: 0, clip: { x: 100, y: 200, width: 200, height: 50 }, scale: 1 } as const;

  it("asks for the clip in device pixels without rotation", () => {
    expect(previewRender(region, LETTER, 0, 2).clip).toEqual({ x: 200, y: 400, width: 400, height: 100 });
  });

  it("turns the clip a quarter clockwise at 90°", () => {
    // Page space (x, y) shows at (height − y, x): the band 200–250 down becomes 542–592 across.
    expect(previewRender(region, LETTER, 90, 1).clip).toEqual({ x: 542, y: 100, width: 50, height: 200 });
  });

  it("turns the clip a quarter anticlockwise at 270°", () => {
    // (x, y) shows at (y, width − x).
    expect(previewRender(region, LETTER, 270, 1).clip).toEqual({ x: 200, y: 312, width: 50, height: 200 });
  });

  it("swaps the crop's CSS size on a quarter turn", () => {
    expect(previewRender(region, LETTER, 90, 1).css).toEqual({ width: 50, height: 200 });
  });
});

describe("previewLines", () => {
  /** A run of `n` glyphs from `x`, `w` wide, on the band `top`–`top + 10`. */
  function runs(spec: readonly { x: number; w: number; top: number; text: string }[]) {
    const quads: number[] = [];
    const out: TextRunData[] = [];
    for (const s of spec) {
      const start = quads.length / 8;
      quads.push(s.x, s.top + 10, s.x + s.w, s.top + 10, s.x + s.w, s.top, s.x, s.top);
      out.push({
        text: s.text,
        size: 10,
        vertical: false,
        actualText: false,
        invisible: false,
        glyphRange: [start, start + 1],
      });
    }
    return previewLines(out, new Float32Array(quads));
  }

  it("joins runs on one band into one line", () => {
    const lines = runs([
      { x: 72, w: 40, top: 100, text: "[12]" },
      { x: 116, w: 100, top: 100, text: "Knuth, 1984" },
    ]);
    expect(lines.map((l) => [l.left, l.right, l.text])).toEqual([[72, 216, "[12] Knuth, 1984"]]);
  });

  it("keeps the two columns apart", () => {
    const lines = runs([
      { x: 72, w: 200, top: 100, text: "left" },
      { x: 316, w: 200, top: 100, text: "right" },
    ]);
    expect(lines.map((l) => l.text)).toEqual(["left", "right"]);
  });
});

describe("previewText", () => {
  it("joins the lines inside the crop and cuts at 300 characters", () => {
    const long = Array.from({ length: 20 }, (_, i) => line(72, 100 + i * 12, 290, "x".repeat(30)));
    const text = previewText(long, { x: 64, y: 92, width: 240, height: 300 });
    expect([text.length, text.endsWith("…")]).toEqual([300, true]);
  });

  it("leaves out lines outside the crop", () => {
    expect(previewText(TWO_COLUMNS, { x: 64, y: 98, width: 240, height: 12 })).toBe("[3] Left reference 3");
  });
});
