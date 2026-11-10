// A floating layer kept against its anchor: placed before it first paints, after every render, and on
// every scroll of any scroller, window or visual viewport resize and change of the layer's own size.
// The selection toolbar goes above its anchor and the link preview below; `place` says which.

import { type RefObject, useLayoutEffect, useRef, useState } from "react";
import { type Box, intersect, type Placement } from "./place.js";

/** Where a layer of `width` × `height` goes against `anchor` inside `bounds` (`placeAbove`, `placeBelow`). */
type Place = (
  anchor: Box,
  width: number,
  height: number,
  bounds: Box,
  gap: number,
  margin: number,
) => Placement | null;

/** How a layer is kept against its anchor. */
export interface Anchoring {
  /** The anchor's box in the viewport, or null while it is out of sight. */
  readonly anchor: () => Box | null;
  /** The area the layer must stay in; the viewport when left out. */
  readonly bounds?: (() => Box | null) | undefined;
  /** Which side of the anchor it goes: `placeAbove` or `placeBelow`. */
  readonly place: Place;
  /** The gap from the anchor, and the margin inside the bounds, in CSS px. */
  readonly gap: number;
}

/** Keeps `layer` placed against its anchor, inside the viewport and the bounds. */
export function useAnchoredPlacement(layer: RefObject<HTMLElement | null>, at: Anchoring): Placement | null {
  // The compiler can't follow the latest-value ref kept here for effects (useEffectEvent could replace it).
  "use no memo";
  const [placement, setPlacement] = useState<Placement | null>(null);
  const measure = () => {
    const element = layer.current;
    const target = at.anchor();
    if (element === null || target === null) return setPlacement(null);
    const viewport = { top: 0, left: 0, width: window.innerWidth, height: window.innerHeight };
    const area = intersect(viewport, at.bounds?.() ?? viewport);
    const next = at.place(target, element.offsetWidth, element.offsetHeight, area, at.gap, at.gap);
    // The same placement keeps the same object, so measuring after a render doesn't render again.
    setPlacement((last) => (last !== null && next !== null && samePlace(last, next) ? last : next));
  };
  const latest = useRef(measure);
  latest.current = measure;
  useLayoutEffect(() => {
    latest.current();
  });
  useLayoutEffect(() => {
    let frame = 0;
    const later = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => latest.current());
    };
    const size = new ResizeObserver(later);
    if (layer.current !== null) size.observe(layer.current);
    document.addEventListener("scroll", later, true);
    window.addEventListener("resize", later);
    window.visualViewport?.addEventListener("resize", later);
    return () => {
      cancelAnimationFrame(frame);
      size.disconnect();
      document.removeEventListener("scroll", later, true);
      window.removeEventListener("resize", later);
      window.visualViewport?.removeEventListener("resize", later);
    };
  }, [layer]);
  return placement;
}

function samePlace(a: Placement, b: Placement): boolean {
  return a.top === b.top && a.left === b.left && a.side === b.side;
}
