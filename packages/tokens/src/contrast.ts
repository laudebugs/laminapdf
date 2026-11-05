// WCAG 2.x contrast: relative luminance and the contrast ratio (WCAG 2.2, "Definitions":
// relative luminance, contrast ratio), and source-over compositing of a translucent colour, so a
// hover fill or the page pill is measured as it looks on what it sits over.

/** A straight (not premultiplied) sRGB colour: channels 0–255, alpha 0–1. */
export interface Rgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

/** `#RRGGBB` as an opaque colour. */
export function hex(value: string): Rgba {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(value);
  if (m === null) throw new Error(`not a #RRGGBB colour: ${value}`);
  const [r, g, b] = m.slice(1).map((h) => Number.parseInt(h, 16)) as [number, number, number];
  return { r, g, b, a: 1 };
}

function channel(c: number): number {
  const s = c / 255;
  // 0.04045 as WCAG 2.2 writes it; WCAG 2.0's 0.03928 gives the same result for 8-bit values.
  return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
}

/** The relative luminance of an opaque colour, 0 (black) to 1 (white). */
function luminance(c: Rgba): number {
  return 0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b);
}

/** The contrast ratio of two opaque colours, 1 to 21, in either order. */
export function contrastRatio(a: Rgba, b: Rgba): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x) as [number, number];
  return (hi + 0.05) / (lo + 0.05);
}

/** `top` drawn over the opaque `bottom` (source-over), rounded to whole channels as a screen shows it. */
export function composite(top: Rgba, bottom: Rgba): Rgba {
  const mix = (t: number, b: number) => Math.round(top.a * t + (1 - top.a) * b);
  return { r: mix(top.r, bottom.r), g: mix(top.g, bottom.g), b: mix(top.b, bottom.b), a: 1 };
}
