// The capability page at /capabilities (O05): Help's "What Lamina PDF can do" pushes the address,
// Back (the page's or the browser's) pops it, and a link from the docs site opens it directly.
// The page covers the reader without closing it: the document stays open behind, inert, and Back
// returns to it where it was. The page and its matrix load with the rest of later.ts (#380).

import { lazy, type ReactNode, Suspense, useEffect, useState } from "react";
import { createPortal } from "react-dom";

/** The page's address. */
const CAPABILITIES_PATH = "/capabilities";

/** Marks the history entry this reader pushed for the page, so its Back can go back to it. */
const KEY = "laminaCapabilities";

/** The page, with the matrix bundled, loaded from later.ts's chunk on first open. */
const Page = lazy(() => import("./later.ts").then((m) => ({ default: m.CapabilityRoutePage })));

/** Whether `win` is at the capability page. */
const atPage = (win: Window) => win.location.pathname === CAPABILITIES_PATH;

/** The capability page's route: whether it shows, and what opens and leaves it. */
export interface CapabilityRoute {
  /** Opens the page (Help › What Lamina PDF can do). */
  readonly show: () => void;
  /** The page over the reader while the address is /capabilities; nothing otherwise. */
  readonly render: (fileName: string | undefined) => ReactNode;
}

/** The capability page's route in `win`. */
export function useCapabilityRoute(win: Window): CapabilityRoute {
  const [open, setOpen] = useState(() => atPage(win));
  useEffect(() => {
    const follow = () => setOpen(atPage(win));
    win.addEventListener("popstate", follow);
    return () => win.removeEventListener("popstate", follow);
  }, [win]);
  // The reader behind the page can't take focus or clicks while the page covers it.
  useEffect(() => {
    const app = win.document.getElementById("app");
    if (app === null) return;
    app.inert = open;
    return () => {
      app.inert = false;
    };
  }, [win, open]);
  const { search, hash } = win.location;
  const show = () => {
    if (atPage(win)) return;
    win.history.pushState({ [KEY]: true }, "", `${CAPABILITIES_PATH}${search}${hash}`);
    setOpen(true);
  };
  const back = () => {
    const state: unknown = win.history.state;
    if (typeof state === "object" && state !== null && KEY in state) {
      win.history.back();
      return;
    }
    // Opened at this address (a link, a reload): there is no entry to go back to, so the reader
    // takes the address's place.
    win.history.replaceState(null, "", `/${win.location.search}${win.location.hash}`);
    setOpen(false);
  };
  return {
    show,
    render: (fileName) =>
      open
        ? createPortal(
            <Suspense fallback={null}>
              <Page fileName={fileName} onBack={back} />
            </Suspense>,
            win.document.body,
          )
        : null,
  };
}
