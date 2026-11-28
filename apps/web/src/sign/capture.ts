// Making a signature image (E10c, Paper G01), apart from the DOM so it runs in Node: a drawn
// signature's strokes thinned and cropped to what was drawn, the three typed styles, an uploaded
// picture's white background removed and the checks on what may be uploaded. render.ts turns these
// into a PNG on a canvas.

import { imageType } from "../comment/stamp-library.ts";

/** A stroke: `[x, y]` points in pixels on the pad. */
export type Stroke = readonly (readonly [number, number])[];

/** The most strokes and points a drawn signature keeps: a signature, not a drawing. */
export const MAX_STROKES = 64;
export const MAX_POINTS = 2000;
/** Points closer than this, in pixels, add nothing to a stroke. */
const MIN_STEP = 0.75;
/** The empty margin round a cropped drawing, in pixels. */
export const MARGIN = 6;
/** The largest picture that may be uploaded, in bytes. */
export const MAX_UPLOAD = 10 * 1024 * 1024;

/** `stroke` without points that sit within a step of the one before, the last point always kept. */
export function thin(stroke: Stroke): Stroke {
  const out: [number, number][] = [];
  for (const point of stroke) {
    const last = out[out.length - 1];
    if (last === undefined || Math.hypot(point[0] - last[0], point[1] - last[1]) >= MIN_STEP) {
      out.push([point[0], point[1]]);
    }
  }
  const end = stroke[stroke.length - 1];
  const last = out[out.length - 1];
  if (end !== undefined && last !== undefined && (last[0] !== end[0] || last[1] !== end[1])) out.push([end[0], end[1]]);
  return out.slice(0, MAX_POINTS);
}

/** A drawing cropped to its ink: its strokes moved to start at the margin, and its size. */
export interface Drawing {
  readonly strokes: readonly Stroke[];
  readonly width: number;
  readonly height: number;
}

/** `strokes` cropped to their bounds plus a margin; undefined when nothing was drawn. */
export function cropDrawing(strokes: readonly Stroke[], pen = 2): Drawing | undefined {
  const kept = strokes
    .slice(0, MAX_STROKES)
    .map(thin)
    .filter((s) => s.length > 0);
  if (kept.length === 0) return undefined;
  let minX = Number.POSITIVE_INFINITY;
  let minY = Number.POSITIVE_INFINITY;
  let maxX = Number.NEGATIVE_INFINITY;
  let maxY = Number.NEGATIVE_INFINITY;
  for (const stroke of kept) {
    for (const [x, y] of stroke) {
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
    }
  }
  const pad = MARGIN + pen / 2;
  return {
    strokes: kept.map((s) => s.map(([x, y]) => [x - minX + pad, y - minY + pad] as const)),
    width: Math.ceil(maxX - minX + pad * 2),
    height: Math.ceil(maxY - minY + pad * 2),
  };
}

/** The ink colours of the Draw tab, RGB 0 to 1: black and dark blue. */
export const INKS = {
  black: [0, 0, 0],
  blue: [0.05, 0.15, 0.45],
} as const satisfies Record<string, readonly [number, number, number]>;

/** An ink's name. */
export type InkName = keyof typeof INKS;

/** `ink` as a CSS colour. */
export function inkCss(ink: readonly [number, number, number]): string {
  const [r, g, b] = ink.map((c) => Math.round(c * 255));
  return `rgb(${r}, ${g}, ${b})`;
}

/** A typed signature's style. */
export interface TypedStyle {
  readonly id: "script" | "italic" | "spaced";
  /** The CSS `font` family stack, the first being a bundled face where there is one. */
  readonly family: string;
  /** The size in pixels the name is drawn at. */
  readonly size: number;
  /** Letter spacing in pixels. */
  readonly spacing: number;
  readonly italic: boolean;
  /** Draws the name in capitals. */
  readonly upper: boolean;
}

/** The three styles of the Type tab: handwriting, italic and spaced capitals. */
export const TYPED_STYLES: readonly TypedStyle[] = [
  {
    id: "script",
    family: '"Caveat", "Segoe Script", "Bradley Hand", "Apple Chancery", cursive',
    size: 64,
    spacing: 0,
    italic: false,
    upper: false,
  },
  {
    id: "italic",
    family: '"Newsreader", "Iowan Old Style", Georgia, serif',
    size: 56,
    spacing: 0,
    italic: true,
    upper: false,
  },
  {
    id: "spaced",
    family: '"Newsreader", "Iowan Old Style", Georgia, serif',
    size: 44,
    spacing: 6,
    italic: false,
    upper: true,
  },
];

/** The text a typed signature draws: the name trimmed, in capitals for the spaced style; empty for a blank name. */
export function typedText(name: string, style: TypedStyle): string {
  const text = name.replace(/\s+/g, " ").trim().slice(0, 80);
  return style.upper ? text.toLocaleUpperCase() : text;
}

/**
 * Makes the near-white pixels of `rgba` (four bytes a pixel, a canvas's `ImageData`) transparent,
 * and fades the pixels between: a photographed or scanned signature on paper becomes ink alone.
 * Returns how many pixels it cleared.
 */
export function removeWhite(rgba: Uint8ClampedArray, lowest = 200, highest = 245): number {
  let cleared = 0;
  for (let i = 0; i + 3 < rgba.length; i += 4) {
    const alpha = rgba[i + 3] ?? 255;
    const lightest = Math.min(rgba[i] ?? 0, rgba[i + 1] ?? 0, rgba[i + 2] ?? 0);
    if (lightest >= highest) {
      rgba[i + 3] = 0;
      cleared += 1;
    } else if (lightest > lowest) {
      const keep = (highest - lightest) / (highest - lowest);
      rgba[i + 3] = Math.round(alpha * keep);
    }
  }
  return cleared;
}

/** Why an uploaded file can't be used, or undefined when it can. */
export function uploadProblem(bytes: Uint8Array): "type" | "size" | undefined {
  if (bytes.length > MAX_UPLOAD) return "size";
  return imageType(bytes) === undefined ? "type" : undefined;
}

/** A name for a saved signature: the typed text, else "My signature" with no clash among `taken`. */
export function nameFor(suggested: string, taken: readonly string[], fallback: string): string {
  const base = suggested.trim() === "" ? fallback : suggested.trim().slice(0, 60);
  if (!taken.includes(base)) return base;
  for (let n = 2; ; n += 1) {
    const candidate = `${base} ${n}`;
    if (!taken.includes(candidate)) return candidate;
  }
}
