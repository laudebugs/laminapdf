// fonts.css: one rule per file, the family names the tokens use, the fallback faces' metrics.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { codePoints, faces, lock } from "./helpers.ts";

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

  it("gives each file a unicode-range that holds every character the file has", () => {
    const { outputs } = lock();
    for (const face of faces().filter((f) => f.src?.startsWith("url("))) {
      const file = face.src?.match(/url\("\.\/([^"]+)"\)/)?.[1] ?? "";
      const range = codePoints(face["unicode-range"] ?? "");
      const outside = [...codePoints(outputs[file]?.coverage ?? "")].filter((p) => !range.has(p));
      expect({ file, outside }).toEqual({ file, outside: [] });
    }
  });

  it("names the families the tokens' font stacks start with", () => {
    expect(stack("sans").slice(0, 2)).toEqual(["Lamina Sans", "Lamina Sans Fallback"]);
    expect(stack("serif")[0]).toBe("Newsreader");
    expect(stack("signature")[0]).toBe("Caveat");
  });

  it("gives each style and weight of the fallback its own Arial face and the lock's metrics", () => {
    const declared = faces()
      .filter((face) => face["font-family"] === '"Lamina Sans Fallback"')
      .map((face) => ({
        style: face["font-style"],
        weight: Number(face["font-weight"]),
        local: [...(face.src ?? "").matchAll(/local\("([^"]+)"\)/g)].map(([, name]) => name),
        sizeAdjust: face["size-adjust"],
        ascentOverride: face["ascent-override"],
        descentOverride: face["descent-override"],
        lineGapOverride: face["line-gap-override"],
      }));
    const expected = lock().fallbacks.map((f) => ({
      style: f.style,
      weight: f.weight,
      local: f.local,
      sizeAdjust: `${f.sizeAdjust}%`,
      ascentOverride: `${f.ascentOverride}%`,
      descentOverride: `${f.descentOverride}%`,
      lineGapOverride: `${f.lineGapOverride}%`,
    }));
    expect(declared).toEqual(expected);
    expect(declared.map((f) => [f.style, f.weight, f.local[0]])).toEqual([
      ["normal", 400, "Arial"],
      ["normal", 700, "Arial Bold"],
      ["italic", 400, "Arial Italic"],
      ["italic", 700, "Arial Bold Italic"],
    ]);
  });
});
