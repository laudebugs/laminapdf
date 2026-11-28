import { describe, expect, it } from "vitest";
import { parseRange } from "../fonts/ranges.ts";
import { boxOf, frameOf } from "./geometry.ts";
import { colourIdOf, cropOf, lacking, rgbOfColour, withInset } from "./inspector-props.ts";

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

  it("makes a crop of insets, none when nothing is cut", () => {
    expect(cropOf({ left: 10, top: 0, right: 0, bottom: 5 }, 100, 50)).toEqual([0.1, 0, 1, 0.9]);
    expect(cropOf({ left: 0, top: 0, right: 0, bottom: 0 }, 100, 50)).toBeNull();
  });

  it("stops an inset short of the opposite edge", () => {
    const none = { left: 0, top: 0, right: 0, bottom: 0 };
    expect(withInset(none, "right", 500, 100, 50).right).toBe(96);
    expect(withInset({ ...none, left: 30 }, "right", 90, 100, 50).right).toBe(66);
    expect(withInset(none, "top", -5, 100, 50).top).toBe(0);
  });
});
