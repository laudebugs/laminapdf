// The capability page at /capabilities (O05): Help's "What Lamina PDF can do" pushes the address,
// Back (the page's or the browser's) pops it, and a link from the docs site opens it directly.
// The page covers the reader without closing it: the document stays open behind, inert, and Back
// returns to it where it was. The page and its matrix load with the rest of later.ts (#380).

import { lazy, type ReactNode, Suspense, useEffect, useState } from "react";

/** The page's address. */
const CAPABILITIES_PATH = "/capabilities";

/** Marks the history entry this reader pushed for the page, so its Back can go back to it. */
export const KEY = "laminaCapabilities";

/** The page, with the matrix bundled, loaded from later.ts's chunk on first open. */
const Page = lazy(() => import("./later.ts").then((m) => ({ default: m.CapabilityRoutePage })));

/** Whether `win` is at the capability page. */
const atPage = (win: Window) => win.location.pathname === CAPABILITIES_PATH;

/** The capability page's route: whether it shows, and what opens it. */
export interface CapabilityRoute {
  /** Opens the page (Help › What Lamina PDF can do). */
  readonly show: () => void;
  /** The page over the reader while the address is /capabilities; nothing otherwise. */
  readonly render: (fileName: string | undefined) => ReactNode;
}

/**
 * The capability page's route in `win`. Only what start needs is here: whether the address is the
 * page's, and Help's way in. The page's Back and the reader's inertness load with the page.
 */
export function useCapabilityRoute(win: Window): CapabilityRoute {
  const [open, setOpen] = useState(() => atPage(win));
  useEffect(() => {
    const follow = () => setOpen(atPage(win));
    win.addEventListener("popstate", follow);
    return () => win.removeEventListener("popstate", follow);
  }, [win]);
  const show = () => {
    if (atPage(win)) return;
    win.history.pushState({ [KEY]: true }, "", `${CAPABILITIES_PATH}${win.location.search}${win.location.hash}`);
    setOpen(true);
  };
  return {
    show,
    render: (fileName) =>
      open ? (
        <Suspense fallback={null}>
          <Page win={win} fileName={fileName} onLeft={() => setOpen(false)} />
        </Suspense>
      ) : null,
  };
}
