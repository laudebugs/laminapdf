// doc.commentSummary() over fixtures/annotations/review.pdf (its font embedded) and flatten.pdf
// (Helvetica, not embedded), as crates/laminapdf/tests/summary.rs reads them.
import { Document, Font, init } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(() => init({ text: true }));

const font = () => Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf"));
const A4: readonly [number, number] = [595.276, 841.89];

/** The summary's text once saved and opened again, its words separated by single spaces. */
function words(summary: Document): string {
  using saved = Document.open(summary.save());
  return saved.extract({ format: "text" }).split(/\s+/).join(" ");
}

describe("commentSummary", () => {
  it("[cap D.15 pdf round-trip] draws the page beside its comments as text", () => {
    using doc = Document.open(read("fixtures/annotations/review.pdf"));
    using summary = doc.commentSummary({ font: font(), paper: A4 });
    const text = words(summary);
    expect([text.split("Quoted words").length - 1, text.includes("1. Highlight"), summary.warnings]).toEqual([
      2,
      true,
      [],
    ]);
  });

  it("lists the comments without the page or their replies when asked", () => {
    using doc = Document.open(read("fixtures/annotations/review.pdf"));
    using summary = doc.commentSummary({ font: font(), layout: "list", replies: false, paper: A4 });
    const text = words(summary);
    expect([text.split("Quoted words").length - 1, text.includes("Agreed")]).toEqual([1, false]);
  });

  it("lists a page whose fonts aren't embedded without its drawing", () => {
    using doc = Document.open(read("fixtures/annotations/flatten.pdf"));
    using summary = doc.commentSummary({ font: font() });
    expect(summary.warnings.map((w) => w.code)).toEqual(["summary-page-not-drawn", "summary-page-not-drawn"]);
  });

  it("writes its own words as the caller gives them, English for the rest", () => {
    using doc = Document.open(read("fixtures/annotations/review.pdf"));
    using summary = doc.commentSummary({
      font: font(),
      layout: "list",
      paper: A4,
      words: {
        page: "Seite {label}",
        marked: "Abgehakt",
        kinds: { Highlight: "Hervorhebung" },
        states: { Accepted: "Angenommen" },
      },
    });
    const text = words(summary);
    expect([
      text.includes("Seite 1"),
      text.includes("1. Hervorhebung"),
      text.includes("Status: Angenommen · Abgehakt"),
      text.includes("Page 1"),
      text.includes("2. Note"),
    ]).toEqual([true, true, true, false, true]);
  });

  it("refuses a word it doesn't know", () => {
    using doc = Document.open(read("fixtures/annotations/review.pdf"));
    const words = { pages: "Seiten" } as unknown as { page: string };
    expect(() => doc.commentSummary({ font: font(), words })).toThrow(
      expect.objectContaining({ code: "unknown-option" }),
    );
  });

  it("refuses a paper it doesn't know", () => {
    using doc = Document.open(read("fixtures/annotations/review.pdf"));
    const paper = "tabloid" as unknown as "document";
    expect(() => doc.commentSummary({ font: font(), paper })).toThrow(
      expect.objectContaining({ code: "invalid-argument" }),
    );
  });
});
