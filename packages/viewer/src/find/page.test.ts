import { describe, expect, it } from "vitest";
import type { TextRunData } from "../protocol.ts";
import { findOnPage, type PageGlyphs, searchText } from "./page.ts";

/** One glyph per character, 10 wide, along a baseline at `y` from `x`; runs given as [text, x, y]. */
function page(runs: readonly (readonly [string, number, number, ("rtl" | "actual")?])[]): PageGlyphs {
  const quads: number[] = [];
  const texts: string[] = [];
  const data: TextRunData[] = [];
  for (const [text, x0, y, kind] of runs) {
    const start = texts.length;
    const chars = [...text];
    // Glyphs in drawing order: right-to-left text is drawn left to right, so reversed.
    const glyphs = kind === "rtl" ? [...chars].reverse() : kind === "actual" ? ["X", "Y"] : chars;
    glyphs.forEach((g, i) => {
      const x = x0 + i * 10;
      quads.push(x, y, x + 10, y, x + 10, y - 10, x, y - 10);
      texts.push(g);
    });
    data.push({
      text,
      size: 10,
      vertical: false,
      actualText: kind === "actual",
      invisible: false,
      glyphRange: [start, texts.length],
    });
  }
  const offsets = [0];
  for (const t of texts) offsets.push((offsets.at(-1) ?? 0) + t.length);
  return {
    runs: data,
    quads: Float32Array.from(quads),
    glyphText: texts.join(""),
    glyphOffsets: Uint32Array.from(offsets),
  };
}

const find = (p: PageGlyphs, query: string, options = {}) => findOnPage(p, searchText(p), query, options);

describe("searchText", () => {
  it("joins runs as copy does: a space at a gap, a new line on the next line", () => {
    expect(
      searchText(
        page([
          ["Hello", 0, 20],
          ["world", 70, 20],
          ["next", 0, 40],
        ]),
      ).text,
    ).toBe("Hello world\nnext");
  });
});

describe("findOnPage", () => {
  it("gives a match's quad from its first glyph to its last", () => {
    expect(find(page([["Hello world", 0, 20]]), "world")).toEqual([[60, 20, 110, 20, 110, 10, 60, 10]]);
  });

  it("gives one quad per run a match crosses", () => {
    const p = page([
      ["Hello", 0, 20],
      ["world", 70, 20],
    ]);
    // Two quads of 8 numbers, one after the other.
    expect(find(p, "lo wo")).toEqual([[30, 20, 50, 20, 50, 10, 30, 10, 70, 20, 90, 20, 90, 10, 70, 10]]);
  });

  it("finds a word a hyphen splits across lines", () => {
    expect(
      find(
        page([
          ["exam-", 0, 20],
          ["ple", 0, 40],
        ]),
        "example",
      ),
    ).toHaveLength(1);
  });

  it("maps right-to-left text to its glyphs in drawing order", () => {
    // שלום drawn as ם ו ל ש from x = 0: "של" is the last two glyphs, x 20 to 40.
    expect(find(page([["שלום", 0, 20, "rtl"]]), "של")).toEqual([[20, 20, 40, 20, 40, 10, 20, 10]]);
  });

  it("covers the whole run for text that replaces its glyphs", () => {
    expect(find(page([["ab", 0, 20, "actual"]]), "b")).toEqual([[0, 20, 20, 20, 20, 10, 0, 10]]);
  });

  it("counts case and whole words as asked", () => {
    const p = page([["Cat concat cat", 0, 20]]);
    expect([
      find(p, "cat").length,
      find(p, "cat", { matchCase: true }).length,
      find(p, "cat", { wholeWords: true }).length,
    ]).toEqual([3, 2, 2]);
  });
});
