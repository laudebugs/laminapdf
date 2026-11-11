import { describe, expect, it } from "vitest";
import type { ReadingBlock } from "../protocol.ts";
import { copySelection, expandLigatures, joinLines } from "./copy.ts";
import type { Piece, SpanSpec } from "./spans.ts";

/** A piece of upright text on page `page` whose span starts at (`x`, `y`) (its top), 10 pt tall. */
const piece = (text: string, x: number, y: number, page = 0): Piece => ({
  page,
  text,
  span: { run: 0, text, left: x, top: y, width: text.length * 5, height: 10, angle: 0, rtl: false },
});

const block = (x: number, y: number, w: number, h: number, type = "paragraph"): ReadingBlock => ({
  bbox: [x, y, w, h],
  type,
});

// Two columns: the left one from x 50 to 250, the right one from 300 to 500; two lines each. The
// runs come in the order the page drew them, line by line across both columns.
const twoColumns = [
  piece("The left column begins", 50, 100),
  piece("The right column begins", 300, 100),
  piece("and ends here.", 50, 112),
  piece("and ends there.", 300, 112),
];
const columns = [block(50, 100, 200, 22), block(300, 100, 200, 22)];

describe("copySelection", () => {
  it("copies as paragraph in reading order, one line per block", () => {
    const { text } = copySelection(twoColumns, "paragraph", () => columns);
    expect(text).toBe("The left column begins and ends here.\n\nThe right column begins and ends there.");
  });

  it("keeps the layout's lines in copy exact, columns side by side as drawn", () => {
    const { text } = copySelection(twoColumns, "exact", () => columns);
    expect(text).toBe("The left column begins The right column begins\nand ends here. and ends there.");
  });

  it("pastes a paragraph with no line break inside it", () => {
    const { text } = copySelection(twoColumns, "paragraph", () => columns);
    for (const paragraph of text.split("\n\n")) expect(paragraph).not.toContain("\n");
  });

  it("writes HTML with a paragraph per block, escaped", () => {
    const pieces = [piece("a < b & c", 50, 100), piece("second", 300, 100)];
    const { html } = copySelection(pieces, "paragraph", () => columns);
    expect(html).toBe("<p>a &lt; b &amp; c</p><p>second</p>");
    expect(copySelection(twoColumns, "exact", () => columns).html).toContain("<br>");
  });

  it("keeps a table's lines apart in copy as paragraph", () => {
    const rows = [piece("Name Count", 50, 100), piece("Ash 3", 50, 112)];
    const { text } = copySelection(rows, "paragraph", () => [block(50, 100, 200, 24, "table")]);
    expect(text).toBe("Name Count\nAsh 3");
  });

  it("puts a piece outside every block in the nearest one", () => {
    const pieces = [piece("inside", 50, 100), piece("just below", 50, 112)];
    const { text } = copySelection(pieces, "paragraph", () => [block(50, 100, 200, 10)]);
    expect(text).toBe("inside just below");
  });

  it("joins a page without known blocks in run order", () => {
    const { text } = copySelection(twoColumns.slice(0, 1), "paragraph", () => undefined);
    expect(text).toBe("The left column begins");
  });

  it("puts each page's blocks after the page before's", () => {
    const pieces = [piece("first page", 50, 100, 0), piece("second page", 50, 100, 1)];
    const { text } = copySelection(pieces, "paragraph", () => [block(50, 100, 200, 10)]);
    expect(text).toBe("first page\n\nsecond page");
  });

  it("expands ligatures in both modes", () => {
    const pieces = [piece("eﬃcient ﬂow", 50, 100)];
    expect(copySelection(pieces, "exact", () => undefined).text).toBe("efficient flow");
    expect(copySelection(pieces, "paragraph", () => undefined).text).toBe("efficient flow");
  });
});

describe("joinLines", () => {
  it("joins lines with a space", () => {
    expect(joinLines(["one line", "and the next"])).toBe("one line and the next");
  });

  it("repairs a word broken by a hyphen before a lowercase letter", () => {
    expect(joinLines(["a broken hy-", "phen here"])).toBe("a broken hyphen here");
  });

  it("keeps a hyphen before a capital or a digit, with no space, as the extraction does", () => {
    expect(joinLines(["Jean-", "Paul"])).toBe("Jean-Paul");
    expect(joinLines(["pre-", "2020"])).toBe("pre-2020");
    expect(joinLines(["a dash -", "then"])).toBe("a dash - then");
  });

  it("joins after a soft hyphen, writes it as a hyphen before a capital, and drops the rest", () => {
    expect(joinLines(["hyphen\u00ad", "ation"])).toBe("hyphenation");
    expect(joinLines(["Jean\u00ad", "Paul"])).toBe("Jean-Paul");
    expect(joinLines(["hy\u00adphen"])).toBe("hyphen");
  });

  it("skips empty lines", () => {
    expect(joinLines(["", "one", "  ", "two"])).toBe("one two");
  });
});

describe("expandLigatures", () => {
  it("writes each Latin ligature as its letters", () => {
    expect(expandLigatures("ﬀﬁﬂﬃﬄﬅﬆ")).toBe("fffiflffifflstst");
  });
});

describe("copy exact: the layout's lines", () => {
  const spec = (left: number, top: number, width: number, extra: Partial<SpanSpec> = {}): SpanSpec => ({
    run: 0,
    text: "",
    left,
    top,
    width,
    height: 10,
    angle: 0,
    rtl: false,
    ...extra,
  });
  const piece = (text: string, s: SpanSpec, page = 0): Piece => ({ page, text, span: s });
  const exact = (pieces: Piece[]) => copySelection(pieces, "exact", () => undefined).text;

  it("puts a space at a gap, nothing where runs touch, a new line at a new baseline and between pages", () => {
    const text = exact([
      piece("Hello", spec(0, 0, 30)),
      piece("world", spec(35, 0, 30)),
      piece("!", spec(65, 0, 3)),
      piece("Next line", spec(0, 14, 50)),
      piece("Page two", spec(0, 0, 40), 1),
    ]);
    expect(text).toBe("Hello world!\nNext line\nPage two");
  });

  it("measures a right-to-left line's gaps from the right, and drops soft hyphens", () => {
    const text = exact([
      piece("שלום", spec(60, 0, 30, { rtl: true })),
      piece("עולם", spec(20, 0, 30, { rtl: true })),
      piece("tog­ether", spec(0, 14, 40)),
    ]);
    expect(text).toBe("שלום עולם\ntogether");
  });
});
