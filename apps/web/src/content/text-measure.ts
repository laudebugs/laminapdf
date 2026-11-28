// How tall added text will be, so a box set to a new size or font holds its text. The core lays the
// text out itself; this only estimates from the browser's own metrics (a canvas measuring the
// same family and size), a little generous, so the box clips nothing.

/** Lines of text take this many times their size, plus a point of room. */
const LEADING = 1.25;

let canvas: CanvasRenderingContext2D | null | undefined;

/** The width of `text` at `size` px in `font`, by the browser's metrics. */
function widthOf(text: string, font: string): number {
  canvas ??= document.createElement("canvas").getContext("2d");
  if (canvas === null || canvas === undefined) return text.length * 0.5 * Number.parseFloat(font);
  canvas.font = font;
  return canvas.measureText(text).width;
}

/** How many lines `text` takes in a box `width` points wide: a line feed starts one, and words wrap. */
function linesOf(text: string, width: number, font: string): number {
  let lines = 0;
  for (const paragraph of text.split("\n")) {
    let used = 0;
    let count = 1;
    for (const word of paragraph.split(/(\s+)/)) {
      const w = widthOf(word, font);
      if (used + w > width && used > 0 && word.trim() !== "") {
        count += 1;
        used = w;
      } else {
        used += w;
      }
    }
    lines += count;
  }
  return Math.max(1, lines);
}

/** A box's height in points for `text` set at `size` in CSS font `family`, `width` wide. */
export function heightFor(
  text: string,
  width: number,
  size: number,
  family: string,
  bold: boolean,
  italic: boolean,
): number {
  const font = `${italic ? "italic " : ""}${bold ? "700 " : ""}${size}px ${family}`;
  return Math.ceil(linesOf(text, width, font) * size * LEADING + 1);
}
