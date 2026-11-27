// What added text is set in, as the inspector and the typing box have it: the registry's font,
// size, style, alignment and colour, read off a text block or typed, and the CSS that draws it in
// the browser while it is typed. The ops take `TextLook`s (ops.ts); this is the same look with
// the font as the registry names it, which the document names by blob IDs only when the text is saved.

import type { TextBlock } from "@laminapdf/sdk";
import { type Rgb, rgbOf, type ShapeLook, type TextLook } from "./ops.ts";

/** A text look with the registry's font it is set in. */
export interface Look extends Omit<TextLook, "fonts"> {
  readonly fontId: string;
}

export const INK: Rgb = [38 / 255, 36 / 255, 33 / 255];

/** The default look of new text. */
export const TEXT_DEFAULT: Look = {
  fontId: "helvetica",
  size: 12,
  color: INK,
  align: "left",
  bold: false,
  italic: false,
};

/** The default look of a new shape: an ink outline, no fill. */
export const SHAPE_DEFAULT: ShapeLook = { fill: null, stroke: { color: INK, width: 1.5, dash: "solid" } };

/** CSS font stack for the typing box, near the PDF font it will be saved in. */
export function cssFamily(fontId: string): string {
  switch (fontId) {
    case "times":
    case "newsreader":
      return '"Times New Roman", Newsreader, serif';
    case "courier":
    case "jetbrains-mono":
      return '"Courier New", monospace';
    default:
      return "Helvetica, Arial, sans-serif";
  }
}

/** The `rgb()` of a colour. */
export const css = ([r, g, b]: Rgb) => `rgb(${Math.round(r * 255)} ${Math.round(g * 255)} ${Math.round(b * 255)})`;

/** The look of text block `block`, set in the registry font `fontId` when this session knows it, else in what the block names if that is a standard font. */
export function lookOfBlock(block: TextBlock, fontId: string | undefined): Look {
  const named = block.fonts?.[0];
  return {
    fontId: fontId ?? (named !== undefined && STANDARD.includes(named) ? named : "helvetica"),
    size: block.size ?? 12,
    color: rgbOf(block.color) ?? INK,
    align: (block.align ?? "left") as Look["align"],
    bold: block.runs.length > 0 && block.runs.every((r) => r.bold === true),
    italic: block.runs.length > 0 && block.runs.every((r) => r.italic === true),
  };
}

const STANDARD: readonly string[] = ["helvetica", "times", "courier"];
