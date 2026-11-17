// A page layer's box: a div sized in page points, scaled and turned by one CSS transform to the
// page's current size, so what a layer places in points stays over the rendered page at every
// zoom and rotation. The text, highlight, link and annotation layers share it.

import type { Rotation } from "../layout/page-layout.js";

/** The page's box: its size in points, scaled and turned to `size` (CSS px) at `rotation`. */
function boxTransform(
  points: { readonly width: number; readonly height: number },
  size: { readonly width: number; readonly height: number },
  rotation: Rotation,
): string {
  const turned = rotation === 90 || rotation === 270;
  const scale = size.width / (turned ? points.height : points.width);
  const [x, y] = { 0: [0, 0], 90: [size.width, 0], 180: [size.width, size.height], 270: [0, size.height] }[rotation];
  return `translate(${x}px, ${y}px) rotate(${rotation}deg) scale(${scale})`;
}

/** Sizes `box` in points and places it over a page `size` CSS px at `rotation`. */
export function placeBox(
  box: HTMLElement,
  points: { readonly width: number; readonly height: number },
  size: { readonly width: number; readonly height: number },
  rotation: Rotation,
): void {
  Object.assign(box.style, {
    width: `${points.width}px`,
    height: `${points.height}px`,
    transform: boxTransform(points, size, rotation),
  });
}

/**
 * A layer's `onZoom` for its page views in `pages`: page `page`'s view, if mounted, takes the
 * page's new size and is placed again by `place`.
 */
export function zoomView<V extends { size: { readonly width: number; readonly height: number } | undefined }>(
  pages: ReadonlyMap<number, V>,
  page: number,
  size: { readonly width: number; readonly height: number },
  place: (page: number, view: V) => void,
): void {
  const view = pages.get(page);
  if (view === undefined) return;
  view.size = size;
  place(page, view);
}
