// A text run's and a text box's `bottom` (B10a): where the next element can start. The same
// cases and numbers as crates/laminapdf-build/tests/bottom.rs, from Helvetica's AFM metrics
// (ascender 718, descender -207 per 1000 units), so Rust and TypeScript agree within 1e-6.
import { Builder, Font, init, type Surface } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";

beforeAll(() => init({ text: true }));

const SIZE = 12;
const ASCENT = (718 * SIZE) / 1000;
const DESCENT = (207 * SIZE) / 1000;
const LEADING = 1.2 * SIZE;
const PARAGRAPH = "Revenue rose twelve per cent over the quarter, led by the new reader and its offline mode.";

/** `draw` on a fresh A4 page with Helvetica registered. */
function onPage<T>(draw: (surface: Surface, font: Font) => T): T {
  const builder = new Builder();
  const font = Font.standard("Helvetica");
  builder.font(font);
  const page = builder.page("A4");
  try {
    return draw(page.surface(), font);
  } finally {
    builder.free();
  }
}

const run = (text: string) => onPage((s, font) => s.text(text, { font, size: SIZE, at: [72, 100] }));
const box = (text: string, w: number, h: number, extra: { lineHeight?: number } = {}) =>
  onPage((s, font) => s.textBox(text, { font, size: SIZE, box: [72, 100, w, h], ...extra }));
/** The descent edge of line `n` (0-based) of a box at y = 100. */
const lineBottom = (n: number, leading = LEADING) => 100 + ASCENT + n * leading + DESCENT;

describe("a text run's bottom", () => {
  it("is the descent below its last drawn line's baseline", () => {
    expect(run("Results").bottom).toBeCloseTo(100 + DESCENT, 6);
    expect(run("one\ntwo\nthree").bottom).toBeCloseTo(100 + 2 * LEADING + DESCENT, 6);
    expect(run("one\n\n  \n").bottom).toBeCloseTo(100 + DESCENT, 6);
  });

  it("is left out when nothing is drawn", () => {
    for (const text of ["", "   ", "\n", "\n\n", " \n \n"]) expect(run(text).bottom, JSON.stringify(text)).toBeUndefined();
  });
});

describe("a text box's bottom", () => {
  it("is its last line's descent edge, not the box's bottom", () => {
    const layout = box(PARAGRAPH, 160, 400);
    expect(layout.lines.length).toBeGreaterThanOrEqual(3);
    expect(layout.bottom).toBeCloseTo(lineBottom(layout.lines.length - 1), 6);
  });

  it("follows the line height", () => {
    expect(box("one\ntwo", 400, 400, { lineHeight: 2 }).bottom).toBeCloseTo(lineBottom(1, 2 * SIZE), 6);
  });

  it("ends at the last drawn line of a clipped box, which keeps its overflow", () => {
    const layout = box(PARAGRAPH, 160, ASCENT + LEADING + DESCENT + 1);
    expect(layout.lines).toHaveLength(2);
    expect(layout.overflow).toBeGreaterThan(0);
    expect(layout.bottom).toBeCloseTo(lineBottom(1), 6);
  });

  it("is left out when no line fits, and the overflow is then 0, which callers test with !== undefined", () => {
    const layout = box(PARAGRAPH, 160, 4);
    expect(layout.bottom).toBeUndefined();
    expect(layout.overflow).toBe(0);
    expect(layout.overflow !== undefined).toBe(true);
  });

  it("is left out for a box of only blanks", () => {
    expect(box("  \n \n", 160, 400).bottom).toBeUndefined();
  });
});
