// The rail's tabs for an open document (R06): Pages, the viewer's thumbnails strip, and Outline,
// the document's outline as a tree, read from the document worker the first time the tab shows.
// The outline marks the section at the top of the view, worked out at most every 150 ms while the
// view scrolls. Where the rail covers the page (a drawer, or the phone's Contents sheet), going
// somewhere from it closes it, so the page shows.

import {
  currentOutlineItem,
  EmptyState,
  message,
  OutlineTree,
  type OutlineTreeItem,
  RAIL_DRAWER_QUERY,
  type RailTab,
  type ShellLayout,
  useMediaQuery,
  useStoreState,
} from "@laminapdf/ui";
import type { ThumbnailStrip, Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { downThePage } from "./outline.ts";
import type { Engine } from "./session.ts";

/** How often the current section is worked out while the view moves, in ms. */
const SECTION_EVERY = 150;

/** The outline's items before it is read. */
const NO_ITEMS: readonly OutlineTreeItem[] = [];

/** The point a page jump puts just under the viewport's top, past the gap left above the page, in CSS px. */
const TOP_PROBE = 24;

/** Closes the rail after a jump where it covers the page. */
function useCloseAfterJump(layout: ShellLayout): () => void {
  const covers = useMediaQuery(RAIL_DRAWER_QUERY);
  return () => {
    if (covers) layout.setRailOpen(false);
  };
}

/** Puts the strip's list in the tab's panel. */
function PagesPanel({ strip, layout }: { strip: ThumbnailStrip; layout: ShellLayout }): ReactNode {
  const host = useRef<HTMLDivElement>(null);
  const closeIfCovering = useCloseAfterJump(layout);
  useEffect(() => {
    if (host.current !== null) strip.mount(host.current);
  }, [strip]);
  // The strip goes to a page on a click, or on Enter or Space on a thumbnail.
  const onKeyUp = (e: { key: string }) => {
    if (e.key === "Enter" || e.key === " ") closeIfCovering();
  };
  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: listens for the strip's own options, which take the clicks and keys
    <div ref={host} className="web-thumbnails" onClick={closeIfCovering} onKeyUp={onKeyUp} />
  );
}

/** The outline item that holds the top of `viewer`'s view, kept up to date as it scrolls. */
function useCurrentSection(viewer: Viewer, items: readonly OutlineTreeItem[]): string | undefined {
  const [current, setCurrent] = useState<string | undefined>(undefined);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const update = () => {
      timer = undefined;
      // The middle of the viewport's width: on a turned view, across is down the page.
      const top = viewer.pointAt(viewer.element.clientWidth / 2, TOP_PROBE);
      if (top === undefined) {
        setCurrent(undefined);
        return;
      }
      // Above the first page the probe is in its gap: that is still the first page's top.
      const clamp = (v: number) => Math.min(1, Math.max(0, v));
      const y = clamp(downThePage(clamp(top.x), clamp(top.y), viewer.getState().rotation));
      setCurrent(currentOutlineItem(items, { page: top.page, y }));
    };
    update();
    const off = viewer.subscribe(() => {
      timer ??= setTimeout(update, SECTION_EVERY);
    });
    return () => {
      off();
      clearTimeout(timer);
    };
  }, [viewer, items]);
  return current;
}

/** The outline, read when the tab first shows. */
function OutlinePanel({ engine, viewer, layout }: { engine: Engine; viewer: Viewer; layout: ShellLayout }): ReactNode {
  const outline = useStoreState(engine, (s) => s.outline);
  const closeIfCovering = useCloseAfterJump(layout);
  // Read once per document: a document opened while the tab shows clears it, so it reads again.
  useEffect(() => {
    if (outline === undefined) void engine.loadOutline();
  }, [engine, outline]);
  const items = typeof outline === "object" ? outline.items : NO_ITEMS;
  const current = useCurrentSection(viewer, items);
  if (outline === "failed") {
    return (
      <EmptyState title={message("reader.outline.failed")} headingLevel={3} icon="list">
        {message("reader.outline.usePages")}
      </EmptyState>
    );
  }
  if (typeof outline !== "object") return null;
  if (items.length === 0) {
    return (
      <EmptyState title={message("reader.outline.none")} headingLevel={3} icon="list">
        {message("reader.outline.usePages")}
      </EmptyState>
    );
  }
  return (
    <OutlineTree
      items={items}
      {...(current === undefined ? {} : { current })}
      onNavigate={(id) => {
        void engine.goToOutlineItem(id);
        closeIfCovering();
      }}
    />
  );
}

/** The rail's tabs while a document is open; none otherwise. */
export function railTabs(
  engine: Engine,
  layout: ShellLayout,
  view: { readonly strip: ThumbnailStrip | undefined; readonly viewer: Viewer | undefined } | undefined,
): RailTab[] {
  const strip = view?.strip;
  const viewer = view?.viewer;
  if (strip === undefined || viewer === undefined) return [];
  return [
    { id: "pages", label: message("shell.rail.pages"), panel: <PagesPanel strip={strip} layout={layout} /> },
    {
      id: "outline",
      label: message("shell.rail.outline"),
      panel: <OutlinePanel engine={engine} viewer={viewer} layout={layout} />,
    },
  ];
}
