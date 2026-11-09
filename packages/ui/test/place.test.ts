import { describe, expect, it } from "vitest";
import { type Box, intersect, placeAbove, selectionBox } from "../src/overlays/place.ts";

const viewport = { top: 0, left: 0, width: 800, height: 600 };
const line = (top: number, left = 300) => ({ top, left, width: 200, height: 20 });

describe("placeAbove", () => {
  it("centres the layer over the anchor, gap above it", () => {
    expect(placeAbove(line(300), 100, 40, viewport, 8, 8)).toEqual({ top: 252, left: 350, side: "above" });
  });

  it("flips below when there is no room above", () => {
    expect(placeAbove(line(20), 100, 40, viewport, 8, 8)).toEqual({ top: 48, left: 350, side: "below" });
  });

  it("stays above when neither side has room, kept inside the bounds", () => {
    const tall = { top: 0, left: 0, width: 800, height: 100 };
    expect(placeAbove(line(40), 100, 90, tall, 8, 8)).toMatchObject({ top: 8, side: "above" });
  });

  it("shifts to stay inside the bounds at either edge", () => {
    expect(placeAbove(line(300, -150), 100, 40, viewport, 8, 8)?.left).toBe(8);
    expect(placeAbove(line(300, 760), 100, 40, viewport, 8, 8)?.left).toBe(692);
  });

  it("places nothing when the anchor has left the bounds, so the layer can hide", () => {
    expect(placeAbove(line(-1700), 100, 40, viewport, 8, 8)).toBeNull();
    expect(placeAbove(line(900), 100, 40, viewport, 8, 8)).toBeNull();
    expect(placeAbove(line(300, 900), 100, 40, viewport, 8, 8)).toBeNull();
    // Partly inside still counts: the toolbar stays with the visible part of the selection.
    expect(placeAbove(line(-10), 100, 40, viewport, 8, 8)).not.toBeNull();
  });

  it("keeps inside a canvas that isn't the whole viewport", () => {
    const canvas = { top: 100, left: 200, width: 400, height: 400 };
    expect(placeAbove(line(110, 200), 100, 40, canvas, 8, 8)).toEqual({ top: 138, left: 250, side: "below" });
  });
});

describe("intersect", () => {
  it("keeps the part of a box inside another", () => {
    const canvas = { top: 100, left: 200, width: 1000, height: 300 };
    expect(intersect(viewport, canvas)).toEqual({ top: 100, left: 200, width: 600, height: 300 });
  });

  it("is empty when the boxes don't meet, so nothing can be placed in it", () => {
    const away = { top: 700, left: 900, width: 50, height: 50 };
    const empty = intersect(viewport, away);
    expect(empty).toEqual({ top: 700, left: 900, width: 0, height: 0 });
    expect(placeAbove(line(300), 100, 40, empty, 8, 8)).toBeNull();
  });
});

describe("selectionBox", () => {
  const overlaps = (a: Box, b: Box) =>
    a.left < b.left + b.width && b.left < a.left + a.width && a.top < b.top + b.height && b.top < a.top + a.height;

  it("covers every rectangle and skips empty ones", () => {
    const box = selectionBox([line(100, 300), { top: 120, left: 50, width: 400, height: 20 }, line(0, 0)]);
    expect(box).toEqual({ top: 0, left: 0, width: 500, height: 140 });
    expect(selectionBox([{ top: 5, left: 5, width: 0, height: 10 }])).toBeNull();
  });

  // A toolbar 220 × 38 against selections of one to three lines across the viewport, at its edges
  // too: above, flipped below or shifted sideways, it never covers a selected line.
  const positions = Array.from({ length: 20 }, (_, k) => ({
    top: k < 4 ? k * 6 : k >= 16 ? 600 - 20 * ((k % 3) + 1) - (k - 16) * 4 : 30 + k * 30,
    left: k % 2 === 0 ? (k * 37) % 700 : k % 3 === 0 ? -40 : 620,
    lines: (k % 3) + 1,
  }));
  it.each(positions)("never covers the selection at top $top, left $left ($lines lines)", ({ top, left, lines }) => {
    const rects = Array.from({ length: lines }, (_, i) => ({
      top: top + i * 20,
      left: i === 0 ? left : 40,
      width: i === lines - 1 ? 180 : 400,
      height: 18,
    }));
    const anchor = selectionBox(rects) as Box;
    const place = placeAbove(anchor, 220, 38, viewport, 8, 8);
    expect(place).not.toBeNull();
    const toolbar = { top: place?.top ?? 0, left: place?.left ?? 0, width: 220, height: 38 };
    for (const r of rects) expect(overlaps(toolbar, r), JSON.stringify({ toolbar, r })).toBe(false);
  });
});
