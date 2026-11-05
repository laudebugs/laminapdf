// fonts.css: one rule per file, the family names the tokens use, the fallback face's metrics.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { faces, lock } from "./helpers.ts";

/** The family list of one of @laminapdf/tokens' font stacks, from its tokens.css. */
function stack(name: string): string[] {
  const css = readFileSync(fileURLToPath(import.meta.resolve("@laminapdf/tokens/tokens.css")), "utf8");
  const value = css.match(new RegExp(`--paper-font-${name}:\\s*([^;]+);`))?.[1] ?? "";
  return value.split(",").map((family) => family.trim().replace(/^"|"$/g, ""));
}

describe("fonts.css", () => {
  it("declares each output once, with the lock's family, style and weight", () => {
    const declared = faces()
      .filter((face) => face.src?.startsWith("url("))
      .map((face) => ({
        file: face.src?.match(/url\("\.\/([^"]+)"\)/)?.[1],
        family: face["font-family"]?.replace(/"/g, ""),
        style: face["font-style"],
        weight: face["font-weight"],
        display: face["font-display"],
      }));
    const expected = Object.entries(lock().outputs).map(([file, output]) => ({
      file,
      family: output.family,
      style: output.style,
      weight: output.weight,
      display: "swap",
    }));
    expect(declared).toEqual(expected);
  });

  it("names the families the tokens' font stacks start with", () => {
    expect(stack("sans").slice(0, 2)).toEqual(["Lamina Sans", "Lamina Sans Fallback"]);
    expect(stack("serif")[0]).toBe("Newsreader");
    expect(stack("signature")[0]).toBe("Caveat");
  });

  it("sizes the fallback face with the lock's metrics", () => {
    const fallback = faces().find((face) => face["font-family"] === '"Lamina Sans Fallback"');
    const { sizeAdjust, ascentOverride, descentOverride, lineGapOverride } = lock().fallback;
    expect(fallback?.["size-adjust"]).toBe(`${sizeAdjust}%`);
    expect(fallback?.["ascent-override"]).toBe(`${ascentOverride}%`);
    expect(fallback?.["descent-override"]).toBe(`${descentOverride}%`);
    expect(fallback?.["line-gap-override"]).toBe(`${lineGapOverride}%`);
    expect(fallback?.src).toMatch(/^local\("Arial"\)/);
  });
});
