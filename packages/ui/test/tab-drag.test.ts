// Where a dragged tab lands: `reordered` puts it at the insertion point the strip showed, and says
// so only when the order changes.

import { describe, expect, it } from "vitest";
import { reordered } from "../src/reader/tab-drag.ts";

const TABS = ["a", "b", "c", "d"];

describe("a dragged tab's new place", () => {
  it("is the insertion point, counted before the move, when it moves right", () => {
    expect(reordered(TABS, "a", 3)).toEqual(["b", "c", "a", "d"]);
  });

  it("is the insertion point when it moves left", () => {
    expect(reordered(TABS, "d", 1)).toEqual(["a", "d", "b", "c"]);
  });

  it("can be the end of the strip", () => {
    expect(reordered(TABS, "b", 4)).toEqual(["a", "c", "d", "b"]);
  });

  it("is no change when the tab would land where it is", () => {
    expect([reordered(TABS, "b", 1), reordered(TABS, "b", 2)]).toEqual([undefined, undefined]);
  });

  it("is no change for a tab the strip doesn't have", () => {
    expect(reordered(TABS, "z", 0)).toBeUndefined();
  });
});
