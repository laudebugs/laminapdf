// Page marks: doc.content.addWatermark, addBackground, setHeaderFooter, applyBates, pageMarks,
// updatePageMark and deletePageMark, saved and reopened, in the extract module (which draws
// the standard fonts). A test's title starts with the capability it proves (its tag: capabilities
// B.10 and C.14, column pdf), which `cargo xtask capabilities` counts.
import { Document, init, isLaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ text: true }));

const AT = "D:20261118120000Z";

function savedAndReopened(doc: Document): Document {
  return Document.open(doc.save({ mode: "incremental" }));
}

/** The text of page `index`, spaces collapsed. */
function textOf(doc: Document, index = 0): string {
  return doc.page(index).text().text().replace(/\s+/g, " ");
}

describe("page marks", () => {
  it("[cap B.10 pdf round-trip] puts a hideable watermark and a background on pages and finds them after a save", () => {
    using doc = Document.open(read("fixtures/corpus/outline.pdf"));
    doc.content.addWatermark([0, 1], {
      id: "draft",
      mark: { text: { text: "SDKWATERMARK", size: 48 } },
      opacity: 0.3,
      angle: 45,
      hideable: true,
      modified: AT,
    });
    doc.content.addBackground([0], {
      id: "paper",
      mark: { text: { text: "SDKBACKGROUND", size: 24 } },
      modified: AT,
    });
    using again = savedAndReopened(doc);
    expect(again.content.pageMarks(0).map((m) => [m.id, m.hideable])).toEqual([
      ["paper", false],
      ["draft", true],
    ]);
    expect(again.content.pageMarks(1).map((m) => m.id)).toEqual(["draft"]);
    expect(again.content.pageMarks(2)).toEqual([]);
    expect(again.content.added(0)).toEqual([]);
    expect(textOf(again)).toContain("SDKWATERMARK");
    expect(textOf(again)).toContain("SDKBACKGROUND");
    expect(textOf(again, 2)).not.toContain("SDKWATERMARK");
  });

  it("[cap C.14 pdf round-trip] fills page numbers, the total and the date in a footer on every page", () => {
    using doc = Document.open(read("fixtures/corpus/outline.pdf"));
    doc.content.setHeaderFooter([0, 1, 2], {
      id: "folio",
      slots: [null, null, "{file} · {date}", null, "Page {page} of {pages}", null],
      size: 9,
      date: "27 Nov 2026",
      file: "outline.pdf",
      modified: AT,
    });
    using again = savedAndReopened(doc);
    for (const index of [0, 1, 2]) {
      expect(textOf(again, index)).toContain(`Page ${index + 1} of 3`);
      expect(textOf(again, index)).toContain("outline.pdf · 27 Nov 2026");
    }
  });

  it("[cap C.14 pdf round-trip] numbers pages with a Bates number from the number given", () => {
    using doc = Document.open(read("fixtures/corpus/outline.pdf"));
    doc.content.applyBates([2, 0, 1], {
      id: "bates",
      prefix: "ACME-",
      start: 41,
      digits: 6,
      slot: "bottomRight",
      modified: AT,
    });
    using again = savedAndReopened(doc);
    expect(textOf(again, 2)).toContain("ACME-000041");
    expect(textOf(again, 0)).toContain("ACME-000042");
    expect(textOf(again, 1)).toContain("ACME-000043");
  });

  it("[cap C.14 pdf round-trip] pads a Bates number to six digits unless told otherwise", () => {
    using doc = Document.open(read("fixtures/corpus/outline.pdf"));
    doc.content.applyBates([0], { id: "padded", start: 7, slot: "topLeft", modified: AT });
    doc.content.applyBates([1], { id: "plain", start: 8, digits: 0, slot: "topLeft", modified: AT });
    using again = savedAndReopened(doc);
    expect(textOf(again, 0)).toContain("000007");
    expect(textOf(again, 1)).toMatch(/(^| )8( |$)/);
    expect(textOf(again, 1)).not.toContain("000008");
  });

  it("[cap B.10 pdf round-trip] changes a mark and takes it off after a save", () => {
    using doc = Document.open(read("fixtures/corpus/outline.pdf"));
    doc.content.addWatermark([0, 1], {
      id: "draft",
      mark: { text: { text: "BEFOREUPDATE", size: 48 } },
      hideable: true,
      modified: AT,
    });
    using again = savedAndReopened(doc);
    again.content.updatePageMark(
      [0, 1],
      "draft",
      { watermark: { mark: { text: { text: "AFTERUPDATE", size: 48 } }, angle: 30 } },
      AT,
    );
    expect(textOf(again)).toContain("AFTERUPDATE");
    expect(textOf(again)).not.toContain("BEFOREUPDATE");
    expect(again.content.pageMarks(0)[0]?.hideable).toBe(false);
    const removed = again.content.deletePageMark([0, 1], "draft");
    expect([again.content.pageMarks(0), again.content.pageMarks(1), removed.stillInEarlierRevisions]).toEqual([
      [],
      [],
      true,
    ]);
  });

  it("refuses an ID a page has, a page without the mark, a mark of another kind and an empty header", () => {
    using doc = Document.open(read("fixtures/corpus/outline.pdf"));
    const code = (f: () => unknown) => {
      const error = thrown(f);
      return isLaminaPdfError(error) ? error.code : error;
    };
    const mark = { id: "draft", mark: { text: { text: "DRAFT", size: 48 } }, modified: AT };
    doc.content.addWatermark([0], mark);
    expect(code(() => doc.content.addWatermark([0], mark))).toBe("content-id-in-use");
    expect(code(() => doc.content.deletePageMark([0, 1], "draft"))).toBe("unknown-page-mark");
    expect(
      code(() =>
        doc.content.updatePageMark([0], "draft", { background: { mark: { text: { text: "X", size: 9 } } } }, AT),
      ),
    ).toBe("invalid-op");
    expect(
      code(() =>
        doc.content.setHeaderFooter([0], { id: "empty", slots: [null, null, null, null, null, null], modified: AT }),
      ),
    ).toBe("invalid-op");
    expect(code(() => doc.content.addWatermark([9], mark))).toBe("page-index-out-of-range");
  });
});
