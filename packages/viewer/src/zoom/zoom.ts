// Zoom (C3b): modes to a scale, the zoom range, and anchoring — the document point under the
// cursor (or the viewport centre) stays where it is on screen when the scale changes.
import { CSS_PX_PER_PT, type PageLayout, type PageSize, type Rotation, rotated } from "../layout/page-layout.js";

/** The fit modes. */
export type ZoomMode = "fit-width" | "fit-page" | "actual-size";
/** A zoom: a fit mode, or a percentage (100 is actual size). */
export type Zoom = ZoomMode | number;

/** The smallest zoom, in percent. */
export const MIN_ZOOM = 10;
/** The largest zoom, in percent. */
export const MAX_ZOOM = 800;

/** `percent` as CSS px per point, clamped to the range. */
export function scaleOfPercent(percent: number): number {
  const p = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, percent));
  return (p / 100) * CSS_PX_PER_PT;
}

/** A scale back as a percentage. */
export function percentOfScale(scale: number): number {
  return (scale / CSS_PX_PER_PT) * 100;
}

/** A viewport's size in CSS px. */
export interface Viewport {
  readonly width: number;
  readonly height: number;
}

/**
 * The scale (CSS px per pt) for `zoom` with `page` the reference page (the current one), the view
 * `rotation`, a `viewport` and `gap` CSS px of margin around the page.
 */
export function scaleFor(zoom: Zoom, page: PageSize, rotation: Rotation, viewport: Viewport, gap: number): number {
  if (typeof zoom === "number") return scaleOfPercent(zoom);
  if (zoom === "actual-size") return CSS_PX_PER_PT;
  const { width: w, height: h } = rotated(page, rotation);
  const byWidth = Math.max(1, viewport.width - 2 * gap) / w;
  const scale = zoom === "fit-width" ? byWidth : Math.min(byWidth, Math.max(1, viewport.height - 2 * gap) / h);
  return Math.min(scaleOfPercent(MAX_ZOOM), Math.max(scaleOfPercent(MIN_ZOOM), scale));
}

/** A point in the document: a page and a fraction of its box (0..1 across, 0..1 down). */
export interface DocumentPoint {
  readonly page: number;
  readonly x: number;
  readonly y: number;
}

/** A position in viewport CSS px (from its top-left), such as the cursor. */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/** A scroll position in CSS px. */
export interface Scroll {
  readonly left: number;
  readonly top: number;
}

/** Where `point` lies in the scroll content (CSS px from its top-left). */
export function contentPosition(layout: PageLayout, viewportWidth: number, point: DocumentPoint): Point {
  return {
    x: layout.leftOf(point.page, viewportWidth) + point.x * (layout.widths[point.page] as number),
    y: layout.offsetOf(point.page) + point.y * (layout.heights[point.page] as number),
  };
}

/** The document point at viewport position `anchor` (CSS px from the viewport's top-left). */
export function pointAt(layout: PageLayout, viewportWidth: number, scroll: Scroll, anchor: Point): DocumentPoint {
  const contentY = scroll.top + anchor.y;
  const contentX = scroll.left + anchor.x;
  const page = layout.pageAt(contentY);
  const top = layout.offsetOf(page);
  const left = layout.leftOf(page, viewportWidth);
  return {
    page,
    x: (contentX - left) / (layout.widths[page] as number),
    y: (contentY - top) / (layout.heights[page] as number),
  };
}

/** The scroll position that puts `point` at viewport position `anchor` in `layout`. */
export function scrollToPoint(layout: PageLayout, viewportWidth: number, point: DocumentPoint, anchor: Point): Scroll {
  const at = contentPosition(layout, viewportWidth, point);
  return { top: Math.max(0, at.y - anchor.y), left: Math.max(0, at.x - anchor.x) };
}
