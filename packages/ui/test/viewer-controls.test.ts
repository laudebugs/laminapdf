import { describe, expect, it } from "vitest";
import { MAX_ZOOM, MIN_ZOOM, pageFromInput, zoomIn, zoomOut } from "../src/reader/viewer-controls.ts";

describe("zoom steps", () => {
  it("step to the next percentage up or down", () => {
    expect([zoomIn(100), zoomIn(110), zoomOut(100), zoomOut(110)]).toEqual([110, 125, 90, 100]);
  });

  it("step from a fit mode's percentage, which is rarely a step", () => {
    expect([zoomIn(118.4), zoomOut(118.4)]).toEqual([125, 110]);
  });

  it("stop at the viewer's range", () => {
    expect([zoomIn(MAX_ZOOM), zoomOut(MIN_ZOOM), zoomOut(12)]).toEqual([MAX_ZOOM, MIN_ZOOM, MIN_ZOOM]);
  });
});

describe("pageFromInput", () => {
  it("reads a page number from 1 to the page count as a 0-based page", () => {
    expect([pageFromInput("1", 24), pageFromInput(" 24 ", 24)]).toEqual([0, 23]);
  });

  it("names no page for a number out of range, a fraction, a sign or nothing", () => {
    expect(["0", "25", "2.5", "-3", "", "  ", "two"].map((text) => pageFromInput(text, 24))).toEqual(
      Array(7).fill(undefined),
    );
  });

  it("reads a page label first, whatever its case, then falls back to the number", () => {
    const labels = [...Array.from({ length: 21 }, (_, n) => String(n + 1)), "A-1", "A-2", "A-3"];
    expect([
      pageFromInput("a-2", 24, labels),
      pageFromInput("A-3", 24, labels),
      pageFromInput("3", 24, labels),
    ]).toEqual([22, 23, 2]);
  });

  it("reads a label like 'iv' that no page number spells", () => {
    expect(pageFromInput("IV", 6, ["i", "ii", "iii", "iv", "1", "2"])).toBe(3);
  });
});
