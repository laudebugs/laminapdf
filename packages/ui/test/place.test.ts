import { describe, expect, it } from "vitest";
import { placeAbove } from "../src/overlays/place.ts";

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
