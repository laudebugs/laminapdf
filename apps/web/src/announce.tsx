// What the reader says to a screen reader without moving focus (WCAG 4.1.3): the document that
// opened or closed, the page in view once scrolling settles, and the zoom. Find says its own counts; failures
// are alerts where they show.

import { Announcer, LiveRegions, message } from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
import type { PageInfo } from "./outline.ts";

/** The page `index` as it is spoken: its label when the document has one. */
export function spokenPage(pages: readonly PageInfo[], index: number): string {
  const label = pages[index]?.label;
  const values = { page: index + 1, count: pages.length, label: label ?? "" };
  return message(label === undefined ? "viewer.page.name" : "viewer.page.labelled", values);
}

/**
 * Announces `viewer`'s page and zoom changes, and `file` with its page count each time a document
 * opens (`opened.pages`), or, when it came back where its reader stopped (`opened.resumedAt`,
 * 0-based), that page; and "Closed {name}" when the reader goes back to the home screen (`empty`)
 * straight from an open document (R19). Returns the live regions to render once.
 */
export function useAnnouncements(
  viewer: Viewer | undefined,
  file: { readonly name: string } | undefined,
  opened: { readonly pages: readonly PageInfo[]; readonly resumedAt?: number | undefined },
  empty: boolean,
): ReactNode {
  const { pages } = opened;
  // Only while a document is open: the engine clears it when one closes.
  const resumedAt = file === undefined ? undefined : opened.resumedAt;
  const [announcer] = useState(() => new Announcer());
  const count = pages.length;
  // Read when a page is announced, so a new page list never restarts the listeners.
  const latest = useRef(pages);
  useEffect(() => {
    latest.current = pages;
  });
  const fileName = file?.name;
  // The open document's name, read when the reader turns empty. This effect runs before the next
  // one clears it, so only a document that was open when the reader closed is announced.
  const openName = useRef<string | undefined>(undefined);
  useEffect(() => {
    if (!empty) return;
    const name = openName.current;
    openName.current = undefined;
    if (name !== undefined) announcer.polite(message("announce.closed", { name }), "open");
  }, [announcer, empty]);
  useEffect(() => {
    openName.current = fileName;
  }, [fileName]);
  useEffect(() => {
    if (file === undefined || fileName === undefined || count === 0) return;
    if (resumedAt !== undefined) {
      const page = latest.current[resumedAt]?.label ?? String(resumedAt + 1);
      announcer.polite(message("announce.resumed", { page }), "open");
      return;
    }
    announcer.polite(
      message(count === 1 ? "announce.opened.one" : "announce.opened", { name: fileName, count }),
      "open",
    );
  }, [announcer, file, fileName, count, resumedAt]);
  useEffect(() => {
    if (viewer === undefined) return;
    // A fit mode refitting after a resize or an open is not news: only a new mode or percentage is.
    let mode = viewer.getState().zoom;
    const stops = [
      viewer.on("pagechange", ({ page }) => announcer.polite(spokenPage(latest.current, page), "page")),
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
  }, [announcer, viewer]);
  return <LiveRegions announcer={announcer} />;
}
