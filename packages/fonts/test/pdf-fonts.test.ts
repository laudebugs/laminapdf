// PDF_FONTS against fonts.lock.json: every TrueType file in pdf/ is listed once, with the family
// and coverage the lock records.

import { describe, expect, it } from "vitest";
import { PDF_FONTS } from "../src/index.ts";
import { lock } from "./helpers.ts";

describe("PDF_FONTS", () => {
  it("lists every TrueType file in pdf/ with its family and coverage", () => {
    const listed = PDF_FONTS.map((font) => [font.file, font.family, font.coverage, font.bytes]);
    const locked = Object.entries(lock().pdf).map(([file, output]) => [
      file,
      output.family,
      output.coverage,
      output.bytes,
    ]);
    expect(listed).toEqual(locked);
  });

  it("lists the picker's fonts in order, Noto Sans Symbols 2 the largest", () => {
    expect(PDF_FONTS.map((font) => font.id)).toEqual([
      "lamina-sans",
      "newsreader",
      "jetbrains-mono",
      "noto-sans-symbols-2",
      "noto-sans-arabic",
    ]);
    const largest = [...PDF_FONTS].sort((a, b) => b.bytes - a.bytes)[0];
    expect(largest?.id).toBe("noto-sans-symbols-2");
  });

  it("tries Lamina Sans, the symbols and the scripts first to fill in for another font", () => {
    expect(PDF_FONTS.filter((font) => font.preferred).map((font) => font.id)).toEqual([
      "lamina-sans",
      "noto-sans-symbols-2",
      "noto-sans-arabic",
    ]);
  });

  it("names each font once and gives it a URL beside the module", () => {
    expect(new Set(PDF_FONTS.map((font) => font.id)).size).toBe(PDF_FONTS.length);
    for (const font of PDF_FONTS) expect(font.href.endsWith(font.file)).toBe(true);
  });
});
