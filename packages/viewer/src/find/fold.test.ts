import { describe, expect, it } from "vitest";
import { fold, foldQuery, matches } from "./fold.js";

const text = (s: string, options = {}) => fold(s, options).text;

describe("fold", () => {
  it("unfolds ligatures", () => {
    expect(text("ﬁnd the ﬂow")).toBe("find the flow");
  });

  it("drops soft hyphens", () => {
    expect(text("hy­phen")).toBe("hyphen");
  });

  it("joins a word a hyphen splits at a line's end", () => {
    expect(text("exam-\nple")).toBe("example");
  });

  it("keeps a hyphen inside a line, and one before a number at a line's end", () => {
    expect([text("well-known"), text("1-\n2")]).toEqual(["well-known", "1- 2"]);
  });

  it("collapses white space, a no-break space included", () => {
    expect(text("a  \n\tb")).toBe("a b");
  });

  it("ignores case unless asked", () => {
    expect([text("ÉCOLE Straße"), text("ÉCOLE", { matchCase: true, matchDiacritics: true })]).toEqual([
      "ecole straße",
      "ÉCOLE",
    ]);
  });

  it("ignores Latin diacritics unless asked", () => {
    expect([text("café"), text("café", { matchDiacritics: true })]).toEqual(["cafe", "café"]);
  });

  it("folds I to i whatever the locale", () => {
    expect(text("I")).toBe("i");
  });

  it("drops Hebrew points and Arabic harakat with the accents, as pdf.js does", () => {
    expect([text("שָׁלוֹם"), text("كِتَاب")]).toEqual(["שלום", "كتاب"]);
  });

  it("keeps the marks when diacritics count, and CJK as it is", () => {
    expect([text("שָׁלוֹם", { matchDiacritics: true }), text("漢字かな")]).toEqual(["שָׁלוֹם", "漢字かな"]);
  });

  it("maps each folded unit back to its source unit", () => {
    const { text: t, from } = fold("ﬁ x");
    expect([t, [...from]]).toEqual(["fi x", [0, 0, 1, 2]]);
  });
});

describe("matches", () => {
  it("finds every match, never overlapping", () => {
    expect(matches("aaaa", "aa")).toEqual([
      [0, 2],
      [2, 4],
    ]);
  });

  it("finds whole words only when asked", () => {
    expect([matches("cat concat cat.", "cat").length, matches("cat concat cat.", "cat", true).length]).toEqual([3, 2]);
  });

  it("lets a whole-word query that ends in punctuation sit against a word", () => {
    expect(matches("e.g.word", "e.g.", true)).toEqual([[0, 4]]);
  });

  it("finds nothing for an empty query", () => {
    expect(matches("text", foldQuery("   "))).toEqual([]);
  });
});
