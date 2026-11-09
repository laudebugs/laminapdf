// What the reader says to a screen reader without moving focus (WCAG 4.1.3): the document that
// opened, the page in view once scrolling settles, and the zoom. Find says its own counts; failures
// are alerts where they show.

import { Announcer, LiveRegions, message } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useState } from "react";
import type { PageInfo } from "./outline.ts";

/** The page `index` as it is spoken: its label when the document has one. */
export function spokenPage(pages: readonly PageInfo[], index: number): string {
  const label = pages[index]?.label;
  const values = { page: index + 1, count: pages.length, label: label ?? "" };
  return message(label === undefined ? "viewer.page.name" : "viewer.page.labelled", values);
}

/**
 * Announces `viewer`'s page and zoom changes, and `fileName` with its page count when a document
 * opens; returns the live regions to render once.
 */
export function useAnnouncements(
  viewer: Viewer | undefined,
  fileName: string | undefined,
  pages: readonly PageInfo[],
): ReactNode {
  const [announcer] = useState(() => new Announcer());
  const count = pages.length;
  useEffect(() => {
    if (fileName === undefined || count === 0) return;
    announcer.polite(
      message(count === 1 ? "announce.opened.one" : "announce.opened", { name: fileName, count }),
      "open",
    );
  }, [announcer, fileName, count]);
  useEffect(() => {
    if (viewer === undefined) return;
    // A fit mode refitting after a resize or an open is not news: only a new mode or percentage is.
    let mode = viewer.getState().zoom;
    const stops = [
      viewer.on("pagechange", ({ page }) => announcer.polite(spokenPage(pages, page), "page")),
      viewer.on("zoomchange", ({ zoom, percent }) => {
        const changed = typeof zoom === "number" || zoom !== mode;
        mode = zoom;
        if (changed) announcer.polite(message("announce.zoom", { percent: Math.round(percent) }), "zoom");
      }),
    ];
    return () => {
      for (const stop of stops) stop();
      announcer.cancel();
    };
  }, [announcer, viewer, pages]);
  return <LiveRegions announcer={announcer} />;
}
