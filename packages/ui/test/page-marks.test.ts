// The Page marks dialog's model (Paper A03): tokens shown in brackets and written as braces, the
// pages a range names, where a slot's text lands and what it runs into, and the footer's words.

import { describe, expect, it } from "vitest";
import {
  bandTexts,
  batesText,
  dateOf,
  fillTokens,
  formatDate,
  overlapOf,
  pagesText,
  readPages,
  showTokens,
  slotBox,
  summaryOf,
  writeTokens,
} from "../src/pagemarks/model.ts";

const LABELS = Array.from({ length: 24 }, (_, i) => String(i + 1));
const PAGE = { width: 612, height: 792 };

describe("tokens", () => {
  it("are written as the core reads them and shown again as typed", () => {
    expect(writeTokens("[Page] of [Pages]")).toBe("{page} of {pages}");
    expect(writeTokens("[Page label] · [Date] · [File name]")).toBe("{label} · {date} · {file}");
    expect(showTokens("{page} of {pages}")).toBe("[Page] of [Pages]");
  });

  it("keep a person's own braces as braces", () => {
    expect(writeTokens("{draft} [Page]")).toBe("{{draft}} {page}");
    expect(showTokens("{{draft}} {page}")).toBe("{draft} [Page]");
  });

  it("round-trip a literal {page}, a token and both together", () => {
    for (const typed of ["{page}", "[Page]", "{page} is [Page]", "{{[Pages]}}", "} {", "{nope}"]) {
      expect(showTokens(writeTokens(typed))).toBe(typed);
    }
    expect(showTokens("{{page}}")).toBe("{page}");
    expect(showTokens("{page}")).toBe("[Page]");
    expect(showTokens("{{page}} {page}")).toBe("{page} [Page]");
  });

  it("are filled for a page", () => {
    const values = { page: 3, pages: 24, label: "iii", date: "27 Nov 2026", file: "a.pdf" };
    expect(fillTokens("Page [Page] of [Pages] ([Page label])", values)).toBe("Page 3 of 24 (iii)");
  });
});

describe("pages", () => {
  it("name all of them, or a range by number and by label", () => {
    expect(readPages({ all: true, range: "" }, LABELS).pages).toHaveLength(24);
    expect(readPages({ all: false, range: "2–4, 9" }, LABELS).pages).toEqual([1, 2, 3, 8]);
    const labelled = ["i", "ii", "1", "2"];
    expect(readPages({ all: false, range: "ii, 2" }, labelled).pages).toEqual([1, 3]);
  });

  it("say why a range names none", () => {
    expect(readPages({ all: false, range: "1–30" }, LABELS).error).toContain("no page 30");
    expect(readPages({ all: false, range: "" }, LABELS)).toEqual({ pages: [] });
  });

  it("are said as ranges", () => {
    expect(pagesText([1, 2, 3, 8], LABELS)).toBe("2–4, 9");
    expect(pagesText([0], LABELS)).toBe("1");
  });
});

describe("numbers", () => {
  it("a Bates number is padded with zeros and grows past its digits", () => {
    expect(batesText({ prefix: "NHT-", suffix: "", digits: 6 }, 1)).toBe("NHT-000001");
    expect(batesText({ prefix: "", suffix: " CONF", digits: 2 }, 1234)).toBe("1234 CONF");
    expect(batesText({ prefix: "", suffix: "", digits: 0 }, 5)).toBe("5");
  });

  it("a stored date is kept until another format is chosen", () => {
    const day = new Date(2026, 10, 27);
    expect(dateOf({ dateFormat: "stored", storedDate: "1 Jan 2020" }, day)).toBe("1 Jan 2020");
    expect(dateOf({ dateFormat: "iso", storedDate: "1 Jan 2020" }, day)).toBe("2026-11-27");
  });

  it("a date is written in each format", () => {
    const day = new Date(2026, 10, 27);
    expect(formatDate(day, "iso")).toBe("2026-11-27");
    expect(formatDate(day, "short")).toBe("27 Nov 2026");
    expect(formatDate(day, "long")).toBe("27 November 2026");
  });
});

describe("where a slot's text lands", () => {
  const margins = { top: 36, bottom: 36 };

  it("hangs a header from the top margin and stands a footer on the bottom one", () => {
    expect(slotBox("topLeft", 100, 10, margins, PAGE)).toEqual([36, 36, 136, 51]);
    const [, y0, , y1] = slotBox("bottomLeft", 100, 10, margins, PAGE);
    expect([y0, y1]).toEqual([741, 756]);
  });

  it("aligns by column and keeps to the text area", () => {
    expect(slotBox("bottomCenter", 100, 10, margins, PAGE)[0]).toBe(256);
    expect(slotBox("bottomRight", 100, 10, margins, PAGE)[2]).toBe(576);
    const [x0, , x1] = slotBox("topRight", 9999, 10, margins, PAGE);
    expect([x0, x1]).toEqual([36, 576]);
  });
});

describe("the overlap notice", () => {
  const existing = [{ text: "Page 3 of 24", bounds: [492, 736, 576, 750] as const }];

  it("finds a page number under a new footer slot", () => {
    const box = slotBox("bottomRight", 60, 9, { top: 36, bottom: 36 }, PAGE);
    expect(overlapOf([{ slot: "bottomRight", box }], existing)).toEqual({
      slot: "bottomRight",
      text: "Page 3 of 24",
      pageNumber: true,
    });
  });

  it("finds nothing on another slot, and calls other text text", () => {
    const left = slotBox("bottomLeft", 60, 9, { top: 36, bottom: 36 }, PAGE);
    expect(overlapOf([{ slot: "bottomLeft", box: left }], existing)).toBeUndefined();
    const over = overlapOf(
      [{ slot: "bottomRight", box: slotBox("bottomRight", 60, 9, { top: 36, bottom: 36 }, PAGE) }],
      [{ text: "Confidential", bounds: [500, 736, 576, 750] }],
    );
    expect(over?.pageNumber).toBe(false);
  });

  it("looks only at the top and bottom tenths of the page", () => {
    const runs = [
      { text: "Body", bounds: [72, 300, 200, 312] as const },
      { text: "Running head", bounds: [72, 30, 200, 42] as const },
      { text: "   ", bounds: [72, 740, 80, 750] as const },
    ];
    expect(bandTexts(runs, PAGE).map((r) => r.text)).toEqual(["Running head"]);
  });
});

describe("the footer's summary", () => {
  it("says what the button does", () => {
    expect(summaryOf("watermark", [...LABELS.keys()], LABELS, false)).toBe("Adds the watermark to all 24 pages.");
    expect(summaryOf("headerFooter", [...LABELS.keys()].slice(1), LABELS, false)).toBe(
      "Adds a header and footer to pages 2–24 (23 pages).",
    );
    expect(summaryOf("bates", [4], LABELS, true)).toBe("Updates the Bates numbers on page 5.");
    expect(summaryOf("bates", [], LABELS, false)).toBe("Choose at least one page.");
  });
});
