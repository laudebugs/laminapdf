// The selection mini toolbar's shell (§7.9): a toolbar above the first selected line, below it
// when there is no room, inside the viewport and the canvas, following the selection as the page
// scrolls or zooms. Shift+F10 moves focus into it, arrows move along it, and Escape dismisses it
// and puts focus back where it was. When it shows and what it holds are the app's.

import { SPACE } from "@laminapdf/tokens";
import { type ReactNode, type RefObject, useEffect, useLayoutEffect, useRef, useState } from "react";
import { Separator, Toolbar } from "react-aria-components";
import { createPortal } from "react-dom";
import { LayerScope } from "./layers.js";
import { type Box, type Placement, placeAbove } from "./place.js";

/** Props of `SelectionToolbar`. */
export interface SelectionToolbarProps {
  /** The toolbar's name: "Selection". */
  "aria-label": string;
  /** The first selected line's rectangle in viewport coordinates, read again whenever the page moves. */
  anchor: () => Box | null;
  /** The area the toolbar must stay in, such as the canvas; the viewport when left out. */
  bounds?: () => Box | null;
  /** Called when Escape dismisses the toolbar; the app then hides it. */
  onDismiss: () => void;
  /** The toolbar's buttons. */
  children: ReactNode;
}

/** The gap between the selection and the toolbar, and between the toolbar and the bounds' edge. */
const GAP = SPACE["8"];

/** A floating toolbar for the current selection. Render it while the selection lasts. */
export function SelectionToolbar({
  "aria-label": label,
  anchor,
  bounds,
  onDismiss,
  children,
}: SelectionToolbarProps): ReactNode {
  const ref = useRef<HTMLDivElement>(null);
  const place = useAnchoredPlacement(ref, anchor, bounds);
  const returnTo = useRef<HTMLElement | null>(null);
  useShiftF10(ref, returnTo);
  const dismiss = () => {
    // Focus goes back only when Escape came from inside the toolbar; from elsewhere it stays put.
    const inside = ref.current?.contains(document.activeElement) === true;
    onDismiss();
    if (inside && returnTo.current?.isConnected === true) returnTo.current.focus();
  };
  return createPortal(
    <LayerScope dismiss={dismiss}>
      <Toolbar
        ref={ref}
        aria-label={label}
        className="paper-selection-toolbar"
        data-side={place?.side}
        style={place === null ? { visibility: "hidden" } : { top: place.top, left: place.left }}
      >
        {children}
      </Toolbar>
    </LayerScope>,
    document.body,
  );
}

/** A divider between groups of the toolbar's buttons. */
export function SelectionToolbarDivider(): ReactNode {
  return <Separator orientation="vertical" className="paper-selection-toolbar-divider" />;
}

/**
 * Keeps `layer` placed against `anchor`: before it first paints, after every render (a new
 * selection comes as a render), and on every scroll (of any scroller, so zooming the canvas
 * counts), resize and change of the layer's own size.
 */
function useAnchoredPlacement(
  layer: RefObject<HTMLElement | null>,
  anchor: () => Box | null,
  bounds: (() => Box | null) | undefined,
): Placement | null {
  const [place, setPlace] = useState<Placement | null>(null);
  const measure = () => {
    const element = layer.current;
    const target = anchor();
    if (element === null || target === null) return setPlace(null);
    const viewport = { top: 0, left: 0, width: window.innerWidth, height: window.innerHeight };
    const area = intersect(viewport, bounds?.() ?? viewport);
    const next = placeAbove(target, element.offsetWidth, element.offsetHeight, area, GAP, GAP);
    // The same placement keeps the same object, so measuring after a render doesn't render again.
    setPlace((last) => (last !== null && samePlace(last, next) ? last : next));
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
  return place;
}

function samePlace(a: Placement, b: Placement): boolean {
  return a.top === b.top && a.left === b.left && a.side === b.side;
}

/** The part of `a` inside `b`. */
function intersect(a: Box, b: Box): Box {
  const top = Math.max(a.top, b.top);
  const left = Math.max(a.left, b.left);
  return {
    top,
    left,
    width: Math.max(0, Math.min(a.left + a.width, b.left + b.width) - left),
    height: Math.max(0, Math.min(a.top + a.height, b.top + b.height) - top),
  };
}

/**
 * Shift+F10 from anywhere moves focus to the toolbar's first control, remembering where it was;
 * focus moving anywhere outside the toolbar forgets it, so a later Escape doesn't pull focus back.
 */
function useShiftF10(toolbar: RefObject<HTMLElement | null>, returnTo: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const element = toolbar.current;
      if (event.key !== "F10" || !event.shiftKey || element === null) return;
      if (element.contains(document.activeElement)) return;
      const first = element.querySelector<HTMLElement>("button:not(:disabled), [tabindex]:not([tabindex='-1'])");
      if (first === null) return;
      event.preventDefault();
      returnTo.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      first.focus();
    };
    const onFocusIn = (event: FocusEvent) => {
      if (!(event.target instanceof Node && toolbar.current?.contains(event.target) === true)) returnTo.current = null;
    };
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("focusin", onFocusIn);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("focusin", onFocusIn);
    };
  }, [toolbar, returnTo]);
}
