import { describe, expect, it } from "vitest";
import { aliases, iconFiles } from "../scripts/files.ts";
import { emitIcons, readIcons } from "../scripts/icons.ts";
import { ICONS } from "../src/index.ts";
import { readRepo } from "./helpers.ts";

describe("the generated module", () => {
  const icons = readIcons(iconFiles(), aliases());

  it("is what the generator writes now", () => {
    expect(readRepo("packages/icons/src/generated/icons.ts")).toBe(emitIcons(icons));
  });

  it("holds every file and every alias, with an alias's markup equal to its icon's", () => {
    expect(Object.keys(ICONS).length).toBe(iconFiles().length + Object.keys(aliases()).length);
    for (const [alias, target] of Object.entries(aliases())) {
      expect(ICONS[alias as keyof typeof ICONS]).toBe(ICONS[target as keyof typeof ICONS]);
    }
  });

  it("refuses an alias of a missing icon, an alias that shadows a file, and a badly named file", () => {
    const files = iconFiles().slice(0, 1);
    const first = files[0]?.[0] ?? "";
    expect(() => readIcons(files, { x: "nope" })).toThrow(/names nope, which isn't an icon/);
    expect(() => readIcons(files, { [first]: first })).toThrow(/is also an icon file/);
    expect(() => readIcons([["Bad_Name", files[0]?.[1] ?? ""]], {})).toThrow(/lowercase words joined by hyphens/);
  });
});
