import { describe, expect, it } from "vitest";
import { parsePrintPages } from "../src/reader/print-pages.ts";

describe("parsePrintPages", () => {
  it("reads numbers and ranges at a hyphen or an en dash, in the order given", () => {
    expect(parsePrintPages("1-3, 7, 5–6", 10)).toEqual({ ok: true, pages: [0, 1, 2, 6, 4, 5] });
  });

  it("reads labels, including a label that holds a hyphen, and ranges between labels", () => {
    const labels = ["i", "ii", "1", "2", "3", "A-1", "A-2", "A-3"];
    expect(parsePrintPages("ii, 2–3, A-1", 8, labels)).toEqual({ ok: true, pages: [1, 3, 4, 5] });
    expect(parsePrintPages("A-1-A-3", 8, labels)).toEqual({ ok: true, pages: [5, 6, 7] });
  });

  it("prefers a label to the page number it spells", () => {
    // Page 3's label is "1": "1" names it, as the reader's page field does.
    expect(parsePrintPages("1", 4, ["i", "ii", "1", "2"])).toEqual({ ok: true, pages: [2] });
  });

  it("names a page the file doesn't have, with its count and label range", () => {
    expect(parsePrintPages("30", 24)).toEqual({
      ok: false,
      error: "There's no page 30. This file has 24 pages.",
    });
    const labels = ["1", "2", "A-1"];
    expect(parsePrintPages("1-9", 3, labels)).toEqual({
      ok: false,
      error: "There's no page 9. This file has 3 pages (1–A-1).",
    });
  });

  it("refuses an empty range and one that runs backwards", () => {
    expect(parsePrintPages(" , ", 5).ok).toBe(false);
    expect(parsePrintPages("4-2", 5)).toEqual({
      ok: false,
      error: "The range 4-2 runs backwards. Write it lowest page first.",
    });
  });
});
