// Draws each page of two PDFs and counts the pixels that differ, to show where a changed PDF looks different.
//
// Both files are drawn by the same renderer, so its own quirks cancel out. In Node, load it with
// the module's bytes:
//
//   const renderer = await loadRenderer(readFileSync(require.resolve("@laminapdf/wasm/lamina_pdf_render_hayro.wasm")));
import { bytesSource, type RenderBackend, type RenderDocument, type RenderedTile } from "@laminapdf/render";
import { initHayro } from "@laminapdf/render-hayro";

/** Pixels per point: 2 draws at 144 dpi, enough to see a glyph that moved. */
const SCALE = 2;
/** How far a colour channel (0–255) may move before the pixel counts as changed: absorbs smoothing noise. */
const TOLERANCE = 8;

/** A rectangle in page points, from the page's top-left corner. */
export interface Box {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** One page: how many pixels differ, and the box around them (`null` when none do). */
export interface PageDifference {
  readonly page: number;
  readonly pixels: number;
  readonly box: Box | null;
}

export interface Comparison {
  /** The page counts of the first and second file; only the pages both have are compared. */
  readonly pageCounts: readonly [number, number];
  readonly pages: readonly PageDifference[];
}

/** Loads the renderer from the module's bytes, a URL, or (in a browser) the file next to its code. */
export function loadRenderer(wasm?: Uint8Array | URL): Promise<RenderBackend> {
  return initHayro(wasm);
}

/** Compares every page the two PDFs share. */
export function comparePages(renderer: RenderBackend, before: Uint8Array, after: Uint8Array): Comparison {
  using a = renderer.open(bytesSource(before));
  using b = renderer.open(bytesSource(after));
  const shared = Math.min(a.pageCount, b.pageCount);
  const pages = Array.from({ length: shared }, (_, page) => comparePage(a, b, page));
  return { pageCounts: [a.pageCount, b.pageCount], pages };
}

function comparePage(a: RenderDocument, b: RenderDocument, page: number): PageDifference {
  const first = a.render({ page, scale: SCALE });
  const second = b.render({ page, scale: SCALE });
  const width = Math.max(first.width, second.width);
  const height = Math.max(first.height, second.height);
  let pixels = 0;
  let [left, top, right, bottom] = [width, height, -1, -1];
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      if (!differs(first, second, x, y)) continue;
      pixels++;
      [left, top] = [Math.min(left, x), Math.min(top, y)];
      [right, bottom] = [Math.max(right, x), Math.max(bottom, y)];
    }
  }
  const box =
    pixels === 0
      ? null
      : { x: left / SCALE, y: top / SCALE, width: (right - left + 1) / SCALE, height: (bottom - top + 1) / SCALE };
  return { page, pixels, box };
}

/** Whether the pixel at (x, y) differs; a pixel only one page has always does. */
function differs(first: RenderedTile, second: RenderedTile, x: number, y: number): boolean {
  if (x >= first.width || y >= first.height || x >= second.width || y >= second.height) return true;
  const i = (y * first.width + x) * 4;
  const j = (y * second.width + x) * 4;
  for (let channel = 0; channel < 4; channel++) {
    const delta = Math.abs((first.pixels[i + channel] ?? 0) - (second.pixels[j + channel] ?? 0));
    if (delta > TOLERANCE) return true;
  }
  return false;
}
