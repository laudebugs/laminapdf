import { describe, expect, it } from "vitest";
import { gapOf, moveToGap, pageRanges, stepGap } from "../src/organise/moves.js";

const ORDER = ["a", "b", "c", "d", "e"];

describe("moveToGap", () => {
  it("puts the selection after the page before the gap", () => {
    const move = moveToGap(ORDER, [0, 1], 4);
    expect(move).toEqual({ pages: ["a", "b"], to: { after: "d" }, order: ["c", "d", "a", "b", "e"], moved: [2, 3] });
  });

  it("goes to the start when no other page is before the gap", () => {
    expect(moveToGap(ORDER, [3], 0)?.to).toBe("start");
    // A gap among selected pages at the front: still the start.
    expect(moveToGap(ORDER, [0, 3], 1)?.order).toEqual(["a", "d", "b", "c", "e"]);
  });

  it("gathers pages that weren't neighbours", () => {
    expect(moveToGap(ORDER, [0, 2, 4], 5)?.order).toEqual(["b", "d", "a", "c", "e"]);
  });

  it("is nothing when the pages wouldn't move", () => {
    expect(moveToGap(ORDER, [1, 2], 1)).toBeUndefined();
    expect(moveToGap(ORDER, [1, 2], 3)).toBeUndefined();
    expect(moveToGap(ORDER, [], 3)).toBeUndefined();
  });
});

describe("gapOf and stepGap", () => {
  it("names Move to page…'s positions", () => {
    expect([gapOf("after", 2, 5), gapOf("before", 2, 5), gapOf("start", 2, 5), gapOf("end", 2, 5)]).toEqual([
      3, 2, 0, 5,
    ]);
  });

  it("moves the selection one page or one row, within the list", () => {
    expect(stepGap([2, 3], -1, 5)).toBe(1);
    expect(stepGap([2, 3], 1, 5)).toBe(5);
    expect(stepGap([0], -1, 5)).toBe(0);
    expect(stepGap([1], 4, 5)).toBe(5);
    expect(stepGap([], 1, 5)).toBeUndefined();
  });
});

describe("pageRanges", () => {
  it("names neighbouring pages as ranges, by label", () => {
    expect(pageRanges([4, 5], (i) => String(i + 1))).toBe("5–6");
    expect(pageRanges([0, 2, 3, 4], (i) => String(i + 1))).toBe("1, 3–5");
    expect(pageRanges([1], (i) => ["i", "ii"][i] ?? "")).toBe("ii");
  });
});
