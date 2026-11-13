// The page-labels edit: doc.setPageLabels and Transaction.setPageLabels, saved and reopened, and
// the ranges it refuses. The names start with the capability each test proves (I.10); they become
// capability tags once `cargo xtask capabilities` runs TypeScript tests.
import { Document, init, isLaminaPdfError, type PageLabelRange } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init());

/** Ten pages: i–iii, 1–4, A-1, A-2, then "Back cover". */
function open(): Document {
  return Document.open(read("fixtures/pages/labels.pdf"));
}

function labels(doc: Document): (string | undefined)[] {
  return doc.pages.map((page) => page.label);
}

describe("page labels", () => {
  it("I.10 (pdf, round-trip): sets labels that a saved and reopened file shows", () => {
    for (const mode of ["incremental", "fullRewrite"] as const) {
      using doc = open();
      const ids = doc.pages.map((page) => page.id);
      doc.setPageLabels([
        { page: ids[0] as string, prefix: "Cover" },
        { page: ids[1] as string, style: "lowerRoman" },
        { page: ids[3] as string, style: "decimal", prefix: "p. ", first: 5 },
      ]);
      using again = Document.open(doc.save({ mode }));
      expect(labels(again)).toEqual(["Cover", "i", "ii", "p. 5", "p. 6", "p. 7", "p. 8", "p. 9", "p. 10", "p. 11"]);
    }
  });

  it("removes every label when given no ranges", () => {
    using doc = open();
    doc.setPageLabels([]);
    expect(labels(doc).every((label) => label === undefined)).toBe(true);
  });

  it("applies inside a transaction, undone as one step", () => {
    using doc = open();
    const before = labels(doc);
    const first = doc.pages[0]?.id as string;
    doc.transact((tx) => tx.setPageLabels([{ page: first, style: "upperLetters" }]));
    expect(labels(doc)[1]).toBe("B");
    doc.undo();
    expect(labels(doc)).toEqual(before);
  });

  it("refuses ranges out of page order with invalid-label-ranges, saying which rule", () => {
    using doc = open();
    const ids = doc.pages.map((page) => page.id);
    const ranges: PageLabelRange[] = [
      { page: ids[0] as string, style: "decimal" },
      { page: ids[4] as string, style: "decimal" },
      { page: ids[2] as string, style: "decimal" },
    ];
    const error = thrown(() => doc.setPageLabels(ranges));
    expect(isLaminaPdfError(error, "invalid-label-ranges") && error.data?.what).toBe("order");
  });
});
