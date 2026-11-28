import { describe, expect, it } from "vitest";
import {
  cropDrawing,
  INKS,
  inkCss,
  MARGIN,
  MAX_UPLOAD,
  nameFor,
  removeWhite,
  TYPED_STYLES,
  thin,
  typedText,
  uploadProblem,
} from "./capture.ts";

describe("a drawn signature", () => {
  it("drops points that sit within a step of the one before, but keeps the end of the stroke", () => {
    expect(
      thin([
        [0, 0],
        [0.2, 0.2],
        [5, 0],
        [5.1, 0],
      ]),
    ).toEqual([
      [0, 0],
      [5, 0],
      [5.1, 0],
    ]);
  });

  it("is cropped to its ink, with a margin all round", () => {
    const drawing = cropDrawing(
      [
        [
          [100, 50],
          [140, 90],
        ],
      ],
      2,
    );
    const pad = MARGIN + 1;
    expect(drawing).toEqual({
      strokes: [
        [
          [pad, pad],
          [pad + 40, pad + 40],
        ],
      ],
      width: 40 + pad * 2,
      height: 40 + pad * 2,
    });
  });

  it("is nothing when no stroke has a point", () => {
    expect(cropDrawing([])).toBeUndefined();
    expect(cropDrawing([[]])).toBeUndefined();
  });

  it("has two inks, black and dark blue", () => {
    expect(inkCss(INKS.black)).toBe("rgb(0, 0, 0)");
    expect(inkCss(INKS.blue)).toBe("rgb(13, 38, 115)");
  });
});

/** The typed style `id`. */
function styleOf(id: string) {
  const style = TYPED_STYLES.find((s) => s.id === id);
  if (style === undefined) throw new Error(`no style ${id}`);
  return style;
}

describe("a typed signature", () => {
  it("collapses spaces, and the spaced style draws capitals", () => {
    expect([typedText("  Amara   Okafor ", styleOf("script")), typedText("Amara Okafor", styleOf("spaced"))]).toEqual([
      "Amara Okafor",
      "AMARA OKAFOR",
    ]);
  });

  it("is empty for a blank name", () => {
    expect(typedText("   ", styleOf("script"))).toBe("");
  });

  it("offers three styles", () => {
    expect(TYPED_STYLES.map((s) => s.id)).toEqual(["script", "italic", "spaced"]);
  });
});

describe("an uploaded picture", () => {
  it("loses its white background and keeps its ink; the shades between fade", () => {
    const pixels = new Uint8ClampedArray([255, 255, 255, 255, 10, 10, 10, 255, 222, 222, 222, 255]);
    const cleared = removeWhite(pixels);
    expect([cleared, pixels[3], pixels[7]]).toEqual([1, 0, 255]);
    expect(pixels[11]).toBeGreaterThan(0);
    expect(pixels[11]).toBeLessThan(255);
  });

  it("must be a PNG or JPEG under 10 MB, judged by its bytes", () => {
    const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0]);
    const big = new Uint8Array(MAX_UPLOAD + 1);
    big.set(png);
    expect([uploadProblem(png), uploadProblem(new Uint8Array([1, 2, 3])), uploadProblem(big)]).toEqual([
      undefined,
      "type",
      "size",
    ]);
  });
});

describe("a saved signature's name", () => {
  it("is the suggestion, else the fallback, numbered past a clash", () => {
    expect(nameFor("Amara Okafor", [], "My signature")).toBe("Amara Okafor");
    expect(nameFor("", [], "My signature")).toBe("My signature");
    expect(nameFor("", ["My signature", "My signature 2"], "My signature")).toBe("My signature 3");
  });
});
