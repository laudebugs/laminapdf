import { describe, expect, it } from "vitest";
import { handleFrame, movedFrame, textFrame } from "./drag.ts";
import {
  boxOf,
  contains,
  dragBox,
  frameOf,
  invert,
  isSkewed,
  keptOnPage,
  lineEnd,
  type Matrix,
  moved,
  multiply,
  nudgeOf,
  reshaped,
  resized,
  sizeOf,
  snapMove,
  turnedTo,
  turnPoint,
} from "./geometry.ts";

const near = (a: number, b: number, digits = 6) => expect(a).toBeCloseTo(b, digits);

describe("frames", () => {
  it("make a rectangle whose size and turn read back", () => {
    const frame = frameOf(10, 20, 100, 40, 30);
    const { width, height, degrees } = sizeOf(frame);
    near(width, 100);
    near(height, 40);
    near(degrees, 30);
  });

  it("turn about their centre", () => {
    const flat = frameOf(10, 20, 100, 40);
    const turned = frameOf(10, 20, 100, 40, 90);
    // The centre is (60, 40) either way.
    near(turned[0] / 2 + turned[2] / 2 + turned[4], 60);
    near(turned[1] / 2 + turned[3] / 2 + turned[5], 40);
    expect(flat).toEqual([100, 0, 0, 40, 10, 20]);
  });

  it("invert and multiply give the identity", () => {
    const frame = frameOf(5, 7, 30, 20, 25);
    const inverse = invert(frame) as Matrix;
    const product = multiply(frame, inverse);
    for (const [i, v] of [1, 0, 0, 1, 0, 0].entries()) near(product[i] ?? 0, v);
  });

  it("find a point inside, with slop outside", () => {
    const frame = frameOf(10, 10, 100, 0.5);
    expect(contains(frame, { x: 50, y: 10.25 })).toBe(true);
    expect(contains(frame, { x: 50, y: 14 })).toBe(false);
    expect(contains(frame, { x: 50, y: 14 }, 5)).toBe(true);
  });
});

describe("resizing by a handle", () => {
  const frame = frameOf(100, 100, 200, 100);

  it("keeps the opposite corner where it was", () => {
    const next = resized(frame, "se", 50, 20);
    expect(sizeOf(next).width).toBe(250);
    expect(sizeOf(next).height).toBe(120);
    expect([next[4], next[5]]).toEqual([100, 100]);
  });

  it("moves the origin with a west or north handle", () => {
    const next = resized(frame, "nw", -20, -10);
    expect([next[4], next[5]]).toEqual([80, 90]);
    expect(sizeOf(next).width).toBe(220);
  });

  it("changes one side only with a side handle", () => {
    const next = resized(frame, "e", 30, 99);
    expect(sizeOf(next).height).toBe(100);
    expect(sizeOf(next).width).toBe(230);
  });

  it("holds the proportions on a corner with Shift", () => {
    const next = resized(frame, "se", 100, 0, true);
    const { width, height } = sizeOf(next);
    near(width / height, 2);
  });

  it("never gets smaller than a few points", () => {
    expect(sizeOf(resized(frame, "e", -500, 0)).width).toBe(4);
  });

  it("follows the frame's own turn", () => {
    const turned = frameOf(100, 100, 200, 100, 90);
    // Dragging down the page lengthens a frame turned a quarter, whose width now runs down.
    const next = resized(turned, "e", 0, 40);
    near(sizeOf(next).width, 240);
  });
});

describe("turning", () => {
  it("points the top of the frame at the pointer", () => {
    const frame = frameOf(100, 100, 100, 100);
    const next = turnedTo(frame, { x: 250, y: 150 });
    near(sizeOf(next).degrees, 90);
  });

  it("rounds to 15 degrees with Shift", () => {
    const frame = frameOf(100, 100, 100, 100);
    const next = turnedTo(frame, { x: 250, y: 140 }, true);
    expect(sizeOf(next).degrees % 15).toBe(0);
  });

  it("puts the handle beyond the top edge", () => {
    const frame = frameOf(100, 100, 100, 100);
    const p = turnPoint(frame, 24);
    near(p.x, 150);
    near(p.y, 76);
  });
});

describe("snapping", () => {
  const page = { width: 600, height: 800 };

  it("pulls an edge onto the margin", () => {
    const snapped = snapMove([40, 100, 140, 160], 0, 0, page, []);
    expect(snapped.dx).toBe(-4);
    expect(snapped.guides).toEqual([{ axis: "x", at: 36, kind: "margin" }]);
  });

  it("pulls the middle onto the page's centre", () => {
    const snapped = snapMove([253, 100, 343, 160], 0, 0, page, []);
    expect(snapped.dx).toBe(2);
    expect(snapped.guides[0]?.kind).toBe("centre");
  });

  it("pulls onto another object's edge", () => {
    const snapped = snapMove([100, 300, 150, 340], 0, 1, page, [[200, 100, 300, 342]]);
    expect(snapped.dy).toBe(2);
    expect(snapped.guides.some((g) => g.kind === "object" && g.axis === "y")).toBe(true);
  });

  it("leaves a move alone when nothing is near", () => {
    const snapped = snapMove([100, 300, 150, 340], 7, 9, page, []);
    expect([snapped.dx, snapped.dy]).toEqual([7, 9]);
    expect(snapped.guides).toEqual([]);
  });
});

describe("keys and drags", () => {
  it("nudge by 1 point, or 10 with Shift", () => {
    expect(nudgeOf("ArrowLeft", false)).toEqual({ x: -1, y: 0 });
    expect(nudgeOf("ArrowDown", true)).toEqual({ x: 0, y: 10 });
    expect(nudgeOf("a", false)).toBeUndefined();
  });

  it("drag a box either way, square with Shift", () => {
    expect(dragBox({ x: 100, y: 100 }, { x: 40, y: 160 })).toEqual([40, 100, 100, 160]);
    expect(dragBox({ x: 100, y: 100 }, { x: 140, y: 110 }, true)).toEqual([100, 100, 140, 140]);
  });

  it("draw a line at multiples of 45 degrees with Shift", () => {
    const end = lineEnd({ x: 0, y: 0 }, { x: 100, y: 10 }, true);
    near(end.y, 0);
    near(end.x, Math.hypot(100, 10));
  });

  it("keeps a box on the page", () => {
    expect(keptOnPage([-10, 780, 90, 830], { width: 600, height: 800 })).toEqual([0, 750, 100, 800]);
  });

  it("moves a frame without changing its size", () => {
    const frame = frameOf(10, 10, 50, 50, 20);
    const next = moved(frame, 5, -5);
    expect(sizeOf(next)).toEqual(sizeOf(frame));
    expect(next[4]).toBe(frame[4] + 5);
  });
});

describe("what a drag makes of a frame", () => {
  const page = { width: 600, height: 800 };
  const frame = frameOf(100, 100, 100, 50);

  it("moves by the drag and snaps with its guides", () => {
    const free = movedFrame(frame, { x: 0, y: 0 }, { x: 50, y: 20 }, page, [], true);
    expect([free.frame[4], free.frame[5], free.guides.length]).toEqual([150, 120, 0]);
    // 100 + 66 = 166 puts the left edge 4 points past the margin at 36 + 130: no; the right edge 266 + 34 is 300.
    const snapped = movedFrame(frame, { x: 0, y: 0 }, { x: -62, y: 0 }, page, [], false);
    expect(snapped.frame[4]).toBe(36);
    expect(snapped.guides).toEqual([{ axis: "x", at: 36, kind: "margin" }]);
  });

  it("changes only a text box's width, the west handle moving its left edge", () => {
    const box = frameOf(100, 100, 200, 30);
    expect(textFrame(box, "e", { x: 0, y: 0 }, { x: 40, y: 99 })).toEqual([240, 0, 0, 30, 100, 100]);
    expect(textFrame(box, "w", { x: 0, y: 0 }, { x: -40, y: 0 })).toEqual([240, 0, 0, 30, 60, 100]);
    expect(textFrame(box, "e", { x: 0, y: 0 }, { x: -500, y: 0 })[0]).toBe(24);
  });

  it("resizes, turns or sets a width by the handle", () => {
    const at = { x: 0, y: 0 };
    expect(sizeOf(handleFrame(frame, false, "se", at, { x: 20, y: 10 }, false)).width).toBe(120);
    expect(sizeOf(handleFrame(frame, true, "e", at, { x: 20, y: 10 }, false)).height).toBe(50);
    near(sizeOf(handleFrame(frame, false, "turn", at, { x: 400, y: 125 }, false)).degrees, 90);
  });
});

describe("reshaping a frame", () => {
  it("sets the fields boxOf reads and keeps the rest", () => {
    const frame = frameOf(100, 100, 80, 40, 30);
    const next = boxOf(reshaped(frame, { x: 10, width: 200 }));
    expect(next.x).toBeCloseTo(10);
    expect(next.y).toBeCloseTo(100);
    expect(next.width).toBeCloseTo(200);
    expect(next.height).toBeCloseTo(40);
    expect(next.rotate).toBeCloseTo(30);
  });

  it("turns a mirrored frame and leaves it mirrored", () => {
    const mirrored: Matrix = [100, 0, 0, -50, 20, 90];
    const turned = reshaped(mirrored, { rotate: 25 });
    expect(turned[0] * turned[3] - turned[1] * turned[2]).toBeLessThan(0);
    expect(sizeOf(turned).degrees).toBeCloseTo(25);
    expect(sizeOf(turned).height).toBeCloseTo(50);
  });

  it("knows a skewed frame", () => {
    expect(isSkewed(frameOf(0, 0, 10, 10, 20))).toBe(false);
    expect(isSkewed([10, 0, 5, 10, 0, 0])).toBe(true);
  });

  it("doesn't scale a flat frame's other side when it is resized", () => {
    const line = frameOf(10, 50, 100, 0.001);
    const next = resized(line, "se", 40, 30);
    expect(sizeOf(next).width).toBeCloseTo(140);
    expect(sizeOf(next).height).toBeCloseTo(0.001);
  });

  it("keeps a turned frame's turn when it moves", () => {
    const frame = frameOf(100, 100, 80, 40, 30);
    expect(boxOf(reshaped(frame, { x: 5, y: 6 })).rotate).toBeCloseTo(30);
  });
});
