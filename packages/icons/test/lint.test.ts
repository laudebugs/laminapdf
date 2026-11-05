import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { iconFiles } from "../scripts/files.ts";
import { lintIcon } from "../scripts/rules.ts";
import { boundingBox, parseSvg, pathPoints } from "../scripts/svg.ts";

const fixture = (name: string) => readFileSync(new URL(`fixtures/${name}.svg`, import.meta.url), "utf8");

describe("the icon lint", () => {
  it("passes every icon in svg/", () => {
    const errors = iconFiles().flatMap(([name, text]) => lintIcon(name, text).errors);
    expect(errors).toEqual([]);
  });

  it.each([
    ["stroke-2", 'stroke-2: <svg> needs stroke-width="1.5", has "2"'],
    ["fill", "fill: <rect> may not carry fill"],
    ["transform", "transform: <path> may not carry transform"],
    ["style", "style: <path> may not carry style"],
    ["edge", "edge: margin 1 is under 2 units"],
    ["dot", "dot: a circle of radius 0.9 draws a dot; only the icons drawn with dots may"],
  ])("fails %s", (name, message) => {
    expect(lintIcon(name, fixture(name)).errors).toContain(message);
  });

  it("has a negative fixture for each of those cases", () => {
    const names = readdirSync(new URL("fixtures/", import.meta.url)).map((f) => f.replace(/\.svg$/, ""));
    expect(names.sort()).toEqual(["dot", "edge", "fill", "stroke-2", "style", "transform"]);
  });

  it("reports bad geometry by icon instead of throwing", () => {
    const wrap = (inner: string) =>
      `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">${inner}</svg>`;
    expect(lintIcon("words", wrap('<path d="hello"></path>')).errors).toEqual([
      'words: "h" needs 1 numbers per segment',
    ]);
    expect(lintIcon("nan", wrap('<rect x="foo" y="0" width="30" height="30"></rect>')).errors).toEqual([
      "nan: a shape's geometry isn't a set of numbers",
    ]);
    expect(lintIcon("empty", wrap('<path d=""></path>')).errors).toEqual([
      "empty: a shape's geometry isn't a set of numbers",
    ]);
    // A zero-length arc draws nothing (SVG 1.1 F.6.2); the rest of the path still counts.
    expect(lintIcon("arc", wrap('<path d="M0 0L30 30M5 5A3 3 0 0 1 5 5"></path>')).errors).toEqual([
      "arc: margin -6 is under 2 units",
    ]);
  });

  it("reads an arc's flags written without spaces", () => {
    expect(pathPoints("M5 5a2 2 0 012 2").at(-1)).toEqual([7, 7]);
  });

  it("refuses text, nesting and other elements", () => {
    const wrap = (inner: string) => `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">${inner}</svg>`;
    expect(() => parseSvg(wrap("hello"))).toThrow(/unexpected text/);
    expect(() => parseSvg(wrap("<g><path d='M0 0'></path></g>"))).toThrow();
    expect(lintIcon("x", wrap('<text x="1"></text>')).errors).toContain("x: <text> is not a shape the icons use");
  });
});

describe("the bounding box", () => {
  it("follows lines, relative moves and implicit line-tos", () => {
    const xs = pathPoints("M4 5l2 3 1 1H20v-2M3 3m1 1").map((p) => p.join(","));
    expect(xs).toEqual(["4,5", "6,8", "7,9", "20,9", "20,7", "3,3", "4,4"]);
  });

  it("measures an arc by its drawn curve, not its radius", () => {
    // Reset's arc: radius 7.5 around (11.86, 11.54), worked out by hand; its endpoints ± the
    // radius would put it 3 units off the grid.
    const box = boundingBox(parseSvg(`<svg><path d="M5 8.5A7.5 7.5 0 1 1 4.5 13"></path></svg>`).shapes);
    expect(box.minX).toBeCloseTo(4.5, 1); // the end point: the arc stops short of its leftmost point
    expect(box.maxX).toBeCloseTo(19.36, 1);
    expect(box.minY).toBeCloseTo(4.04, 1);
  });

  it("measures a cubic curve by its points, not its control points", () => {
    const box = boundingBox(parseSvg(`<svg><path d="M4 12C4 2 20 2 20 12"></path></svg>`).shapes);
    expect(box.minY).toBeCloseTo(4.5, 1);
  });
});
