// Where a link goes, as a picture (R07, design language §7.27): the region of the destination page
// a preview shows, and the scale it is drawn at. Pure numbers in ADR 0027 page space (points from
// the displayed page's top-left, y down), so its rules are tested in Node; the view rotation is
// applied when the region is rendered.
//
// A destination with a point (`XYZ` with a top, `FitH`, `FitR`) gets a region preview: a crop that
// starts just above and left of the point, as wide as the text column that starts there, so a
// citation shows its reference entry and a reference in the right column shows that column.
// Anything else gets R07's page preview: the top of the page at thumbnail scale.

import type { Target } from "@laminapdf/sdk";
import type { PageSize } from "../layout/page-layout.js";
import { CSS_PX_PER_PT } from "../layout/page-layout.js";
import type { TextRunData } from "../protocol.js";

/** How far above and left of the destination point the crop starts, in points. */
export const PREVIEW_PAD = 8;
/** A text line belongs to the destination's column when it starts this close to its left, in points. */
export const PREVIEW_ALIGN = 12;
/** The crop's width without text lines to measure (an image-only page, no text module), in points. */
export const PREVIEW_NO_TEXT_WIDTH = 360;
/** The largest crop a region preview draws, in CSS px. */
export const PREVIEW_MAX = { width: 480, height: 180 } as const;
/** A region preview's zoom: the reader's, clamped to these percentages; lowered to fit, never below the third. */
export const PREVIEW_ZOOM = { min: 100, max: 150, floor: 75 } as const;
/** R07's page preview: a crop this wide and this high (CSS px) of the page's top. */
export const PAGE_PREVIEW = { width: 224, height: 92 } as const;

/** A rectangle in page space, in points. */
export interface PageRect {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** One line of a page's text, in page space: its box and its text. */
export interface PreviewLine {
  readonly left: number;
  readonly top: number;
  readonly right: number;
  readonly bottom: number;
  readonly text: string;
}

/** What a preview shows: `clip` of page `page`, drawn at `scale` CSS px per point. */
export interface PreviewRegion {
  /** `region`: around the destination point; `page`: the page's top (R07). */
  readonly kind: "region" | "page";
  readonly page: number;
  readonly clip: PageRect;
  readonly scale: number;
}

const clamp = (v: number, low: number, high: number) => Math.max(low, Math.min(high, v));
/** A finite number, or `fallback` (a malformed file's NaN or infinity, ADR 0003). */
const finite = (v: number | undefined, fallback: number) => (v !== undefined && Number.isFinite(v) ? v : fallback);

/**
 * A page's text lines, from its runs and glyph quads (`pageText`). Runs come in reading order:
 * a run joins the line before it when it sits on the same band and starts no further right than
 * twice its font size past the line's end, so the two columns of a page stay two lines. Vertical
 * and empty runs are left out.
 */
export function previewLines(runs: readonly TextRunData[], quads: Float32Array): PreviewLine[] {
  const lines: { left: number; top: number; right: number; bottom: number; parts: string[] }[] = [];
  for (const run of runs) {
    if (run.vertical || run.text.trim() === "") continue;
    const [start, end] = run.glyphRange;
    let left = Number.POSITIVE_INFINITY;
    let top = Number.POSITIVE_INFINITY;
    let right = Number.NEGATIVE_INFINITY;
    let bottom = Number.NEGATIVE_INFINITY;
    for (let g = start; g < end; g++) {
      for (let k = 0; k < 4; k++) {
        const x = quads[g * 8 + k * 2];
        const y = quads[g * 8 + k * 2 + 1];
        if (x === undefined || y === undefined || !Number.isFinite(x) || !Number.isFinite(y)) continue;
        left = Math.min(left, x);
        right = Math.max(right, x);
        top = Math.min(top, y);
        bottom = Math.max(bottom, y);
      }
    }
    if (!(left <= right && top <= bottom)) continue;
    const last = lines.at(-1);
    const middle = (top + bottom) / 2;
    const reach = Math.max(2 * run.size, PREVIEW_ALIGN);
    if (last !== undefined && middle >= last.top && middle <= last.bottom && left - last.right <= reach && left >= last.left - reach) {
      last.left = Math.min(last.left, left);
      last.right = Math.max(last.right, right);
      last.top = Math.min(last.top, top);
      last.bottom = Math.max(last.bottom, bottom);
      last.parts.push(run.text);
    } else {
      lines.push({ left, top, right, bottom, parts: [run.text] });
    }
  }
  return lines.map(({ parts, ...box }) => ({ ...box, text: parts.join(" ").replace(/\s+/g, " ").trim() }));
}

/** CSS px per point at the reader's `percent`, clamped as a region preview draws. */
function zoomScale(percent: number): number {
  return (clamp(finite(percent, 100), PREVIEW_ZOOM.min, PREVIEW_ZOOM.max) / 100) * CSS_PX_PER_PT;
}

/** The page's top, at thumbnail scale: R07's page preview. */
function pagePreview(page: number, size: PageSize): PreviewRegion {
  const width = Math.max(1, finite(size.width, 1));
  const height = Math.max(1, finite(size.height, 1));
  const scale = PAGE_PREVIEW.width / width;
  return { kind: "page", page, clip: { x: 0, y: 0, width, height: Math.min(height, PAGE_PREVIEW.height / scale) }, scale };
}

/**
 * What a preview of `target` shows. `size` is the destination page's displayed size, `lines` its
 * text lines (`undefined` without text), `percent` the reader's zoom. Bad values (a top above the
 * page, a rectangle outside it, NaN) are clamped onto the page and never throw.
 */
export function previewRegion(
  target: Target,
  size: PageSize,
  lines: readonly PreviewLine[] | undefined,
  percent: number,
): PreviewRegion {
  const page = target.page;
  const width = Math.max(1, finite(size.width, 1));
  const height = Math.max(1, finite(size.height, 1));
  const view = target.view;
  switch (view.kind) {
    case "fit-rect":
      return rectPreview(page, width, height, view);
    case "at":
    case "fit-width":
      if (view.y === undefined || !Number.isFinite(view.y)) return pagePreview(page, size);
      return pointPreview(page, width, height, view.kind === "at" ? view.x : undefined, view.y, lines, percent);
    default:
      return pagePreview(page, size);
  }
}

/** A `FitR` destination: the rectangle, scaled to fit the card. */
function rectPreview(
  page: number,
  width: number,
  height: number,
  rect: { readonly x0: number; readonly y0: number; readonly x1: number; readonly y1: number },
): PreviewRegion {
  const xs = [clamp(finite(rect.x0, 0), 0, width), clamp(finite(rect.x1, width), 0, width)];
  const ys = [clamp(finite(rect.y0, 0), 0, height), clamp(finite(rect.y1, height), 0, height)];
  const x = Math.min(...xs);
  const y = Math.min(...ys);
  const w = Math.max(1, Math.max(...xs) - x);
  const h = Math.max(1, Math.max(...ys) - y);
  const scale = Math.min(PREVIEW_MAX.width / w, PREVIEW_MAX.height / h, zoomScale(PREVIEW_ZOOM.max));
  return {
    kind: "region",
    page,
    clip: { x, y, width: Math.min(w, PREVIEW_MAX.width / scale), height: Math.min(h, PREVIEW_MAX.height / scale) },
    scale,
  };
}

/** A destination point: the crop from just above and left of it, as wide as its text column. */
function pointPreview(
  page: number,
  width: number,
  height: number,
  left: number | undefined,
  top: number,
  lines: readonly PreviewLine[] | undefined,
  percent: number,
): PreviewRegion {
  let scale = zoomScale(percent);
  const y = clamp(top - PREVIEW_PAD, 0, height - 1);
  const band = (line: PreviewLine) => line.top >= y && line.top <= y + PREVIEW_MAX.height / scale;
  const inBand = (lines ?? []).filter(band);
  let x: number;
  let right: number | undefined;
  if (left === undefined || !Number.isFinite(left)) {
    // No left (`FitH`, an `XYZ` without one): the band's text, from its leftmost line.
    x = inBand.length === 0 ? 0 : clamp(Math.min(...inBand.map((l) => l.left)) - PREVIEW_PAD, 0, width - 1);
    right = inBand.length === 0 ? undefined : Math.max(...inBand.map((l) => l.right));
  } else {
    const at = clamp(left, 0, width);
    x = clamp(at - PREVIEW_PAD, 0, width - 1);
    const column = inBand.filter((line) => Math.abs(line.left - at) <= PREVIEW_ALIGN);
    right = column.length === 0 ? undefined : Math.max(...column.map((l) => l.right));
  }
  let w = clamp((right ?? x + PREVIEW_NO_TEXT_WIDTH) + PREVIEW_PAD - x, 1, width - x);
  if (w * scale > PREVIEW_MAX.width) {
    scale = Math.max((PREVIEW_ZOOM.floor / 100) * CSS_PX_PER_PT, PREVIEW_MAX.width / w);
    w = Math.min(w, PREVIEW_MAX.width / scale);
  }
  const h = Math.max(1, Math.min(PREVIEW_MAX.height / scale, height - y));
  return { kind: "region", page, clip: { x, y, width: w, height: h }, scale };
}

/** The text of the lines inside `clip`, in reading order, joined and cut at `max` characters. */
export function previewText(lines: readonly PreviewLine[], clip: PageRect, max = 300): string {
  const inside = lines.filter(
    (l) =>
      l.right > clip.x &&
      l.left < clip.x + clip.width &&
      (l.top + l.bottom) / 2 >= clip.y &&
      (l.top + l.bottom) / 2 <= clip.y + clip.height,
  );
  const text = inside
    .map((l) => l.text)
    .join(" ")
    .trim();
  return text.length <= max ? text : `${text.slice(0, max - 1).trimEnd()}…`;
}

/** What to ask the renderer for: the page at `scale` (device px per point), cut to `clip`. */
export interface PreviewRender {
  readonly scale: number;
  readonly clip: { readonly x: number; readonly y: number; readonly width: number; readonly height: number };
  /** The crop's size on screen, in CSS px (turned with the view). */
  readonly css: { readonly width: number; readonly height: number };
}

/**
 * The render request for `region` of a page of `size`, turned `rotation` degrees clockwise as the
 * view is, at `dpr` device pixels per CSS px. The clip is in the rotated page's device pixels, as
 * the renderer takes it, rounded out to whole pixels and kept on the page.
 */
export function previewRender(region: PreviewRegion, size: PageSize, rotation: 0 | 90 | 180 | 270, dpr: number): PreviewRender {
  const scale = region.scale * Math.max(0.5, finite(dpr, 1));
  const { x, y, width, height } = region.clip;
  // The clip's corners as fractions of the page, turned as the view turns the page.
  const corners: [number, number][] = [
    [x / size.width, y / size.height],
    [(x + width) / size.width, (y + height) / size.height],
  ];
  const turned = corners.map(([fx, fy]) => {
    switch (rotation) {
      case 90:
        return [1 - fy, fx];
      case 180:
        return [1 - fx, 1 - fy];
      case 270:
        return [fy, 1 - fx];
      default:
        return [fx, fy];
    }
  });
  const quarter = rotation === 90 || rotation === 270;
  const pageW = (quarter ? size.height : size.width) * scale;
  const pageH = (quarter ? size.width : size.height) * scale;
  const xs = turned.map(([fx]) => (fx ?? 0) * pageW);
  const ys = turned.map(([, fy]) => (fy ?? 0) * pageH);
  const x0 = clamp(Math.floor(Math.min(...xs)), 0, Math.max(0, Math.floor(pageW) - 1));
  const y0 = clamp(Math.floor(Math.min(...ys)), 0, Math.max(0, Math.floor(pageH) - 1));
  const x1 = clamp(Math.ceil(Math.max(...xs)), x0 + 1, Math.max(x0 + 1, Math.floor(pageW)));
  const y1 = clamp(Math.ceil(Math.max(...ys)), y0 + 1, Math.max(y0 + 1, Math.floor(pageH)));
  const cssW = quarter ? height * region.scale : width * region.scale;
  const cssH = quarter ? width * region.scale : height * region.scale;
  return { scale, clip: { x: x0, y: y0, width: x1 - x0, height: y1 - y0 }, css: { width: cssW, height: cssH } };
}
