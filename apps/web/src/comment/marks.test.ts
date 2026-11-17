import type { PageAnnotation } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import {
  boxToUser,
  caretBox,
  colourName,
  colourValue,
  coversText,
  dragBox,
  type Frame,
  lineEnd,
  markSpec,
  type PageText,
  resizeOp,
  snapLines,
  textLines,
  toUser,
} from "./marks.ts";

/** A US Letter page, unturned, its crop box at the origin. */
const LETTER: Frame = { crop: [0, 0, 612, 792], rotate: 0, unit: 1 };

/** Two lines of text: three glyphs 10 pt wide at y 100–112, then two at y 130–142. */
const TEXT: PageText = {
  runs: [{ glyphRange: [0, 3] }, { glyphRange: [3, 5] }],
  quads: [...glyph(50, 100), ...glyph(60, 100), ...glyph(70, 100), ...glyph(50, 130), ...glyph(60, 130)],
};

/** A glyph's quad, 10 × 12 pt from (`x`, `y`). */
function glyph(x: number, y: number): number[] {
  return [x, y + 12, x + 10, y + 12, x + 10, y, x, y];
}

describe("the page frame", () => {
  it("maps page points to user space at every rotation", () => {
    expect(toUser(LETTER, [10, 20])).toEqual({ x: 10, y: 772 });
    expect(toUser({ ...LETTER, rotate: 90 }, [10, 20])).toEqual({ x: 20, y: 10 });
    expect(toUser({ ...LETTER, rotate: 180 }, [10, 20])).toEqual({ x: 602, y: 20 });
    expect(toUser({ ...LETTER, rotate: 270 }, [10, 20])).toEqual({ x: 592, y: 782 });
  });

  it("follows the crop box's origin and the user unit", () => {
    expect(toUser({ crop: [100, 100, 712, 892], rotate: 0, unit: 2 }, [10, 20])).toEqual({ x: 105, y: 882 });
  });

  it("normalises a box", () => {
    expect(boxToUser(LETTER, [10, 20, 30, 40])).toEqual({ x0: 10, y0: 752, x1: 30, y1: 772 });
  });
});

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

describe("specs", () => {
  it("writes a highlight's quads in user space", () => {
    const spec = markSpec({ type: "markup", kind: "highlight", lines: [[10, 20, 30, 40]] }, LETTER, {
      colour: [1, 1, 0],
      width: 1,
      opacity: 1,
    });
    expect(spec).toEqual({
      type: "textMarkup",
      kind: "highlight",
      quads: [
        [
          { x: 10, y: 772 },
          { x: 30, y: 772 },
          { x: 10, y: 752 },
          { x: 30, y: 752 },
        ],
      ],
    });
  });

  it("writes an arrow as a line with an open arrow at its end", () => {
    const spec = markSpec({ type: "line", from: [0, 0], to: [10, 0], arrow: true }, LETTER, {
      colour: [0, 0, 0],
      width: 2,
      opacity: 1,
    });
    expect(spec).toMatchObject({ type: "line", ends: ["none", "openArrow"], border: { width: 2 } });
  });

  it("resizes a square through its inner box", () => {
    const square = { id: "a9.0", subtype: "Square", geometry: { kind: "none" } } as unknown as PageAnnotation;
    const resized = { ...square, geometry: { kind: "inner", rect: [10, 20, 30, 40] } } as unknown as PageAnnotation;
    expect(resizeOp(square, resized, LETTER)).toEqual({
      op: "updateAnnot",
      args: { id: "a9.0", spec: { type: "shape", rect: { x0: 10, y0: 752, x1: 30, y1: 772 } } },
    });
  });

  it("names the colours this reader writes, and no others", () => {
    expect(colourName([...colourValue("red", "line")], "line")).toBe("red");
    expect(colourName([0.5, 0.5, 0.5], "line")).toBeUndefined();
  });
});
