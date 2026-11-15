// Leaving overlays (motion plan §3, 6): an overlay the app stops rendering stays on screen for its
// exit, marked data-exiting and inert, so it fades out while clicks and keys pass through it.
// React Aria does this itself for its Popover, Modal and Tooltip; Presence does it for the
// overlays it doesn't own (the selection toolbar, the link preview, toasts, the rail drawer).

import { MOTION } from "@laminapdf/tokens";
import { createContext, type ReactNode, useContext, useEffect, useState } from "react";

const Exiting = createContext(false);

/** Whether the overlay rendering this is on its way out: it marks itself `data-exiting` and `inert`. */
export function useExiting(): boolean {
  return useContext(Exiting);
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
  if (isOpen && kept !== value) setKept(value);
  return isOpen ? value : kept;
}

/** The exit times of plan §4.1: a small overlay's or a panel's. */
export type ExitSize = "small" | "panel";

/** Props of `Presence`. */
export interface PresenceProps<T> {
  /** What the overlay shows, or null or undefined while it is closed. */
  value: T | null | undefined;
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
  const [kept, setKept] = useState<T | null | undefined>(value);
  const [gone, setGone] = useState(value == null);
  const open = value != null;
  if (open && value !== kept) setKept(value);
  if (open && gone) setGone(false);
  useEffect(() => {
    if (open) return;
    const timer = setTimeout(() => setGone(true), MOTION[`exit-${exit}`]);
    return () => clearTimeout(timer);
  }, [open, exit]);
  const shown = open ? value : kept;
  if (shown == null || (!open && gone)) return null;
  return <Exiting value={!open}>{children(shown)}</Exiting>;
}
