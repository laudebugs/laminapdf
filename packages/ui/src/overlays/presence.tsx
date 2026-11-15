// Leaving overlays (motion plan §3, 6): an overlay the app stops rendering stays on screen for its
// exit, marked data-exiting and inert, so it fades out while clicks and keys pass through it.
// React Aria does this itself for its Popover, Modal and Tooltip; Presence does it for the
// overlays it doesn't own (the selection toolbar, the link preview, toasts, the rail drawer).

import { MOTION } from "@laminapdf/tokens";
import { createContext, type ReactNode, useContext, useEffect, useState } from "react";

const Exiting = createContext(false);

/** Two frames at 60 Hz: how much later than its exit token an overlay leaves the page. */
const EXIT_MARGIN_MS = 34;

/** Whether the overlay rendering this is on its way out: it marks itself `data-exiting` and `inert`. */
export function useExiting(): boolean {
  return useContext(Exiting);
}

/** The attributes of an overlay that is leaving, or of one that isn't: data-exiting and inert. */
export function exitingAttributes(exiting: boolean): { "data-exiting"?: true; inert: boolean } {
  return exiting ? { "data-exiting": true, inert: true } : { inert: false };
}

/** Marks what's inside as leaving, for an overlay that times its own exit (a toast). */
export function Leaving({ leaving, children }: { leaving: boolean; children: ReactNode }): ReactNode {
  return <Exiting value={leaving}>{children}</Exiting>;
}

/**
 * `value` while `isOpen`, and the last value it had while open once it closes: a dialog's title
 * and content stay as they were for its exit, though the state they came from has cleared.
 */
export function useLastOpen<T>(value: T, isOpen: boolean): T {
  const [kept, setKept] = useState(value);
  // Props arrive as a new object each render: kept only when a field changed, so an open dialog
  // doesn't render twice for every render.
  if (isOpen && !sameFields(kept, value)) setKept(value);
  return isOpen ? value : kept;
}

/** Whether `a` and `b` are the same value, or objects whose own fields are the same values. */
function sameFields(a: unknown, b: unknown): boolean {
  if (Object.is(a, b)) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  const keys = Object.keys(a);
  return (
    keys.length === Object.keys(b).length &&
    keys.every((k) => Object.is((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]))
  );
}

/** The exit times of plan §4.1: a small overlay's or a panel's. */
type ExitSize = "small" | "panel";

/** Props of `Presence`. */
export interface PresenceProps<T> {
  /** What the overlay shows, or null, undefined or false while it is closed. */
  value: T | null | undefined | false;
  /** How long it takes to leave: a small overlay's exit or a panel's. */
  exit?: ExitSize;
  /** The overlay for a value; while leaving, it gets the last value it showed. */
  children: (value: T) => ReactNode;
}

/**
 * Renders `children(value)` while `value` is set, and for the exit's time after it clears, with the
 * last value it had, so the overlay can leave without its content emptying. Its exit is timed by
 * the token rather than by the element's transition, so a tab that never draws (and never ends a
 * transition) can't leave a ghost behind.
 */
export function Presence<T>({ value, exit = "small", children }: PresenceProps<T>): ReactNode {
  const open = value != null && value !== false;
  const [kept, setKept] = useState<T | null>(open ? value : null);
  const [gone, setGone] = useState(!open);
  // Callers pass a fresh object each render: kept only when a field changed, not every render.
  if (open && !sameFields(kept, value)) setKept(value);
  if (open && gone) setGone(false);
  useEffect(() => {
    if (open) return;
    // A little past the exit's time: the timer starts at commit, a frame or two before the
    // transition does, and must not clip its last frame.
    const timer = setTimeout(() => setGone(true), MOTION[`exit-${exit}`] + EXIT_MARGIN_MS);
    return () => clearTimeout(timer);
  }, [open, exit]);
  const shown = open ? value : kept;
  if (shown == null || (!open && gone)) return null;
  return <Exiting value={!open}>{children(shown)}</Exiting>;
}
