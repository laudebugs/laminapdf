import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  type LabelRangeValue,
  labelsOf,
  numberIn,
  pageLabels,
  previewOf,
  problemsOf,
  rangesOf,
  row,
  rowsOf,
  sharedLabel,
  startsOf,
} from "../src/organise/labels.js";

/** What pdf.js and PDFium show for the labels fixtures, which `SetPageLabels` wrote (fixtures/pages/set-labels/). */
const facts = JSON.parse(
  readFileSync(new URL("../../../fixtures/pages/set-labels/facts.json", import.meta.url), "utf8"),
) as { files: Record<string, { pdfjs: string[]; pdfium: string[] }> };

const PAGES = Array.from({ length: 10 }, (_, i) => `p${i + 1}.0`);

/** Each fixture's ranges, as laminapdf-save's tests/labels.rs writes them: first page, style, prefix, first number. */
const FIXTURES: Record<string, readonly [number, LabelRangeValue["style"], string, number][]> = {
  "roman.pdf": [[0, "upperRoman", "", 1]],
  "alpha.pdf": [[0, "upperLetters", "", 25]],
  "prefixed.pdf": [[0, "decimal", "Ch-", 1]],
  "restarts.pdf": [
    [0, "decimal", "", 1],
    [3, "decimal", "", 1],
    [7, "decimal", "", 10],
  ],
  "mixed.pdf": [
    [0, "lowerRoman", "", 1],
    [2, "decimal", "", 5],
    [8, "lowerLetters", "App. ", 1],
    [9, undefined, "Back cover", 1],
  ],
};

const ranges = (spec: (typeof FIXTURES)[string]): LabelRangeValue[] =>
  spec.map(([at, style, prefix, first]) => ({
    page: PAGES[at] ?? "",
    ...(style === undefined ? {} : { style }),
    ...(prefix === "" ? {} : { prefix }),
    ...(first === 1 ? {} : { first }),
  }));

describe("the labels preview", () => {
  for (const [name, spec] of Object.entries(FIXTURES)) {
    it(`shows what pdf.js and PDFium show for ${name}`, () => {
      const shown = labelsOf(rowsOf(ranges(spec), PAGES), PAGES.length).map((l) => l.label);
      const fact = facts.files[name];
      expect([shown, shown]).toEqual([fact?.pdfjs, fact?.pdfium]);
    });

    it(`gives back the ranges it was given for ${name}`, () => {
      expect(rangesOf(rowsOf(ranges(spec), PAGES), PAGES)).toEqual(ranges(spec));
    });
  }

  it("numbers as Lamina PDF does past the letters and above 10,000", () => {
    expect([numberIn("upperLetters", 27), numberIn("lowerLetters", 53), numberIn("upperRoman", 1994)]).toEqual([
      "AA",
      "aaa",
      "MCMXCIV",
    ]);
    expect([numberIn("lowerRoman", 10_001), numberIn("none", 4)]).toEqual(["10001", ""]);
  });

  it("starts a document without labels at 1, 2, 3 from page 1", () => {
    const rows = rowsOf([], PAGES);
    expect(rows.map((r) => [r.from, r.style, r.prefix, r.start])).toEqual([["1", "decimal", "", "1"]]);
    expect(rangesOf(rows, PAGES)).toEqual([{ page: "p1.0", style: "decimal" }]);
  });

  it("starts a file whose first range is after page 1 with 1, 2, 3 from page 1", () => {
    const rows = rowsOf([{ page: "p3.0", style: "upperRoman" }], PAGES);
    expect(rangesOf(rows, PAGES)).toEqual([
      { page: "p1.0", style: "decimal" },
      { page: "p3.0", style: "upperRoman" },
    ]);
    expect(labelsOf(rows, 4).map((l) => l.label)).toEqual(["1", "2", "I", "II"]);
  });

  it("names two rows on one page, a page past the end and a first number of 0", () => {
    const rows = [row(1), row(22, "decimal"), row(22, "upperLetters"), row(30), row(2, "decimal", "", 0)];
    const problems = problemsOf(rows, 24);
    expect(problems.clash?.page).toBe(22);
    expect([...(problems.clash?.keys ?? [])]).toEqual([rows[1]?.key, rows[2]?.key]);
    expect([...problems.outside]).toEqual([rows[3]?.key]);
    expect([...problems.badStart]).toEqual([rows[4]?.key]);
  });

  it("notes the first two pages that share a label", () => {
    expect(sharedLabel(["1", "2", "1", "2"])).toEqual({ first: 1, second: 3, label: "1" });
    expect(sharedLabel(["i", "ii", "1"])).toBeUndefined();
  });

  it("shows the first pages, each range's start and the last pages, and names the runs between", () => {
    const rows = [row(1, "lowerRoman"), row(3), row(22, "decimal", "A-")];
    const labels = labelsOf(rows, 24);
    const preview = previewOf(labels, startsOf(rows, 24));
    expect(
      preview.map((e) =>
        e.kind === "page" ? `${e.page}:${e.label}` : `${e.from}-${e.to}:${e.firstLabel}-${e.lastLabel}`,
      ),
    ).toEqual(["1:i", "2:ii", "3:1", "4:2", "5-20:3-18", "21:19", "22:A-1", "23:A-2", "24:A-3"]);
  });
});

describe("pageLabels", () => {
  const pages = ["p1.0", "p2.0", "p3.0", "p4.0"];

  it("labels every page under the ranges, the pages before the first by their number", () => {
    const ranges: LabelRangeValue[] = [
      { page: "p2.0", style: "lowerRoman" },
      { page: "p4.0", style: "decimal", prefix: "A-" },
    ];
    expect(pageLabels(ranges, pages)).toEqual(["1", "i", "ii", "A-1"]);
  });

  it("leaves every page unlabelled when the document labels none", () => {
    expect(pageLabels([], pages)).toEqual([undefined, undefined, undefined, undefined]);
  });
});
