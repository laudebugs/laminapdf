// The export's encoding helpers: the scale a resolution gives the renderers, and the resolution
// written into the canvas's PNG and JPEG files.
import { describe, expect, it } from "vitest";
import { scaleFor, withJpegDensity, withPngDensity } from "../src/worker/export.js";

describe("scaleFor", () => {
  it.each([72, 96, 150, 200, 300, 600])("never gives fewer pixels than %s dpi asks for", (dpi) => {
    const scale = scaleFor(dpi);
    expect(Math.fround(scale)).toBe(scale);
    expect(scale).toBeGreaterThanOrEqual(dpi / 72);
    expect(Math.floor(612 * scale)).toBe(Math.round((612 * dpi) / 72));
  });
});

/** A PNG's signature and IHDR, then IEND: enough for the chunk to go in after the IHDR. */
const PNG = Uint8Array.from([
  0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
  0, 0, 0x1f, 0x15, 0xc4, 0x89, 0, 0, 0, 0, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
]);

describe("withPngDensity", () => {
  it("adds a pHYs chunk after the IHDR, in pixels per metre, with its CRC", () => {
    const out = withPngDensity(PNG, 300);
    const view = new DataView(out.buffer);
    expect(String.fromCharCode(...out.subarray(37, 41))).toBe("pHYs");
    expect([view.getUint32(41), view.getUint32(45), out[49]]).toEqual([11811, 11811, 1]);
    // CRC-32 of "pHYs" and its nine bytes, as zlib computes it.
    expect(view.getUint32(50)).toBe(0x78a53f76);
    expect(String.fromCharCode(...out.subarray(58, 62))).toBe("IEND");
  });
});

describe("withJpegDensity", () => {
  it("sets the density of the canvas's JFIF segment", () => {
    const jfif = Uint8Array.from([
      0xff, 0xd8, 0xff, 0xe0, 0, 16, 0x4a, 0x46, 0x49, 0x46, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 0xff, 0xd9,
    ]);
    const out = withJpegDensity(jfif, 150);
    expect(Array.from(out.subarray(13, 18))).toEqual([1, 0, 150, 0, 150]);
    expect(out.length).toBe(jfif.length);
  });

  it("adds a JFIF segment to a JPEG without one", () => {
    const bare = Uint8Array.from([0xff, 0xd8, 0xff, 0xdb, 0, 2, 0xff, 0xd9]);
    const out = withJpegDensity(bare, 200);
    expect(String.fromCharCode(...out.subarray(6, 11))).toBe("JFIF\0");
    expect(Array.from(out.subarray(13, 18))).toEqual([1, 0, 200, 0, 200]);
    expect(Array.from(out.subarray(20))).toEqual([0xff, 0xdb, 0, 2, 0xff, 0xd9]);
  });
});
