// Glyph coverage: the characters UI text uses are in the Latin files, or knowingly left to the
// system font. Until @laminapdf/ui has a message catalogue, the list is the design language's
// punctuation and the shortcut chips' symbols.

import { describe, expect, it } from "vitest";
import { codePoints, lock } from "./helpers.ts";

const ASCII = Array.from({ length: 0x7f - 0x20 }, (_, i) => String.fromCodePoint(0x20 + i)).join("");
const WESTERN = "àáâäãåæçèéêëìíîïñòóôöõøœùúûüýÿßÀÉÖÜ";
const PUNCTUATION = "’‘“”·…–—€•×°";
const SHORTCUTS = "⌘⌥⇧⌫←↑→↓✓";
/** Characters the UI may use that the fonts lack: they render in the system font, per glyph. */
const SYSTEM_FALLBACK = "↵";

function covered(file: string): Set<number> {
  return codePoints(lock().outputs[file]?.coverage ?? "");
}

function missing(text: string, points: Set<number>): string[] {
  return [...text].filter((c) => !points.has(c.codePointAt(0) ?? 0));
}

describe("glyph coverage", () => {
  it("covers UI text and the shortcut symbols in Lamina Sans' Latin files", () => {
    for (const style of ["normal", "italic"]) {
      const points = covered(`files/lamina-sans-latin-wght-${style}.woff2`);
      expect(missing(ASCII + WESTERN + PUNCTUATION + SHORTCUTS, points)).toEqual([]);
    }
  });

  it("covers titles and quotes in Newsreader's Latin files", () => {
    for (const file of ["newsreader-latin-wght-normal", "newsreader-latin-400-italic"]) {
      expect(missing(ASCII + WESTERN + PUNCTUATION, covered(`files/${file}.woff2`))).toEqual([]);
    }
  });

  it("leaves to the system font only the characters it lists", () => {
    const points = covered("files/lamina-sans-latin-wght-normal.woff2");
    expect(missing(SYSTEM_FALLBACK, points)).toEqual([...SYSTEM_FALLBACK]);
  });
});
