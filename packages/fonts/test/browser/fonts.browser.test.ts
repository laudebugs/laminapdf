// fonts.css in a real page: which files the browser fetches, from where, and what happens to
// text the fonts don't cover.

import "../../fonts.css";
import { describe, expect, it } from "vitest";

/** The font files the page has requested so far. */
function fontRequests(): string[] {
  return performance
    .getEntriesByType("resource")
    .map((entry) => entry.name)
    .filter((name) => name.includes(".woff2"));
}

/** A paragraph of `text` in `font`, added to the page. */
function show(text: string, font: string): HTMLElement {
  const paragraph = document.createElement("p");
  paragraph.textContent = text;
  paragraph.style.font = font;
  document.body.append(paragraph);
  return paragraph;
}

describe("fonts.css", () => {
  it("fetches Lamina Sans' Latin file for Latin text, and Latin Extended only when needed", async () => {
    show("Open a PDF", '400 16px "Lamina Sans"');
    await document.fonts.load('400 16px "Lamina Sans"', "Open a PDF");
    await document.fonts.ready;
    expect(fontRequests().some((url) => url.includes("lamina-sans-latin-wght-normal"))).toBe(true);
    expect(fontRequests().filter((url) => url.includes("latin-ext"))).toEqual([]);

    show("Łódź, Ōsaka", '400 16px "Lamina Sans"');
    await document.fonts.load('400 16px "Lamina Sans"', "Łō");
    expect(fontRequests().some((url) => url.includes("lamina-sans-latin-ext-wght-normal"))).toBe(true);
  });

  it("loads Newsreader and Caveat only when text uses them", async () => {
    expect(fontRequests().filter((url) => /newsreader|caveat/.test(url))).toEqual([]);
    show("The quarterly report", '500 20px "Newsreader"');
    await document.fonts.load('500 20px "Newsreader"', "The quarterly report");
    expect(fontRequests().some((url) => url.includes("newsreader-latin-wght-normal"))).toBe(true);
    expect(fontRequests().some((url) => url.includes("caveat"))).toBe(false);
  });

  it("makes every request to the page's own origin", () => {
    const origins = new Set(performance.getEntriesByType("resource").map((entry) => new URL(entry.name).origin));
    expect([...origins]).toEqual([location.origin]);
  });

  it("draws Greek, Cyrillic and CJK text in the system's fonts", async () => {
    const samples = ["Ελληνικά", "Кириллица", "漢字かな"];
    const before = fontRequests().length;
    const paragraphs = samples.map((text) =>
      show(text, '400 16px "Lamina Sans", "Lamina Sans Fallback", system-ui, sans-serif'),
    );
    await Promise.all(samples.map((text) => document.fonts.load('400 16px "Lamina Sans"', text)));
    await document.fonts.ready;
    for (const paragraph of paragraphs) {
      const range = document.createRange();
      range.selectNodeContents(paragraph);
      expect(range.getBoundingClientRect().width).toBeGreaterThan(0);
    }
    // No face's unicode-range covers these scripts, so the browser fetches nothing for them.
    expect(fontRequests().slice(before)).toEqual([]);
  });
});
