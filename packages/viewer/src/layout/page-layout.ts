// The viewer's page layout (C3b): one continuous column of pages, each sized from its displayed
// size in points (ADR 0027: `/Rotate` applied) times the zoom, turned by the view rotation, with a
// gap between pages. Offsets are prefix sums, so finding the page at a scroll position is a binary
// search and a zoom or rotation change costs one O(n) pass. No DOM: the view reads it.

/** A page's displayed size in points. */
export interface PageSize {
  readonly width: number;
  readonly height: number;
}

/** View rotation, clockwise, on top of the page's own `/Rotate`. */
export type Rotation = 0 | 90 | 180 | 270;

/** CSS pixels per point at 100 % ("actual size"): 96 CSS px per inch, 72 pt per inch. */
export const CSS_PX_PER_PT = 96 / 72;

/** `size` as displayed after a view rotation: width and height swap at 90° and 270°. */
export function rotated(size: PageSize, rotation: Rotation): PageSize {
  return rotation === 90 || rotation === 270 ? { width: size.height, height: size.width } : size;
}

/** Whether `value` is a view rotation (0, 90, 180 or 270). */
export function isRotation(value: number): value is Rotation {
  return value === 0 || value === 90 || value === 180 || value === 270;
}

/** How to lay pages out: the zoom as CSS px per point, the view rotation and the gap. */
export interface LayoutOptions {
  /** CSS pixels per point (100 % is {@link CSS_PX_PER_PT}). */
  readonly scale: number;
  readonly rotation?: Rotation;
  /** CSS pixels between pages, and above the first and below the last. */
  readonly gap?: number;
}

/** Page geometry in CSS pixels for one zoom and rotation. */
export class PageLayout {
  readonly scale: number;
  readonly rotation: Rotation;
  readonly gap: number;
  /** Each page's box width and height in CSS px. */
  readonly widths: Float64Array;
  readonly heights: Float64Array;
  /** `tops[i]` is page i's top edge; `tops[n]` is the bottom of the last gap (the content height). */
  readonly tops: Float64Array;
  readonly maxWidth: number;
  /** Elementary steps the last build and searches took (tests assert the cost by count, not time). */
  operations = 0;

  constructor(sizes: readonly PageSize[], options: LayoutOptions) {
    const { scale, rotation = 0, gap = 16 } = options;
    if (!(scale > 0) || !Number.isFinite(scale)) throw new RangeError(`scale must be positive, got ${scale}`);
    this.scale = scale;
    this.rotation = rotation;
    this.gap = gap;
    const n = sizes.length;
    this.widths = new Float64Array(n);
    this.heights = new Float64Array(n);
    this.tops = new Float64Array(n + 1);
    let y = gap;
    let max = 0;
    for (let i = 0; i < n; i++) {
      const shown = rotated(sizes[i] as PageSize, rotation);
      const w = shown.width * scale;
      const h = shown.height * scale;
      this.widths[i] = w;
      this.heights[i] = h;
      this.tops[i] = y;
      y += h + gap;
      if (w > max) max = w;
      this.operations++;
    }
    this.tops[n] = y;
    this.maxWidth = max;
  }

  get pageCount(): number {
    return this.widths.length;
  }

  /** The content height in CSS px (every page and gap). */
  get height(): number {
    return this.tops[this.pageCount] ?? 0;
  }

  /** Page `index`'s top edge in CSS px. */
  offsetOf(index: number): number {
    if (!Number.isInteger(index) || index < 0 || index >= this.pageCount) {
      throw new RangeError(`page ${index} is outside 0..${this.pageCount - 1}`);
    }
    return this.tops[index] as number;
  }

  /**
   * The page at content offset `y`: the last page whose top is at or above `y` (a gap belongs to
   * the page above it), clamped to the first and last page. -1 for an empty document.
   */
  pageAt(y: number): number {
    const n = this.pageCount;
    if (n === 0) return -1;
    let lo = 0;
    let hi = n - 1;
    while (lo < hi) {
      this.operations++;
      const mid = (lo + hi + 1) >> 1;
      if ((this.tops[mid] as number) <= y) lo = mid;
      else hi = mid - 1;
    }
    return lo;
  }

  /** The pages any part of which lies in `[top, bottom)`, as an inclusive range. */
  rangeIn(top: number, bottom: number): { readonly first: number; readonly last: number } {
    const first = this.pageAt(top);
    let last = this.pageAt(Math.max(top, bottom - 1e-6));
    // `pageAt` puts a gap with the page above; a range that ends in a gap ends at that page.
    if (last < first) last = first;
    return { first, last };
  }

  /** Page `index`'s left edge in CSS px when pages are centred in a viewport `viewportWidth` wide. */
  leftOf(index: number, viewportWidth: number): number {
    return (this.contentWidth(viewportWidth) - (this.widths[index] as number)) / 2;
  }

  /** The content width in CSS px for a viewport `viewportWidth` wide. */
  contentWidth(viewportWidth: number): number {
    return Math.max(viewportWidth, this.maxWidth + 2 * this.gap);
  }
}
