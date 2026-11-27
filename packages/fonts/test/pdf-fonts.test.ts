// PDF_FONTS against fonts.lock.json: every TrueType file in pdf/ is listed once, with the family
// and coverage the lock records.

import { describe, expect, it } from "vitest";
import { PDF_FONTS } from "../src/index.ts";
import { lock } from "./helpers.ts";

describe("PDF_FONTS", () => {
  it("lists every TrueType file in pdf/ with its family and coverage", () => {
    const listed = PDF_FONTS.map((font) => [font.file, font.family, font.coverage]);
    const locked = Object.entries(lock().pdf).map(([file, output]) => [file, output.family, output.coverage]);
    expect(listed).toEqual(locked);
  });

  it("names each font once and gives it a URL beside the module", () => {
    expect(new Set(PDF_FONTS.map((font) => font.id)).size).toBe(PDF_FONTS.length);
    for (const font of PDF_FONTS) expect(font.href.endsWith(font.file)).toBe(true);
  });
});
