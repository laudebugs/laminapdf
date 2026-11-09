// What part of its page a reader-mode crop draws, in the renderer's pixels.

/**
 * `bbox`'s pixels at `scale`, kept inside the page's (whose size the renderer rounds down): a crop
 * of a whole page, a page image's, would otherwise reach a pixel past the page and be refused.
 */
export function cropClip(
  bbox: readonly [number, number, number, number],
  scale: number,
  page: { readonly width: number; readonly height: number } | undefined,
): { x: number; y: number; width: number; height: number } {
  const [x, y, width, height] = bbox;
  const left = Math.max(0, Math.floor(x * scale));
  const top = Math.max(0, Math.floor(y * scale));
  const right = Math.ceil((x + width) * scale);
  const bottom = Math.ceil((y + height) * scale);
  const maxX = page === undefined ? right : Math.floor(page.width * scale);
  const maxY = page === undefined ? bottom : Math.floor(page.height * scale);
  return {
    x: left,
    y: top,
    width: Math.max(1, Math.min(right, maxX) - left),
    height: Math.max(1, Math.min(bottom, maxY) - top),
  };
}
