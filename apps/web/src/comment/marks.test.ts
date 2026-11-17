import { describe, expect, it } from "vitest";
import { caretBox, coversText, dragBox, lineEnd, type PageText, snapLines, textLines } from "./marks.ts";

/** Two lines of text: three glyphs 10 pt wide at y 100–112, then two at y 130–142. */
const TEXT: PageText = {
  runs: [{ glyphRange: [0, 3] }, { glyphRange: [3, 5] }],
  quads: [...glyph(50, 100), ...glyph(60, 100), ...glyph(70, 100), ...glyph(50, 130), ...glyph(60, 130)],
};

/** A glyph's quad, 10 × 12 pt from (`x`, `y`). */
function glyph(x: number, y: number): number[] {
  return [x, y + 12, x + 10, y + 12, x + 10, y, x, y];
}

describe("drawing", () => {
  it("draws a box from corner to corner, and a square with Shift", () => {
    expect(dragBox([10, 10], [40, 20], false)).toEqual([10, 10, 40, 20]);
    expect(dragBox([40, 20], [10, 10], false)).toEqual([10, 10, 40, 20]);
    expect(dragBox([10, 10], [40, 20], true)).toEqual([10, 10, 40, 40]);
  });

  it("turns a line to the nearest 45° with Shift", () => {
    const [x, y] = lineEnd([0, 0], [10, 1], true);
    expect([Math.round(x), Math.round(y)]).toEqual([10, 0]);
    expect(lineEnd([0, 0], [10, 1], false)).toEqual([10, 1]);
  });
});

describe("text", () => {
  it("marks the glyphs from the press to the release, one box per line", () => {
    expect(textLines(TEXT, [62, 105], [55, 135])).toEqual([
      [60, 100, 80, 112],
      [50, 130, 60, 142],
    ]);
  });

  it("marks nothing for a press far from any text", () => {
    expect(textLines(TEXT, [400, 400], [55, 135])).toEqual([]);
  });

  it("snaps a selection's boxes to the glyphs inside them", () => {
    expect(snapLines(TEXT, [[48, 98, 72, 114]])).toEqual([[50, 100, 70, 112]]);
  });

  it("puts a caret on the glyph edge nearest the click, in the line's lower half", () => {
    expect(caretBox(TEXT, [59, 104])).toEqual([56, 106, 64, 114]);
  });
});

describe("the covered-text notice", () => {
  const style = { colour: [0, 0, 0] as const, width: 1, opacity: 1, fill: [1, 1, 0] as const };

  it("sees an opaque filled rectangle over text", () => {
    expect(coversText({ type: "shape", kind: "square", box: [40, 90, 90, 120] }, style, TEXT)).toBe(true);
  });

  it("ignores a rectangle without a fill, a see-through one, and one over empty space", () => {
    const over = { type: "shape", kind: "square", box: [40, 90, 90, 120] } as const;
    const { fill: _, ...outline } = style;
    expect(coversText(over, outline, TEXT)).toBe(false);
    expect(coversText(over, { ...style, opacity: 0.5 }, TEXT)).toBe(false);
    expect(coversText({ type: "shape", kind: "square", box: [300, 300, 400, 400] }, style, TEXT)).toBe(false);
  });

  it("sees text inside a polygon, and invisible text never counts", () => {
    const triangle = {
      type: "poly",
      kind: "polygon",
      vertices: [
        [40, 90],
        [100, 90],
        [40, 150],
      ],
    } as const;
    expect(coversText(triangle, style, TEXT)).toBe(true);
    const hidden = { ...TEXT, runs: TEXT.runs.map((r) => ({ ...r, invisible: true })) };
    expect(coversText(triangle, style, hidden)).toBe(false);
  });
});
