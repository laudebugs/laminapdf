// How the viewer's marks over a page land on their designed colours on each page appearance
// (docs/bugs/selection-on-dark-pages). The page is a canvas the viewer can't draw under, so a
// mark lies over it:
//
// - The selection is translucent: the design language defines `selection-tint` as the accent at
//   22% over the paper (the dark accent at 35% on a Dark page), so that accent at that opacity
//   lands on the tint, and shows over the page's pictures too.
// - A find mark is blended, so the ink under its strong amber stays as drawn: `multiply` on a light
//   paper, `screen` on a dark one, with the fill solved so paper and fill give the designed colour.
//
// High contrast's colours are the system's, unknown here; the generator writes its values as CSS.

import { luminance, type Rgba } from "../src/contrast.ts";

/** The appearances whose page colours are known when the tokens are built; High contrast's are the system's. */
export const MARKED = ["original", "sepia", "dark"] as const;

/** The blend a find mark is drawn with: multiply darkens a light paper, screen lightens a dark one. */
export type Blend = "multiply" | "screen";

const channels = ["r", "g", "b"] as const;

/** WCAG relative luminance below which a paper counts as dark (mid-grey, `#767676`, is 0.18). */
const DARK_PAPER = 0.18;

const clamp = (v: number) => Math.max(0, Math.min(255, Math.round(v)));

/** What `blend` of `fill` over `under` shows, in whole channels. */
export function blended(blend: Blend, under: Rgba, fill: Rgba): Rgba {
  const one = (u: number, f: number) => (blend === "multiply" ? (u * f) / 255 : 255 - ((255 - u) * (255 - f)) / 255);
  return { r: clamp(one(under.r, fill.r)), g: clamp(one(under.g, fill.g)), b: clamp(one(under.b, fill.b)), a: 1 };
}

/**
 * The blend and fill that draw `colour` over `paper`: per channel, colour ÷ paper under multiply,
 * 1 − (1 − colour) ÷ (1 − paper) under screen. A channel the blend can't reach (screen can't go
 * below the paper, multiply above it) is clamped, so it lands as near as the blend allows.
 */
export function blendFill(paper: Rgba, colour: Rgba): { blend: Blend; fill: Rgba } {
  const blend: Blend = luminance(paper) < DARK_PAPER ? "screen" : "multiply";
  const solve = (p: number, c: number) =>
    blend === "multiply" ? (p === 0 ? 0 : (c * 255) / p) : p === 255 ? 255 : 255 - ((255 - c) * 255) / (255 - p);
  const [r, g, b] = channels.map((k) => clamp(solve(paper[k], colour[k]))) as [number, number, number];
  return { blend, fill: { r, g, b, a: 1 } };
}

/**
 * The selection overlay: `accent` at the opacity that lands on `tint` over `paper`, solved by least
 * squares over the three channels and kept to whole percent, as CSS writes it.
 */
export function selectionOverlay(paper: Rgba, accent: Rgba, tint: Rgba): Rgba {
  let num = 0;
  let den = 0;
  for (const k of channels) {
    num += (tint[k] - paper[k]) * (accent[k] - paper[k]);
    den += (accent[k] - paper[k]) ** 2;
  }
  const a = den === 0 ? 0 : Math.round((num / den) * 100) / 100;
  return { r: accent.r, g: accent.g, b: accent.b, a: Math.max(0, Math.min(1, a)) };
}

/** The largest difference of any channel between two colours. */
export function distance(a: Rgba, b: Rgba): number {
  return Math.max(...channels.map((k) => Math.abs(a[k] - b[k])));
}

/** A colour as CSS: `#RRGGBB` when opaque, else `rgb(r g b / n%)`. */
export function css(c: Rgba): string {
  const hex = `#${channels.map((k) => c[k].toString(16).padStart(2, "0").toUpperCase()).join("")}`;
  return c.a >= 1 ? hex : `rgb(${c.r} ${c.g} ${c.b} / ${Math.round(c.a * 100)}%)`;
}
