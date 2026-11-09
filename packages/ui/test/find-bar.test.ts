// The find bar's count and note, in the R05 wording.
import { describe, expect, it } from "vitest";
import { type FindSlice, findCount, findNote } from "../src/index.ts";

const state = (s: Partial<FindSlice>): FindSlice => ({
  query: "repairs",
  options: {},
  total: 0,
  current: undefined,
  searched: 0,
  pageCount: 24,
  done: false,
  pagesWithoutText: 0,
  ...s,
});

describe("findCount", () => {
  it.each([
    [{ query: "" }, ""],
    [{}, "…"],
    [{ total: 40 }, "40+"],
    [{ total: 9, current: 2, done: true }, "3 of 9"],
    [{ total: 9, done: true }, "1 of 9"],
    [{ done: true }, "No matches"],
  ] as const)("%o → %s", (s, want) => {
    expect(findCount(state(s))).toBe(want);
  });
});

describe("findNote", () => {
  it("says how far the search has got, then that it searched every page", () => {
    expect([findNote(state({ searched: 14 })), findNote(state({ done: true }))]).toEqual([
      "Searching… 14 of 24 pages",
      "Searched all 24 pages",
    ]);
  });

  it("adds the pages without text", () => {
    expect([
      findNote(state({ done: true, pagesWithoutText: 4 })),
      findNote(state({ done: true, pagesWithoutText: 1 })),
    ]).toEqual([
      "Searched all 24 pages · 4 pages have no text to search",
      "Searched all 24 pages · 1 page has no text to search",
    ]);
  });
});
