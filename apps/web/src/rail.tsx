// The rail's tabs for an open document (R06): Pages, the viewer's thumbnails strip, and Outline,
// the document's outline as a tree, read from the document worker the first time the tab shows.
// The outline marks the section at the top of the view, worked out at most every 150 ms while the
// view scrolls.

import {
  currentOutlineItem,
  EmptyState,
  message,
  OutlineTree,
  type OutlineTreeItem,
  type RailTab,
  useStoreState,
} from "@laminapdf/ui";
import type { ThumbnailStrip, Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
import type { Engine } from "./session.ts";

/** How often the current section is worked out while the view moves, in ms. */
const SECTION_EVERY = 150;

/** The outline's items before it is read. */
const NO_ITEMS: readonly OutlineTreeItem[] = [];

/** Puts the strip's list in the tab's panel. */
function PagesPanel({ strip }: { strip: ThumbnailStrip }): ReactNode {
  const host = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (host.current !== null) strip.mount(host.current);
  }, [strip]);
  return <div ref={host} className="web-thumbnails" />;
}

/** The outline item that holds the top of `viewer`'s view, kept up to date as it scrolls. */
function useCurrentSection(viewer: Viewer, items: readonly OutlineTreeItem[]): string | undefined {
  const [current, setCurrent] = useState<string | undefined>(undefined);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const update = () => {
      timer = undefined;
      const top = viewer.pointAt(0, 0);
      setCurrent(top === undefined ? undefined : currentOutlineItem(items, { page: top.page, y: top.y }));
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
function OutlinePanel({ engine, viewer }: { engine: Engine; viewer: Viewer }): ReactNode {
  const outline = useStoreState(engine, (s) => s.outline);
  // Read once per document: a document opened while the tab shows clears it, so it reads again.
  useEffect(() => {
    if (outline === undefined) void engine.loadOutline();
  }, [engine, outline]);
  const items = typeof outline === "object" ? outline.items : NO_ITEMS;
  const current = useCurrentSection(viewer, items);
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
      onNavigate={(id) => void engine.goToOutlineItem(id)}
    />
  );
}

/** The rail's tabs while a document is open; none otherwise. */
export function railTabs(engine: Engine, open: boolean): RailTab[] {
  const { strip, viewer } = engine.getState();
  if (!open || strip === undefined || viewer === undefined) return [];
  return [
    { id: "pages", label: message("shell.rail.pages"), panel: <PagesPanel strip={strip} /> },
    { id: "outline", label: message("shell.rail.outline"), panel: <OutlinePanel engine={engine} viewer={viewer} /> },
  ];
}
