// The text layer's geometry, pure: one span per text run, placed from its glyphs' quads (Lamina
// PDF's, never the renderer's: ADR 0004's two-parsers rule) in page space, points from the
// displayed page's top-left, y down. The span's own frame follows the run's baseline, so a rotated
// or vertical run gets a rotated span, and a right-to-left run reads right to left. Also where one
// piece of a selection's text ends a line or a word (copy.ts joins them).

import type { TextRunData } from "../protocol.js";

/** Where one run's span goes, in page space (points). */
export interface SpanSpec {
  /** The run's index in the page's runs. */
  readonly run: number;
  /** Its text (document text). */
  readonly text: string;
  /** The span's top-left corner, before its rotation. */
  readonly left: number;
  readonly top: number;
  /** Its extent along the baseline and across it. */
  readonly width: number;
  readonly height: number;
  /** The baseline's angle, clockwise from the x axis, in radians. */
  readonly angle: number;
  /** The run reads right to left. */
  readonly rtl: boolean;
  /** The marked-content ID, for the structure tree. */
  readonly mcid?: number;
}

interface Vec {
  readonly x: number;
  readonly y: number;
}

const dot = (a: Vec, b: Vec) => a.x * b.x + a.y * b.y;

/** A unit vector along `v`, or `fallback` for a zero vector. */
function unit(v: Vec, fallback: Vec): Vec {
  const length = Math.hypot(v.x, v.y);
  return length < 1e-6 ? fallback : { x: v.x / length, y: v.y / length };
}

/** Glyph `g`'s corners: bottom-left, bottom-right, top-right, top-left. */
function corners(quads: Float32Array, g: number): readonly [Vec, Vec, Vec, Vec] {
  const at = (k: number): Vec => ({ x: quads[g * 8 + k * 2] ?? 0, y: quads[g * 8 + k * 2 + 1] ?? 0 });
  return [at(0), at(1), at(2), at(3)];
}

function centre(c: readonly Vec[]): Vec {
  return { x: c.reduce((s, p) => s + p.x, 0) / c.length, y: c.reduce((s, p) => s + p.y, 0) / c.length };
}

/** Letters of right-to-left scripts (Hebrew, Arabic, Syriac, Thaana, N'Ko and their presentation forms). */
const RTL = /[\u0590-\u08ff\ufb1d-\ufdff\ufe70-\ufefc]/;

/**
 * Whether `text` reads right to left: its first letter is of a right-to-left script (UAX #9's
 * rule P2 for a paragraph's direction). A run's glyphs come in drawing order and its text in
 * logical order, so only the text can say.
 */
function readsRightToLeft(text: string): boolean {
  const first = /\p{L}/u.exec(text)?.[0];
  return first !== undefined && RTL.test(first);
}

/**
 * Each run's span, in run order (the page's reading order). Runs without glyphs or text have none.
 * The baseline's direction comes from the first glyph's quad (its bottom edge), or for vertical
 * writing from the first glyph's centre to the last's; the span's box covers every glyph's quad
 * in that frame.
 */
export function layoutSpans(runs: readonly TextRunData[], quads: Float32Array): SpanSpec[] {
  return runs.flatMap((run, index) => {
    const span = spanOf(run, index, quads);
    return span === undefined ? [] : [span];
  });
}

/** Run `run`'s span, or undefined for a run without glyphs or text. */
function spanOf(run: TextRunData, index: number, quads: Float32Array): SpanSpec | undefined {
  const [start, end] = run.glyphRange;
  if (end <= start || run.text === "") return undefined;
  const first = corners(quads, start);
  const last = corners(quads, end - 1);
  const along = run.vertical
    ? unit({ x: centre(last).x - centre(first).x, y: centre(last).y - centre(first).y }, { x: 0, y: 1 })
    : unit({ x: first[1].x - first[0].x, y: first[1].y - first[0].y }, { x: 1, y: 0 });
  // Across the baseline, upwards: the baseline turned a quarter anticlockwise (y points down).
  const up = { x: along.y, y: -along.x };
  const { minA, maxA, minB, maxB } = extent(quads, start, end, along, up);
  return {
    run: index,
    text: run.text,
    left: along.x * minA + up.x * maxB,
    top: along.y * minA + up.y * maxB,
    width: maxA - minA,
    height: maxB - minB,
    angle: Math.atan2(along.y, along.x),
    rtl: !run.vertical && readsRightToLeft(run.text),
    ...(run.mcid === undefined ? {} : { mcid: run.mcid }),
  };
}

/** How far glyphs `start..end` reach along the baseline (`A`) and across it (`B`). */
function extent(quads: Float32Array, start: number, end: number, along: Vec, up: Vec) {
  let [minA, maxA, minB, maxB] = [Infinity, -Infinity, Infinity, -Infinity];
  for (let g = start; g < end; g++) {
    for (const p of corners(quads, g)) {
      const a = dot(p, along);
      const b = dot(p, up);
      minA = Math.min(minA, a);
      maxA = Math.max(maxA, a);
      minB = Math.min(minB, b);
      maxB = Math.max(maxB, b);
    }
  }
  return { minA, maxA, minB, maxB };
}

/** A selected piece of a span's text, with where its span is, for joining. */
export interface Piece {
  readonly page: number;
  readonly text: string;
  readonly span: SpanSpec;
}

/** What goes between two pieces as copy writes them: a new line, a space or nothing. */
export function separator(a: Piece, b: Piece): string {
  if (a.page !== b.page) return "\n";
  const along = { x: Math.cos(a.span.angle), y: Math.sin(a.span.angle) };
  const up = { x: along.y, y: -along.x };
  // Bottoms, not tops: a drop cap or a superscript on the same line has another top, while its
  // bottom stays within half a line of the line's.
  const lineA = dot({ x: a.span.left, y: a.span.top }, up) - a.span.height;
  const lineB = dot({ x: b.span.left, y: b.span.top }, up) - b.span.height;
  if (Math.abs(lineA - lineB) > Math.max(a.span.height, b.span.height) / 2) return "\n";
  const startA = dot({ x: a.span.left, y: a.span.top }, along);
  const startB = dot({ x: b.span.left, y: b.span.top }, along);
  // The space between the two boxes along the baseline, whichever comes first: right to left, or a
  // comma between two words of another direction, the next run sits before the last.
  const gap = Math.max(startB - (startA + a.span.width), startA - (startB + b.span.width));
  const joined = /\s$/.test(a.text) || /^\s/.test(b.text);
  return !joined && gap > a.span.height * 0.15 ? " " : "";
}
