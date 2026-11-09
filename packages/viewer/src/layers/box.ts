// A page layer's box: a div sized in page points, scaled and turned by one CSS transform to the
// page's current size, so what a layer places in points stays over the rendered page at every
// zoom and rotation. The text and highlight layers share it, and the viewer's parts share
// `selectable`.

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

/** Sets `element`'s `user-select`, with WebKit's prefix (Safari before 26 reads only that). */
export function selectable(element: HTMLElement, value: "none" | "text"): void {
  element.style.setProperty("user-select", value);
  element.style.setProperty("-webkit-user-select", value);
}
