// The selection mini toolbar's shell (§7.9): a toolbar above the first selected line, below it
// when there is no room, inside the viewport and the canvas, following the selection as anything
// scrolls and after every render, and hidden while the selection is out of view. Shift+F10 moves focus into it, arrows move along it, and Escape dismisses it
// and puts focus back where it was. When it shows and what it holds are the app's.

import { SPACE } from "@laminapdf/tokens";
import { type ReactNode, type RefObject, useEffect, useLayoutEffect, useRef } from "react";
import { Separator, Toolbar } from "react-aria-components";
import { createPortal } from "react-dom";
import { useAnchoredPlacement } from "./anchored.js";
import { LayerScope } from "./layers.js";
import { type Box, placeAbove } from "./place.js";

/** Props of `SelectionToolbar`. */
export interface SelectionToolbarProps {
  /** The toolbar's name: "Selection". */
  "aria-label": string;
  /**
   * The first selected line's rectangle in viewport coordinates, or null while there is none. It is
   * read again after every render and whenever anything scrolls or the window resizes; when the
   * selection moves any other way, as a zoom that doesn't scroll moves it, render the toolbar again.
   */
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
  const place = useAnchoredPlacement(ref, { anchor, bounds, place: placeAbove, gap: GAP });
  const returnTo = useRef<HTMLElement | null>(null);
  useShiftF10(ref, returnTo);
  useKeptSelection(ref);
  useLayoutEffect(() => {
    // Hiding the toolbar would drop the focus inside it to the page: it goes back where it came from.
    if (place === null && ref.current?.contains(document.activeElement) === true) returnTo.current?.focus();
  }, [place]);
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
 * Shift+F10 moves focus to the toolbar's first control, unless something else took the key, a
 * modal layer has made the toolbar inert, or the toolbar is hidden. However focus enters the
 * toolbar (Shift+F10, Tab, a click), the element it came from is where Escape returns it; focus
 * moving anywhere outside the toolbar forgets it.
 */
function useShiftF10(toolbar: RefObject<HTMLElement | null>, returnTo: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const element = toolbar.current;
      if (event.key !== "F10" || !event.shiftKey || event.defaultPrevented || element === null) return;
      if (element.contains(document.activeElement) || !reachable(element)) return;
      const first = element.querySelector<HTMLElement>("button:not(:disabled), [tabindex]:not([tabindex='-1'])");
      if (first === null) return;
      event.preventDefault();
      first.focus();
    };
    const onFocusIn = (event: FocusEvent) => {
      const element = toolbar.current;
      const into = event.target instanceof Node && element?.contains(event.target) === true;
      if (!into) returnTo.current = null;
      else if (event.relatedTarget instanceof HTMLElement && element?.contains(event.relatedTarget) !== true)
        returnTo.current = event.relatedTarget;
    };
    document.addEventListener("keydown", onKeyDown);
    document.addEventListener("focusin", onFocusIn);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
      document.removeEventListener("focusin", onFocusIn);
    };
  }, [toolbar, returnTo]);
}

/**
 * WebKit drops the page's selection when a button takes focus, and the toolbar's actions act on
 * that selection: the last range selected is put back whenever focus moves into the toolbar.
 */
function useKeptSelection(toolbar: RefObject<HTMLElement | null>): void {
  useEffect(() => {
    let kept: Range | undefined;
    const onSelectionChange = () => {
      const selection = document.getSelection();
      if (selection !== null && selection.rangeCount > 0 && !selection.isCollapsed) kept = selection.getRangeAt(0);
    };
    const onFocusIn = (event: FocusEvent) => {
      const selection = document.getSelection();
      const into = event.target instanceof Node && toolbar.current?.contains(event.target) === true;
      if (!into || kept === undefined || selection === null || !selection.isCollapsed) return;
      selection.removeAllRanges();
      selection.addRange(kept);
    };
    onSelectionChange();
    document.addEventListener("selectionchange", onSelectionChange);
    document.addEventListener("focusin", onFocusIn);
    return () => {
      document.removeEventListener("selectionchange", onSelectionChange);
      document.removeEventListener("focusin", onFocusIn);
    };
  }, [toolbar]);
}

/** Whether the toolbar can take focus: shown (it has a placement) and not inert under a modal layer. */
function reachable(toolbar: HTMLElement): boolean {
  return toolbar.style.visibility !== "hidden" && toolbar.closest("[inert]") === null;
}
