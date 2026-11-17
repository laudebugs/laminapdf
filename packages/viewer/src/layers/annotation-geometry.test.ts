import type { PageAnnotation } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import {
  annotationPreview,
  handlePoints,
  hitTest,
  pageMove,
  resizeBox,
  resizedAnnotation,
  shownAnnotations,
  unionBox,
  userMove,
} from "./annotation-geometry.js";

/** A record with the fields the layer reads; the rest as a reader leaves them. */
function record(
  index: number,
  subtype: string,
  rect: [number, number, number, number] | undefined,
  geometry: PageAnnotation["geometry"] = { kind: "none" },
  flags = 4,
): PageAnnotation {
  return {
    id: `a${index + 10}.0`,
    index,
    subtype,
    flags,
    color: [1, 0, 0],
    grouped: false,
    hasAppearance: true,
    hasRichText: false,
    geometry,
    ...(rect === undefined ? {} : { rect }),
  } as PageAnnotation;
}

describe("shownAnnotations", () => {
  it("leaves out pop-ups, links, widgets, hidden ones and those without a rectangle", () => {
    const shown = shownAnnotations([
      record(0, "Square", [10, 10, 20, 20]),
      record(1, "Popup", [10, 30, 20, 40]),
      record(2, "Link", [10, 50, 20, 60]),
      record(3, "Widget", [10, 70, 20, 80]),
      record(4, "Square", [10, 90, 20, 100], { kind: "none" }, 2),
      record(5, "Square", [10, 110, 20, 120], { kind: "none" }, 32),
      record(6, "Text", undefined),
    ]);
    expect(shown.map(({ annotation }) => annotation.index)).toEqual([0]);
    expect(shown[0]?.box).toEqual([10, 10, 20, 20]);
  });

  it("puts them in reading order: top to bottom, then left to right on one line", () => {
    const shown = shownAnnotations([
      record(0, "Square", [200, 100, 220, 120]),
      record(1, "Square", [10, 101, 30, 120]),
      record(2, "Square", [10, 10, 30, 30]),
    ]);
    expect(shown.map(({ annotation }) => annotation.index)).toEqual([2, 1, 0]);
  });
});

describe("hitTest", () => {
  const ink = record(0, "Ink", [0, 0, 100, 100], { kind: "ink", strokes: [[0, 0, 100, 100]] });
  const square = record(1, "Square", [40, 40, 60, 60]);
  const highlight = record(2, "Highlight", [0, 200, 100, 230], {
    kind: "quads",
    quads: [0, 200, 100, 200, 0, 210, 100, 210, 0, 220, 100, 220, 0, 230, 100, 230],
  });

  it("finds an ink stroke near its line, not anywhere in its box", () => {
    expect(hitTest([ink], 50, 51, 3)?.index).toBe(0);
    expect(hitTest([ink], 90, 10, 3)).toBeUndefined();
  });

  it("takes the annotation drawn last where two overlap", () => {
    expect(hitTest([ink, square], 50, 50, 3)?.index).toBe(1);
    expect(hitTest([square, ink], 50, 50, 3)?.index).toBe(1);
  });

  it("finds a markup inside one of its areas only", () => {
    expect(hitTest([highlight], 50, 225, 0)?.index).toBe(2);
    expect(hitTest([highlight], 50, 215, 0)).toBeUndefined();
  });
});

describe("the selection's box and handles", () => {
  it("bounds every annotation, moved", () => {
    const box = unionBox([record(0, "Square", [10, 10, 20, 20]), record(1, "Square", [30, 5, 40, 15])], {
      dx: 1,
      dy: 2,
    });
    expect(box).toEqual([11, 7, 41, 22]);
  });

  it("has eight handles: corners, then the middles of the edges", () => {
    expect(handlePoints([0, 0, 10, 20])).toEqual([
      [0, 0],
      [10, 0],
      [10, 20],
      [0, 20],
      [5, 0],
      [10, 10],
      [5, 20],
      [0, 10],
    ]);
  });
});

describe("moves", () => {
  it("turns a pointer's move on a turned page into page points", () => {
    // 20 CSS px right and 10 down at 2 px per point.
    expect(pageMove(20, 10, 2, 0)).toEqual({ dx: 10, dy: 5 });
    // A quarter turn clockwise: the page's x axis points down the screen.
    expect(pageMove(20, 10, 2, 90)).toEqual({ dx: 5, dy: -10 });
    expect(pageMove(20, 10, 2, 180)).toEqual({ dx: -10, dy: -5 });
    expect(pageMove(20, 10, 2, 270)).toEqual({ dx: -5, dy: 10 });
  });

  it("turns a move in page points into user space for each /Rotate", () => {
    const move = { dx: 3, dy: 4 };
    expect(userMove(move, 0)).toEqual({ dx: 3, dy: -4 });
    expect(userMove(move, 90)).toEqual({ dx: 4, dy: 3 });
    expect(userMove(move, 180)).toEqual({ dx: -3, dy: 4 });
    expect(userMove(move, 270)).toEqual({ dx: -4, dy: -3 });
    expect(userMove(move, -90)).toEqual({ dx: -4, dy: -3 });
  });
});

describe("annotationPreview", () => {
  it("fills a highlight's areas where it moved to", () => {
    const highlight = record(0, "Highlight", [0, 0, 10, 5], { kind: "quads", quads: [0, 0, 10, 0, 0, 5, 10, 5] });
    const [shape] = annotationPreview(highlight, { dx: 1, dy: 2 });
    expect(shape).toMatchObject({ d: "M1 2 L11 2 L11 7 L1 7 Z", fill: "rgb(255 0 0)" });
  });

  it("strokes ink and outlines a square in its own colour", () => {
    const ink = record(0, "Ink", [0, 0, 10, 10], { kind: "ink", strokes: [[0, 0, 10, 10]] });
    expect(annotationPreview(ink, { dx: 0, dy: 0 })).toEqual([
      { d: "M0 0 L10 10", stroke: "rgb(255 0 0)", width: 1.5, opacity: 1 },
    ]);
    const square = record(1, "Square", [0, 0, 10, 10]);
    expect(annotationPreview(square, { dx: 5, dy: 0 })[0]?.d).toBe("M5 0 L15 0 L15 10 L5 10 Z");
  });
});

describe("resizeBox", () => {
  const box = [10, 20, 50, 40] as const;

  it("moves the edges a corner joins, or the one edge a middle handle sits on", () => {
    expect(resizeBox(box, 2, { dx: 10, dy: 5 })).toEqual([10, 20, 60, 45]);
    expect(resizeBox(box, 0, { dx: -5, dy: -5 })).toEqual([5, 15, 50, 40]);
    expect(resizeBox(box, 5, { dx: 10, dy: 99 })).toEqual([10, 20, 60, 40]);
    expect(resizeBox(box, 4, { dx: 99, dy: -10 })).toEqual([10, 10, 50, 40]);
  });

  it("never turns the box inside out", () => {
    expect(resizeBox(box, 2, { dx: -100, dy: -100 })).toEqual([10, 20, 12, 22]);
  });

  it("keeps the proportions with Shift: a corner about the opposite corner, an edge about the middle across", () => {
    // 40 × 20 doubled in width asks for 80 × 40.
    expect(resizeBox(box, 2, { dx: 40, dy: 0 }, true)).toEqual([10, 20, 90, 60]);
    expect(resizeBox(box, 0, { dx: -40, dy: 0 }, true)).toEqual([-30, 0, 50, 40]);
    expect(resizeBox(box, 5, { dx: 40, dy: 0 }, true)).toEqual([10, 10, 90, 50]);
  });
});

describe("resizedAnnotation", () => {
  it("stretches the rectangle and the points with the box", () => {
    const ink = record(0, "Ink", [10, 10, 30, 20], { kind: "ink", strokes: [[10, 10, 30, 20]] });
    const resized = resizedAnnotation(ink, [10, 10, 30, 20], [10, 10, 50, 40]);
    expect(resized.rect).toEqual([10, 10, 50, 40]);
    expect(resized.geometry).toEqual({ kind: "ink", strokes: [[10, 10, 50, 40]] });
  });
});
