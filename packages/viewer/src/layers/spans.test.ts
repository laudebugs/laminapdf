import { describe, expect, it } from "vitest";
import type { TextRunData } from "../protocol.ts";
import { layoutSpans, type SpanSpec } from "./spans.ts";

/** Quads for glyphs `width` wide and `height` tall from `x` along a baseline at `y` (upright, y down). */
function upright(xs: readonly number[], y: number, width: number, height: number): number[] {
  return xs.flatMap((x) => [x, y, x + width, y, x + width, y - height, x, y - height]);
}

const run = (text: string, glyphs: readonly [number, number], extra: Partial<TextRunData> = {}): TextRunData => ({
  text,
  size: 10,
  vertical: false,
  actualText: false,
  invisible: false,
  glyphRange: glyphs,
  ...extra,
});

const round = (spec: SpanSpec) => ({
  left: Math.round(spec.left * 100) / 100,
  top: Math.round(spec.top * 100) / 100,
  width: Math.round(spec.width * 100) / 100,
  height: Math.round(spec.height * 100) / 100,
  angle: Math.round((spec.angle * 180) / Math.PI),
  rtl: spec.rtl,
});

describe("layoutSpans", () => {
  it("covers an upright run's glyphs from its first glyph's top-left", () => {
    const quads = new Float32Array(upright([100, 106, 112], 200, 6, 10));
    expect(round(layoutSpans([run("abc", [0, 3])], quads)[0] as SpanSpec)).toEqual({
      left: 100,
      top: 190,
      width: 18,
      height: 10,
      angle: 0,
      rtl: false,
    });
  });

  it("turns a run drawn at 90° with its baseline", () => {
    // A baseline running down the page: each glyph's bottom edge points along +y, its top to +x.
    const quad = (y: number) => [50, y, 50, y + 6, 60, y + 6, 60, y];
    const quads = new Float32Array([...quad(100), ...quad(106)]);
    expect(round(layoutSpans([run("ab", [0, 2])], quads)[0] as SpanSpec)).toEqual({
      left: 60,
      top: 100,
      width: 12,
      height: 10,
      angle: 90,
      rtl: false,
    });
  });

  it("reads a run right to left by its first letter, whatever order its glyphs were drawn in", () => {
    const quads = new Float32Array(upright([100, 106, 112, 130, 136, 150], 200, 6, 10));
    const spans = layoutSpans([run("שלם", [0, 3]), run("«ab", [3, 5]), run("1 ש", [5, 6])], quads);
    expect(spans.map((s) => [s.rtl, round(s).left])).toEqual([
      [true, 100],
      [false, 130],
      [true, 150],
    ]);
  });

  it("lays a vertical run down the page and keeps its MCID", () => {
    const quads = new Float32Array([...upright([100], 110, 10, 10), ...upright([100], 122, 10, 10)]);
    const [span] = layoutSpans([run("縦書", [0, 2], { vertical: true, mcid: 4 })], quads);
    expect([span && round(span).angle, span && round(span).width, span?.mcid]).toEqual([90, 22, 4]);
  });
});
