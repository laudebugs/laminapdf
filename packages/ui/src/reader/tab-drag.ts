// Dragging a document tab, by pointer events. HTML5 drag and drop can't serve here: the desktop
// webview turns it off on Windows while it handles dropped files, and where it works it shows a
// "not allowed" cursor over windows that take the tab. A press on a tab that moves a few pixels
// starts a drag; the tab captures the pointer, so its moves and its release arrive even outside
// the window. The drag listens on the document, so it also follows events that a test dispatches
// to whatever is under the point, which no capture redirects. Over the strip the drag shows where the tab would go, and a release there reorders;
// a release anywhere else is the shell's (`onDragOut`). Escape, or the OS taking the pointer away,
// cancels it (`onDragCancel`). Mouse and pen only: a finger on the strip scrolls it.

import { type PointerEvent as ReactPointerEvent, useEffect, useRef, useState } from "react";

/** How far, in CSS pixels, a pressed tab must move before it drags rather than clicks. */
const DRAG_THRESHOLD_PX = 4;

/** What the shell does with a drag. */
export interface TabDragHandlers {
  /** The tabs' new order, after one was dropped elsewhere in the strip. */
  readonly onReorder?: ((ids: readonly string[]) => void) | undefined;
  /** A tab began to be dragged. */
  readonly onDragStart?: ((id: string) => void) | undefined;
  /** A tab was released off the strip; `outside`: the pointer was outside the window. */
  readonly onDragOut?: ((id: string, outside: boolean) => void) | undefined;
  /** A drag was cancelled: Escape, or the OS took the pointer. */
  readonly onDragCancel?: ((id: string) => void) | undefined;
}

/** A tab strip's drag, as its tabs render it. */
export interface TabDrag {
  /** The tab being dragged. */
  readonly dragged: string | undefined;
  /** Where the dragged tab would go if released now, as an index into the tabs (0 to their
      count), or none while the pointer is off the strip. */
  readonly insertAt: number | undefined;
  /** Whether a drag can start: only when the shell takes a reorder or a drag out. */
  readonly enabled: boolean;
  /** A tab was pressed. */
  pointerDown(event: ReactPointerEvent<HTMLElement>, id: string): void;
  /** Whether a click on a tab should be ignored: the release that ended a drag clicks too. */
  takeClick(): boolean;
}

/** One press, from the pointer going down until it comes up or the drag ends. */
interface Press {
  readonly id: string;
  readonly element: HTMLElement;
  readonly pointerId: number;
  readonly startX: number;
  readonly startY: number;
  dragging: boolean;
  insertAt: number | undefined;
  /** Undoes everything the press set up. */
  stop(): void;
}

/** The strip's drag of its `tabs` (their ids, in order). */
export function useTabDrag(tabs: readonly string[], handlers: TabDragHandlers): TabDrag {
  const [dragged, setDragged] = useState<string>();
  const [insertAt, setInsertAt] = useState<number>();
  const press = useRef<Press | undefined>(undefined);
  const swallowClick = useRef(false);
  // The handlers and tabs of the latest render, read when the pointer moves.
  const latest = useRef({ tabs, handlers });
  latest.current = { tabs, handlers };
  // A drag still running when the strip goes (its window closing) leaves nothing behind.
  useEffect(() => () => press.current?.stop(), []);
  useGrabbingCursor(dragged !== undefined);

  const end = (releasedAt: { x: number; y: number } | undefined) => {
    const current = press.current;
    if (current === undefined) return;
    current.stop();
    press.current = undefined;
    setDragged(undefined);
    setInsertAt(undefined);
    if (!current.dragging) return;
    swallowClick.current = true;
    settle(current, latest.current, releasedAt);
  };

  const move = (event: PointerEvent) => {
    const current = press.current;
    if (current === undefined || event.pointerId !== current.pointerId) return;
    if (!current.dragging) {
      if (Math.hypot(event.clientX - current.startX, event.clientY - current.startY) < DRAG_THRESHOLD_PX) return;
      current.dragging = true;
      setDragged(current.id);
      latest.current.handlers.onDragStart?.(current.id);
    }
    current.insertAt = insertionIndex(current.element, event.clientX, event.clientY);
    setInsertAt(current.insertAt);
  };

  const enabled = handlers.onReorder !== undefined || handlers.onDragOut !== undefined;
  return {
    dragged,
    insertAt,
    enabled,
    pointerDown: (event, id) => {
      const supported = event.pointerType === "mouse" || event.pointerType === "pen";
      if (!enabled || !supported || event.button !== 0 || press.current !== undefined) return;
      const element = event.currentTarget;
      const pointerId = event.pointerId;
      press.current = {
        id,
        element,
        pointerId,
        startX: event.clientX,
        startY: event.clientY,
        dragging: false,
        insertAt: undefined,
        stop: follow(element, pointerId, {
          move,
          up: end,
          cancel: () => end(undefined),
          dragging: () => press.current?.dragging === true,
        }),
      };
    },
    takeClick: () => {
      const swallow = swallowClick.current;
      swallowClick.current = false;
      return swallow;
    },
  };
}

/** While `dragging`, the whole page shows the grabbing cursor (document-tabs.css). */
function useGrabbingCursor(dragging: boolean): void {
  useEffect(() => {
    if (!dragging) return;
    document.documentElement.dataset.tabDragging = "";
    return () => {
      delete document.documentElement.dataset.tabDragging;
    };
  }, [dragging]);
}

/** Hands a drag that ended to the shell: a cancel (no `at`), a reorder when it was released over
    the strip, else a drag out. */
function settle(
  drag: Press,
  { tabs, handlers: on }: { tabs: readonly string[]; handlers: TabDragHandlers },
  at: { x: number; y: number } | undefined,
): void {
  if (at === undefined) on.onDragCancel?.(drag.id);
  else if (drag.insertAt !== undefined) {
    const order = reordered(tabs, drag.id, drag.insertAt);
    if (order !== undefined) on.onReorder?.(order);
  } else on.onDragOut?.(drag.id, outsideWindow(at));
}

/** What a pressed tab hears until the press ends. */
interface Listeners {
  readonly move: (event: PointerEvent) => void;
  readonly up: (at: { x: number; y: number }) => void;
  readonly cancel: () => void;
  /** Whether the press has become a drag, which Escape cancels. */
  readonly dragging: () => boolean;
}

/** Listens to pointer `pointerId`, which `element` captures, and to Escape; returns what undoes it
    all. */
function follow(element: HTMLElement, pointerId: number, on: Listeners): () => void {
  const move = (event: PointerEvent) => {
    if (event.pointerId === pointerId) on.move(event);
  };
  const up = (event: PointerEvent) => {
    if (event.pointerId === pointerId) on.up({ x: event.clientX, y: event.clientY });
  };
  const cancel = (event: PointerEvent) => {
    if (event.pointerId === pointerId) on.cancel();
  };
  const key = (event: KeyboardEvent) => {
    if (event.key !== "Escape" || !on.dragging()) return;
    event.preventDefault();
    event.stopPropagation();
    on.cancel();
  };
  document.addEventListener("pointermove", move, true);
  document.addEventListener("pointerup", up, true);
  document.addEventListener("pointercancel", cancel, true);
  element.addEventListener("lostpointercapture", on.cancel);
  document.addEventListener("keydown", key, true);
  try {
    element.setPointerCapture(pointerId);
  } catch {
    // A pointer the browser doesn't track (a test's dispatched event) can't be captured: the
    // drag still follows the events that reach the tab.
  }
  return () => {
    document.removeEventListener("pointermove", move, true);
    document.removeEventListener("pointerup", up, true);
    document.removeEventListener("pointercancel", cancel, true);
    element.removeEventListener("lostpointercapture", on.cancel);
    document.removeEventListener("keydown", key, true);
    if (element.hasPointerCapture(pointerId)) element.releasePointerCapture(pointerId);
  };
}

/**
 * The order of `ids` with `moving` put at `insertAt` (an index into `ids` before the move), or
 * none when that leaves the order as it is.
 */
export function reordered(ids: readonly string[], moving: string, insertAt: number): string[] | undefined {
  const from = ids.indexOf(moving);
  if (from < 0 || insertAt === from || insertAt === from + 1) return undefined;
  const rest = ids.filter((id) => id !== moving);
  rest.splice(insertAt > from ? insertAt - 1 : insertAt, 0, moving);
  return rest;
}

/** Where a tab released at (`x`, `y`) would go in the strip that holds `element`: the number of
    tabs whose middle comes before the pointer in the strip's direction, or none when the pointer
    is off the strip. */
function insertionIndex(element: HTMLElement, x: number, y: number): number | undefined {
  const strip = element.closest<HTMLElement>('[role="tablist"]');
  if (strip === null) return undefined;
  const box = strip.getBoundingClientRect();
  if (x < box.left || x >= box.right || y < box.top || y >= box.bottom) return undefined;
  const rtl = getComputedStyle(strip).direction === "rtl";
  const tabs = [...strip.querySelectorAll<HTMLElement>('[role="tab"]')];
  return tabs.filter((tab) => {
    const rect = tab.getBoundingClientRect();
    const middle = rect.left + rect.width / 2;
    return rtl ? middle > x : middle < x;
  }).length;
}

/** Whether a point in the page's coordinates lies outside the window's page. */
function outsideWindow({ x, y }: { x: number; y: number }): boolean {
  const root = document.documentElement;
  return x < 0 || y < 0 || x >= root.clientWidth || y >= root.clientHeight;
}
