import type { PageAnnotation } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { colourName, colourValue } from "./marks.ts";
import {
  borderPatch,
  borderStyle,
  boxToUser,
  endsPatch,
  fillPatch,
  kindOf,
  markSpec,
  resizeOp,
  subtypeOf,
  type ToUser,
} from "./ops.ts";

/**
 * A US Letter page, unturned, its crop box at the origin: user space is page space flipped. The
 * viewer's pageToUser, which Comment mode passes, is held to the model by its own shared cases.
 */
const LETTER: ToUser = ([x, y]) => ({ x, y: 792 - y });

describe("a page's frame", () => {
  it("normalises a box in user space", () => {
    expect(boxToUser(LETTER, [10, 20, 30, 40])).toEqual({ x0: 10, y0: 752, x1: 30, y1: 772 });
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
    expect(resizeOp(square, resized, LETTER)).toMatchObject({
      op: "updateAnnot",
      args: { id: "a9.0", spec: { type: "shape", rect: { x0: 10, y0: 752, x1: 30, y1: 772 } } },
    });
  });

  it("refuses a resize for a kind it can't resize", () => {
    const note = { id: "a9.0", subtype: "Text", geometry: { kind: "none" } } as unknown as PageAnnotation;
    expect(resizeOp(note, note, LETTER)).toBeUndefined();
  });

  it("names the colours this reader writes, and no others", () => {
    expect(colourName([...colourValue("red", "line")], "line")).toBe("red");
    expect(colourName([0.5, 0.5, 0.5], "line")).toBeUndefined();
  });
});

describe("the kinds", () => {
  it("give each mark's subtype, and the inspector's patches only to the kinds that take them", () => {
    expect(subtypeOf({ type: "poly", kind: "polyLine", vertices: [] })).toBe("PolyLine");
    expect(kindOf("Circle")).toMatchObject({ patch: "shape", closed: true, family: "line" });
    expect(fillPatch("Line", null)).toBeUndefined();
    expect(fillPatch("Polygon", [1, 0, 0])).toEqual({ type: "poly", interior: [1, 0, 0] });
    expect(endsPatch("PolyLine", ["none", "circle"])).toEqual({ type: "poly", ends: ["none", "circle"] });
    expect(endsPatch("Square", ["none", "circle"])).toBeUndefined();
    expect(borderPatch("Ink", { width: 3, style: borderStyle("solid") })).toEqual({ type: "ink", width: 3 });
    expect(borderPatch("Highlight", { width: 3, style: borderStyle("solid") })).toBeUndefined();
  });
});
