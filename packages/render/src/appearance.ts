// Page appearance (ADR 0024 §5): a page drawn in Sepia, Dark or High contrast instead of its own
// colours. The port of laminapdf-render's `appearance` module, for adapters written in
// TypeScript: integer arithmetic only, so both give the same bytes (fixtures/appearance/colours.json
// holds both to it). A tile is drawn as usual, then recoloured pixel by pixel, except where it
// shows an image (each adapter lists where its own parser placed them) or an annotation (the
// tile drawn again with annotations: what they change comes from that drawing, and so does the
// whole rectangle of one that changes most of it, so text under a highlight keeps its colour).

import type { Rgba8 } from "./index.js";

/** How the page's colours are drawn: its own, Sepia, Dark, or High contrast with two colours. */
export type PageAppearance =
  | "original"
  | "sepia"
  | "dark"
  | {
      readonly kind: "high-contrast";
      /** What black becomes: usually the system's `CanvasText`. */
      readonly ink: Rgba8;
      /** What white becomes: usually the system's `Canvas`. */
      readonly paper: Rgba8;
    };

/** An appearance's paper (what white becomes) and ink (what black becomes). */
export interface PagePalette {
  readonly paper: Rgba8;
  readonly ink: Rgba8;
}

/** Sepia's paper and ink: `@laminapdf/tokens`' `PAGE_APPEARANCES.sepia` (a test holds them to it). */
export const SEPIA: PagePalette = Object.freeze({
  paper: Object.freeze({ r: 0xf4, g: 0xec, b: 0xd8, a: 255 }),
  ink: Object.freeze({ r: 0x3b, g: 0x2f, b: 0x20, a: 255 }),
});

/** Dark's paper and ink: `@laminapdf/tokens`' `PAGE_APPEARANCES.dark`. */
export const DARK: PagePalette = Object.freeze({
  paper: Object.freeze({ r: 0x22, g: 0x21, b: 0x1f, a: 255 }),
  ink: Object.freeze({ r: 0xe6, g: 0xe1, b: 0xd8, a: 255 }),
});

/** Whether `appearance` changes any colour. */
export function recolours(appearance: PageAppearance | undefined): boolean {
  return appearance !== undefined && appearance !== "original";
}

/** Rec. 601 luma in 8.8 fixed point, 0–255. */
function luma(r: number, g: number, b: number): number {
  return Math.min(255, (77 * r + 150 * g + 29 * b + 128) >> 8);
}

/** `round(v / 255)` for `v` ≤ 255 × 255. */
function div255(v: number): number {
  return Math.min(255, Math.floor((v + 127) / 255));
}

/** `a` at `t` = 0 to `b` at `t` = 255, rounded. */
function lerp(a: number, b: number, t: number): number {
  return div255(a * (255 - t) + b * t);
}

function clamp(v: number): number {
  return Math.max(0, Math.min(255, v));
}

/** The colour `[r, g, b]` (straight sRGB) becomes in `appearance`; alpha is never changed. */
export function recolourRgb(appearance: PageAppearance, r: number, g: number, b: number): [number, number, number] {
  const y = luma(r, g, b);
  if (appearance === "original") return [r, g, b];
  if (appearance === "sepia") {
    // Each channel from ink (0) to paper (255): black is ink, white is paper.
    const { paper, ink } = SEPIA;
    return [lerp(ink.r, paper.r, r), lerp(ink.g, paper.g, g), lerp(ink.b, paper.b, b)];
  }
  if (appearance === "dark") {
    // The lightness from ink (black) to paper (white), plus the colour's offset from its
    // lightness, so a blue bar stays bluish.
    const { paper, ink } = DARK;
    const shift = (from: number, to: number, c: number) => clamp(lerp(from, to, y) + c - y);
    return [shift(ink.r, paper.r, r), shift(ink.g, paper.g, g), shift(ink.b, paper.b, b)];
  }
  // Squaring the lightness pushes greys toward ink, so light-grey text stays readable.
  const { ink, paper } = appearance;
  const t = div255(y * y);
  return [lerp(ink.r, paper.r, t), lerp(ink.g, paper.g, t), lerp(ink.b, paper.b, t)];
}

/** A quadrilateral in tile pixels: four `[x, y]` corners. */
export type Quad = readonly [
  readonly [number, number],
  readonly [number, number],
  readonly [number, number],
  readonly [number, number],
];

/** The unit square under the affine `[a, b, c, d, e, f]` (to tile pixels). */
export function quadOf([a, b, c, d, e, f]: readonly number[]): Quad {
  const at = (x: number, y: number): [number, number] => [
    (a ?? 0) * x + (c ?? 0) * y + (e ?? 0),
    (b ?? 0) * x + (d ?? 0) * y + (f ?? 0),
  ];
  return [at(0, 0), at(1, 0), at(1, 1), at(0, 1)];
}

/** Whether the point is inside the quad (on its edge counts), whichever way it winds. */
function contains(quad: Quad, x: number, y: number): boolean {
  const side = (p: readonly [number, number], q: readonly [number, number]) =>
    (q[0] - p[0]) * (y - p[1]) - (q[1] - p[1]) * (x - p[0]);
  const s = [side(quad[0], quad[1]), side(quad[1], quad[2]), side(quad[2], quad[3]), side(quad[3], quad[0])];
  return s.every((v) => v >= 0) || s.every((v) => v <= 0);
}

/** The pixel columns and rows whose centres can be inside `quad`, or `undefined` if none can. */
function bounds(quad: Quad, width: number, height: number): [number, number, number, number] | undefined {
  const xs = quad.map((p) => p[0]);
  const ys = quad.map((p) => p[1]);
  const [x0, x1, y0, y1] = [Math.min(...xs), Math.max(...xs), Math.min(...ys), Math.max(...ys)];
  if (![x0, x1, y0, y1].every(Number.isFinite)) return undefined;
  // Pixel centres at i + 0.5: the first and last columns whose centre can be inside.
  const cx0 = Math.max(0, Math.ceil(x0 - 0.5));
  const cx1 = Math.min(width - 1, Math.floor(x1 - 0.5));
  const cy0 = Math.max(0, Math.ceil(y0 - 0.5));
  const cy1 = Math.min(height - 1, Math.floor(y1 - 0.5));
  return cx0 > cx1 || cy0 > cy1 ? undefined : [cx0, cx1, cy0, cy1];
}

/** Whether `quad` covers the centre of any pixel of a `width` × `height` tile (its bounding box does, at least). */
export function touches(quad: Quad, width: number, height: number): boolean {
  return bounds(quad, width, height) !== undefined;
}

/** The indices of the pixels whose centres are inside `quad`, in a `width` × `height` tile. */
function pixelsIn(quad: Quad, width: number, height: number): number[] {
  const box = bounds(quad, width, height);
  if (box === undefined) return [];
  const [cx0, cx1, cy0, cy1] = box;
  const out: number[] = [];
  for (let y = cy0; y <= cy1; y++) {
    for (let x = cx0; x <= cx1; x++) {
      if (contains(quad, x + 0.5, y + 0.5)) out.push(y * width + x);
    }
  }
  return out;
}

/** What a recoloured tile keeps as drawn. */
export interface Kept {
  /** Opaque images: every pixel inside is kept. */
  readonly images?: readonly Quad[];
  /**
   * Images the page shows through (a soft mask, a mask, a blend other than Normal): inside,
   * only pixels that differ from `background` are kept, so the paper around a masked shape is
   * recoloured with the page.
   */
  readonly seeThrough?: readonly Quad[];
  /** The tile's background; `null` is transparent. */
  readonly background?: Rgba8 | null;
  /**
   * The tile drawn again with its annotations, and the rectangles `mostlyChanged` picked: every
   * pixel inside them comes from that drawing, so text under a highlight keeps its colour.
   */
  readonly annotated?: { readonly pixels: Uint8ClampedArray; readonly whole: readonly Quad[] };
}

/** Each pixel's fate: 0 recoloured, KEEP as drawn, ANNOTATION from the annotated drawing. */
const KEEP = 1;
const ANNOTATION = 2;

/**
 * Recolours a tile drawn without annotations, in place: straight RGBA8 rows of `width × 4`.
 * Opaque images keep their pixels, see-through ones all but the background's; annotations as
 * the module comment says.
 */
export function recolourTile(
  appearance: PageAppearance,
  width: number,
  pixels: Uint8ClampedArray,
  kept: Kept = {},
): void {
  if (!recolours(appearance) || width === 0) return;
  const count = pixels.length >> 2;
  const height = Math.floor(count / width);
  const fates = new Uint8Array(count);
  markImages(fates, pixels, kept, width, height);
  const annotated = kept.annotated;
  if (annotated !== undefined) markAnnotations(fates, pixels, annotated.pixels, annotated.whole, width, height);
  for (let i = 0, o = 0; i < count; i++, o += 4) {
    const fate = fates[i];
    if (fate === KEEP) continue;
    if (fate === ANNOTATION && annotated !== undefined) {
      pixels.set(annotated.pixels.subarray(o, o + 4), o);
      continue;
    }
    const [r, g, b] = recolourRgb(appearance, pixels[o] ?? 0, pixels[o + 1] ?? 0, pixels[o + 2] ?? 0);
    pixels[o] = r;
    pixels[o + 1] = g;
    pixels[o + 2] = b;
  }
}

/** Marks the pixels images keep: all of an opaque one's, a see-through one's but the background. */
function markImages(fates: Uint8Array, pixels: Uint8ClampedArray, kept: Kept, width: number, height: number): void {
  for (const quad of kept.images ?? []) for (const i of pixelsIn(quad, width, height)) fates[i] = KEEP;
  const bg = kept.background ?? { r: 0, g: 0, b: 0, a: 0 };
  const showsPaper = (o: number) =>
    pixels[o] === bg.r && pixels[o + 1] === bg.g && pixels[o + 2] === bg.b && pixels[o + 3] === bg.a;
  for (const quad of kept.seeThrough ?? []) {
    for (const i of pixelsIn(quad, width, height)) if (!showsPaper(i * 4)) fates[i] = KEEP;
  }
}

/** Whether pixel `i` differs between the two drawings. */
function changed(plain: Uint8ClampedArray, over: Uint8ClampedArray, i: number): boolean {
  const o = i * 4;
  return (
    plain[o] !== over[o] || plain[o + 1] !== over[o + 1] || plain[o + 2] !== over[o + 2] || plain[o + 3] !== over[o + 3]
  );
}

/**
 * Which annotations change at least half of their rectangle (a highlight, a filled shape, a
 * stamp), decided once for the page: `plain` and `over` are the whole page drawn small, without
 * and with annotations, `width` pixels wide, and `rects` the rectangles in its pixels. Deciding
 * per tile instead would split an annotation that two tiles share.
 */
export function mostlyChanged(
  rects: readonly Quad[],
  plain: Uint8ClampedArray,
  over: Uint8ClampedArray,
  width: number,
): boolean[] {
  const height = Math.floor(plain.length / 4 / width);
  return rects.map((rect) => {
    const inside = pixelsIn(rect, width, height);
    // Too small to cover a pixel of the small drawing: kept whole, as a highlight that small would be.
    return inside.length === 0 || inside.filter((i) => changed(plain, over, i)).length * 2 >= inside.length;
  });
}

/** Marks the pixels taken from the annotated drawing: the whole rectangles, and every other pixel the annotations change. */
function markAnnotations(
  fates: Uint8Array,
  plain: Uint8ClampedArray,
  over: Uint8ClampedArray,
  whole: readonly Quad[],
  width: number,
  height: number,
): void {
  for (const rect of whole) for (const i of pixelsIn(rect, width, height)) fates[i] = ANNOTATION;
  for (let i = 0; i < fates.length; i++) if (changed(plain, over, i)) fates[i] = ANNOTATION;
}
