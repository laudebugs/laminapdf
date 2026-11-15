import { describe, expect, it } from "vitest";
import { unsharedClaims } from "../src/organise/merge.tsx";
import { partNames, partsAtBookmarks, partsEvery, partsOfRanges } from "../src/organise/parts.ts";

const pagesOf = (result: ReturnType<typeof partsEvery>) =>
  result.ok ? result.parts.map((p) => p.pages) : result.error;

describe("the parts a split makes", () => {
  it("cuts every n pages, the last part with what is left", () => {
    expect(pagesOf(partsEvery(2, 5))).toEqual([[0, 1], [2, 3], [4]]);
    expect(pagesOf(partsEvery(5, 5))).toEqual([[0, 1, 2, 3, 4]]);
  });

  it("refuses a count that isn't a whole number from 1 to the page count", () => {
    for (const count of [0, 6, 1.5, Number.NaN]) {
      expect(partsEvery(count, 5)).toEqual({ ok: false, error: "A number from 1 to 5." });
    }
  });

  it("makes one part per comma part of the ranges, by number or label, and lists shared pages", () => {
    const labels = ["i", "ii", "1", "2", "3", "4"];
    const result = partsOfRanges("i–ii, 1-3, 4, 2–4", 6, labels);
    expect(pagesOf(result)).toEqual([[0, 1], [2, 3, 4], [5], [3, 4, 5]]);
    expect(result.ok && result.shared).toEqual([3, 4, 5]);
  });

  it("says why ranges name no pages, as the print dialog does", () => {
    expect(partsOfRanges("16–30", 24)).toEqual({ ok: false, error: "There's no page 30. This file has 24 pages." });
    expect(partsOfRanges(" , ", 24)).toMatchObject({ ok: false });
    expect(partsOfRanges("6–2", 24)).toMatchObject({ ok: false, error: expect.stringContaining("backwards") });
  });

  it("starts a part at each top-level bookmark, pages before the first one a part of their own", () => {
    const chapters = [
      { title: "Repairs", page: 3 },
      { title: "Summary", page: 1 },
      { title: "Also summary", page: 1 },
      { title: "Off the end", page: 40 },
    ];
    const result = partsAtBookmarks(chapters, 6);
    expect(result.ok && result.parts).toEqual([
      { pages: [0] },
      { pages: [1, 2], title: "Summary" },
      { pages: [3, 4, 5], title: "Repairs" },
    ]);
    expect(partsAtBookmarks([], 6)).toMatchObject({ ok: false });
  });
});

describe("part names", () => {
  const parts = [
    { pages: [0, 1], title: "Chair’s summary" },
    { pages: [2, 3, 4] },
    { pages: [5], title: "Chair’s summary" },
  ];
  const label = (i: number) => String(i + 1);

  it("names by number and bookmark, by file name and number, or by file name and pages", () => {
    expect(partNames(parts, "number-title", "Q3", label)).toEqual([
      "1. Chair’s summary",
      "2. Q3",
      "3. Chair’s summary",
    ]);
    expect(partNames(parts, "name-number", "Q3", label)).toEqual(["Q3 1", "Q3 2", "Q3 3"]);
    expect(partNames(parts, "name-pages", "Q3", label)).toEqual(["Q3 pages 1-2", "Q3 pages 3-5", "Q3 pages 6"]);
  });

  it("tells names apart that would repeat", () => {
    expect(partNames([{ pages: [1] }, { pages: [1] }], "name-pages", "Q3", label)).toEqual([
      "Q3 pages 2",
      "Q3 pages 2 (2)",
    ]);
  });

  it("pads numbers so the files sort in order", () => {
    const many = Array.from({ length: 12 }, (_, i) => ({ pages: [i] }));
    expect(partNames(many, "name-number", "Q3", label).slice(0, 2)).toEqual(["Q3 01", "Q3 02"]);
  });
});

describe("the merge's conformance notice", () => {
  it("lists the claims some files make and others don't", () => {
    expect(unsharedClaims([["PDF/A-2B"], ["PDF/A-2B"]])).toEqual([]);
    expect(unsharedClaims([["PDF/A-2B", "PDF/UA-1"], ["PDF/UA-1"], []])).toEqual(["PDF/A-2B", "PDF/UA-1"]);
  });
});
