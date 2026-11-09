import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  DARK,
  isRenderError,
  mostlyChanged,
  type PageAppearance,
  planTile,
  quadOf,
  recolourRgb,
  recolourTile,
  SEPIA,
} from "../src/index.js";

interface Table {
  readonly highContrast: { readonly ink: number[]; readonly paper: number[] };
  readonly samples: readonly Record<string, number[]>[];
}

const table = JSON.parse(
  readFileSync(new URL("../../../fixtures/appearance/colours.json", import.meta.url), "utf8"),
) as Table;

const rgba = ([r = 0, g = 0, b = 0]: number[]) => ({ r, g, b, a: 255 });

describe("page appearance colours", () => {
  it("map every sample exactly as laminapdf-render does (fixtures/appearance/colours.json)", () => {
    const modes: [string, PageAppearance][] = [
      ["original", "original"],
      ["sepia", "sepia"],
      ["dark", "dark"],
      [
        "high-contrast",
        { kind: "high-contrast", ink: rgba(table.highContrast.ink), paper: rgba(table.highContrast.paper) },
      ],
    ];
    for (const sample of table.samples) {
      const [r = 0, g = 0, b = 0] = sample.rgb ?? [];
      for (const [name, mode] of modes)
        expect(recolourRgb(mode, r, g, b), `${name} ${sample.rgb}`).toEqual(sample[name]);
    }
  });
});

describe("recolourTile", () => {
  const ink = [DARK.ink.r, DARK.ink.g, DARK.ink.b];
  const paper = [DARK.paper.r, DARK.paper.g, DARK.paper.b];

  it("keeps an opaque image's pixels and every alpha", () => {
    const pixels = new Uint8ClampedArray([0, 0, 0, 255, 0, 0, 0, 128]);
    recolourTile("dark", 2, pixels, { images: [quadOf([1, 0, 0, 1, 0, 0])] });
    expect([...pixels]).toEqual([0, 0, 0, 255, ...ink, 128]);
  });

  it("recolours the paper inside a see-through image", () => {
    const pixels = new Uint8ClampedArray([255, 255, 255, 255, 40, 120, 200, 255]);
    recolourTile("dark", 2, pixels, {
      seeThrough: [quadOf([2, 0, 0, 1, 0, 0])],
      background: { r: 255, g: 255, b: 255, a: 255 },
    });
    expect([...pixels]).toEqual([...paper, 255, 40, 120, 200, 255]);
  });

  it("gives a mostly changed annotation its whole rectangle, letters under it included", () => {
    const pixels = new Uint8ClampedArray([255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255]);
    const over = new Uint8ClampedArray([255, 255, 0, 255, 0, 0, 0, 255, 255, 255, 0, 255]);
    const rect = quadOf([3, 0, 0, 1, 0, 0]);
    expect(mostlyChanged([rect], pixels, over, 3)).toEqual([true]);
    recolourTile("dark", 3, pixels, { annotated: { pixels: over, whole: [rect] } });
    expect([...pixels]).toEqual([...over]);
  });

  it("leaves the tile alone in the original appearance", () => {
    const pixels = new Uint8ClampedArray([12, 34, 56, 255]);
    recolourTile("original", 1, pixels);
    expect([...pixels]).toEqual([12, 34, 56, 255]);
  });
});

describe("planTile", () => {
  const page = { width: 100, height: 100, rotate: 0 as const };
  const limits = { maxRenderPixels: 1e7, maxRenderSide: 1e4, maxInputBytes: 1e9 };

  it("refuses an unknown appearance and a high contrast colour out of range", () => {
    for (const appearance of [
      "night" as unknown as PageAppearance,
      { kind: "high-contrast", ink: { r: 256, g: 0, b: 0, a: 255 }, paper: { r: 0, g: 0, b: 0, a: 255 } } as const,
    ]) {
      let error: unknown;
      try {
        planTile(page, { page: 0, scale: 1, appearance }, limits);
      } catch (e) {
        error = e;
      }
      expect(isRenderError(error, "invalid-argument")).toBe(true);
    }
  });
});

describe("the palettes", () => {
  it("match @laminapdf/tokens' Sepia and Dark paper and ink", () => {
    const tokens = readFileSync(new URL("../../tokens/src/generated/tokens.ts", import.meta.url), "utf8");
    const hex = (c: { r: number; g: number; b: number }) =>
      `#${[c.r, c.g, c.b].map((v) => v.toString(16).padStart(2, "0").toUpperCase()).join("")}`;
    for (const [name, palette] of [
      ["sepia", SEPIA],
      ["dark", DARK],
    ] as const) {
      // PAGE_APPEARANCES.<name>.<accent> lists paper, then ink; every accent shares them.
      const block = tokens.slice(tokens.indexOf(`"${name}": {`, tokens.indexOf("PAGE_APPEARANCES")));
      const [, paper, ink] = block.match(/"paper": "(#[0-9A-F]{6})",\s*"ink": "(#[0-9A-F]{6})"/) ?? [];
      expect([paper, ink], name).toEqual([hex(palette.paper), hex(palette.ink)]);
    }
  });
});
