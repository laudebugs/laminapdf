import { describe, expect, it } from "vitest";
import { parseRange } from "../fonts/ranges.ts";
import { boxOf, frameOf } from "./geometry.ts";
import { colourIdOf, lacking, rgbOfColour } from "./inspector-props.ts";

describe("what the inspector reads off an object", () => {
  it("names a content colour it wrote, and none it didn't", () => {
    expect(colourIdOf(rgbOfColour("teal"))).toBe("teal");
    expect(colourIdOf([1, 0.5, 0])).toBeUndefined();
    expect(colourIdOf(undefined)).toBeUndefined();
  });

  it("shows a turned frame's position as the box it has unturned", () => {
    const g = boxOf(frameOf(100, 200, 80, 40, 90));
    expect(g.x).toBeCloseTo(100);
    expect(g.y).toBeCloseTo(200);
    expect(g.width).toBeCloseTo(80);
    expect(g.rotate).toBeCloseTo(90);
  });

  it("lists the characters a font lacks", () => {
    const font = {
      id: "f",
      family: "F",
      source: "bundled" as const,
      coverage: parseRange("U+0020-007E"),
      preferred: false,
    };
    expect(lacking(font, "abc → ©")).toEqual(["→", "©"]);
    expect(lacking(undefined, "→")).toEqual([]);
  });
});
