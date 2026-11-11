import { describe, expect, it } from "vitest";
import type { TextRunData } from "../protocol.ts";
import { findOnPage, matchRanges, type PageGlyphs, type SearchText, searchText, snippetOf } from "./page.ts";

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

describe("snippetOf", () => {
  const textOnly = (text: string): SearchText => ({
    text,
    lo: new Int32Array(text.length),
    hi: new Int32Array(text.length),
    runOf: new Int32Array(0),
  });
  const snippets = (text: string, query: string, options = {}) => {
    const search = textOnly(text);
    const { folded, ranges } = matchRanges(search, query, options);
    return ranges.map((r) => snippetOf(search, folded, r));
  };

  it("keeps the whole line when it is short, with the match as the page spells it", () => {
    expect(snippets("The Café opens at nine.", "cafe")).toEqual([
      { before: "The ", match: "Café", after: " opens at nine." },
    ]);
  });

  it("cuts long context at a word on each side and marks the cut", () => {
    const text = `${"alpha beta gamma delta epsilon zeta eta theta "}needle${" iota kappa lambda mu nu xi omicron pi rho"}`;
    const [s] = snippets(text, "needle");
    expect(s?.before.startsWith("…")).toBe(true);
    expect(s?.after.endsWith("…")).toBe(true);
    // Whole words only: no word is cut in half.
    const words = new Set(text.split(" "));
    for (const w of [
      ...(s?.before.slice(1).trim().split(" ") ?? []),
      ...(s?.after.slice(0, -1).trim().split(" ") ?? []),
    ]) {
      expect(words.has(w)).toBe(true);
    }
    expect((s?.before.length ?? 0) <= 41 && (s?.after.length ?? 0) <= 41).toBe(true);
  });

  it("collapses the page's line breaks and runs of spaces to single spaces", () => {
    expect(snippets("first line\nsecond   line", "second")).toEqual([
      { before: "first line ", match: "second", after: " line" },
    ]);
  });

  it("never splits a character outside the BMP at a cut, whichever half the cut lands on", () => {
    // Each 𝒜 is two units and the "b" one, so a cut 40 units out lands between an 𝒜's halves.
    const text = `${"𝒜".repeat(21)}b針b${"𝒜".repeat(21)}`;
    const [s] = snippets(text, "針");
    expect(s?.match).toBe("針");
    expect(/^…(?:𝒜)+b$/u.test(s?.before ?? "")).toBe(true);
    expect(/^b(?:𝒜)+…$/u.test(s?.after ?? "")).toBe(true);
  });

  it("keeps a match's last character whole: a character outside the BMP, or one with combining marks", () => {
    expect(snippets("see 🎉🎉 here", "🎉🎉")).toEqual([{ before: "see ", match: "🎉🎉", after: " here" }]);
    expect(snippets("The Cafe\u0301 opens", "cafe")).toEqual([
      { before: "The ", match: "Cafe\u0301", after: " opens" },
    ]);
  });

  it("keeps a word the cut lands at the start or end of", () => {
    const left = `${"x".repeat(9)} ${"abcd ".repeat(8)}needle`;
    const [l] = snippets(left, "needle");
    // The cut 40 units back lands at the first "abcd": it is kept whole.
    expect(l?.before).toBe(`…${"abcd ".repeat(8)}`);
    const right = `needle${" abcd".repeat(8)} ${"x".repeat(9)}`;
    const [r] = snippets(right, "needle");
    expect(r?.after).toBe(`${" abcd".repeat(8)}…`);
  });

  it("cuts inside a long word when only spaces separate it from the match, rather than show nothing", () => {
    const [one] = snippets(`${"y".repeat(60)} needle ${"z".repeat(60)}`, "needle");
    expect(one?.before).toBe(`…${"y".repeat(39)} `);
    expect(one?.after).toBe(` ${"z".repeat(39)}…`);
    // Text from a PDF often has two spaces, or a space and a line break.
    const [two] = snippets(`${"y".repeat(60)}  needle \n${"z".repeat(60)}`, "needle");
    expect(two?.before).toBe(`…${"y".repeat(38)} `);
    expect(two?.after).toBe(` ${"z".repeat(38)}…`);
  });

  it("never starts or ends a cut on a combining mark", () => {
    const word = "e\u0301".repeat(40);
    const [s] = snippets(`${word} needle ${word}`, "needle");
    expect(/^…e/u.test(s?.before ?? "")).toBe(true);
    expect(/e\u0301…$/u.test(s?.after ?? "")).toBe(true);
  });
});
