// The rail's tabs every reader has for an open document: Pages, the thumbnails strip, and Outline,
// the document's outline as a tree, read from the engine the first time the tab shows. The outline
// marks the section at the top of the view, worked out at most every 150 ms while the view scrolls.
// Where the rail covers the page (a drawer, or the phone's Contents sheet), going somewhere from it
// closes it, so the page shows. The web reader and the desktop both build their rails from these;
// each adds the tabs only it has (Threads, Attachments and the bookmark editor on the web).

import { type ReactNode, useEffect, useRef, useState } from "react";
import { EmptyState } from "../components/empty-state.js";
import { message } from "../messages.js";
import type { ShellLayout } from "../shell/layout.js";
import { RAIL_DRAWER_QUERY, type RailTab } from "../shell/left-rail.js";
import { type ExternalStore, useMediaQuery, useStoreState } from "../state/hooks.js";
import {
  downThePage,
  type OutlineModel,
  type OutlineRotation,
  type OutlineTarget,
  TOP_PROBE,
} from "./outline-model.js";
import { currentOutlineItem, OutlineTree, type OutlineTreeItem } from "./outline-tree.js";

/** How often the current section is worked out while the view moves, in ms. */
const SECTION_EVERY = 150;

/** The outline's items before it is read. */
const NO_ITEMS: readonly OutlineTreeItem[] = [];

/** What the rail needs of the viewer: its element, the point under a position, its turn and its changes. */
export interface RailViewer {
  readonly element: HTMLElement;
  /** The page, and the point on it as fractions of its box, under `x`, `y` in the viewport; none off the pages. */
  pointAt(x: number, y: number): { readonly page: number; readonly x: number; readonly y: number } | undefined;
  /** The view's turn. */
  getState(): { readonly rotation: OutlineRotation };
  /** Calls `listener` as the view changes; returns the function that stops it. */
  subscribe(listener: () => void): () => void;
}

/** What the rail needs of the thumbnails strip: somewhere to mount (a click on a thumbnail goes to its page). */
export interface RailStrip {
  /** Puts the strip's list in `container`. */
  mount(container: HTMLElement): void;
}

/** The outline as the rail's Outline tab reads it. */
export type OutlineState<T extends OutlineTarget = OutlineTarget> = OutlineModel<T> | "loading" | "failed" | undefined;

/** The engine, for the rail: a store whose state holds the outline, once read, and the calls the tab makes. */
export interface OutlineSource<T extends OutlineTarget = OutlineTarget>
  extends ExternalStore<{
    readonly outline: OutlineState<T>;
  }> {
  /** Reads the open document's outline, once. */
  loadOutline(): Promise<void>;
  /** Goes to outline item `id` of the open document. */
  goToOutlineItem(id: string): Promise<void>;
}

/** The outline's editing part (web only): it shows the read view, or the bookmark editor. */
export type OutlineEditorSlot = (props: {
  readonly current: string | undefined;
  readonly onNavigate: (id: string) => void;
  /** The read view: the tree, or the empty state. */
  readonly view: ReactNode;
}) => ReactNode;

/** Closes the rail after a jump where it covers the page. */
export function useCloseAfterJump(layout: ShellLayout): () => void {
  const covers = useMediaQuery(RAIL_DRAWER_QUERY);
  return () => {
    if (covers) layout.setRailOpen(false);
  };
}

/** Puts the strip's list in the tab's panel. */
function PagesPanel({ strip, layout }: { strip: RailStrip; layout: ShellLayout }): ReactNode {
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
    <div ref={host} className="paper-rail-thumbnails" onClick={closeIfCovering} onKeyUp={onKeyUp} />
  );
}

/** The outline item that holds the top of `viewer`'s view, kept up to date as it scrolls. */
function useCurrentSection(viewer: RailViewer, items: readonly OutlineTreeItem[]): string | undefined {
  const [current, setCurrent] = useState<string | undefined>(undefined);
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const update = () => {
      timer = undefined;
      // The middle of the viewport's width. On a view turned a quarter, down the page runs across
      // the screen, so scrolling moves only from page to page: the probe stays half way down each,
      // and an item in a page's lower half is marked once the next page reaches the top.
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
      if (timer === undefined) timer = setTimeout(update, SECTION_EVERY);
    });
    return () => {
      off();
      clearTimeout(timer);
    };
  }, [viewer, items]);
  return current;
}

/** The outline, read when the tab first shows; with the editor once it has loaded. */
function OutlinePanel({
  source,
  viewer,
  layout,
  editor,
}: {
  source: OutlineSource;
  viewer: RailViewer;
  layout: ShellLayout;
  editor: OutlineEditorSlot | undefined;
}): ReactNode {
  const outline = useStoreState(source, (s) => s.outline);
  const closeIfCovering = useCloseAfterJump(layout);
  // Read once per document: a document opened while the tab shows clears it, so it reads again.
  useEffect(() => {
    if (outline === undefined) void source.loadOutline();
  }, [source, outline]);
  const items = typeof outline === "object" ? outline.items : NO_ITEMS;
  const current = useCurrentSection(viewer, items);
  const go = (id: string) => {
    void source.goToOutlineItem(id);
    closeIfCovering();
  };
  if (outline === "failed") {
    return (
      <EmptyState title={message("reader.outline.failed")} headingLevel={3} icon="list">
        {message("reader.outline.usePages")}
      </EmptyState>
    );
  }
  if (typeof outline !== "object") return null;
  const view =
    items.length === 0 ? (
      <EmptyState title={message("reader.outline.none")} headingLevel={3} icon="list">
        {message("reader.outline.usePages")}
      </EmptyState>
    ) : (
      <OutlineTree items={items} {...(current === undefined ? {} : { current })} onNavigate={go} />
    );
  return editor === undefined ? view : editor({ current, onNavigate: go, view });
}

/** What {@link readerRailTabs} builds the tabs from. */
export interface ReaderRailTabsOptions {
  /** The engine that reads the outline and goes to its items. */
  readonly source: OutlineSource;
  readonly layout: ShellLayout;
  readonly strip: RailStrip;
  readonly viewer: RailViewer;
  /** The outline's editing part, when the app has one. */
  readonly editor?: OutlineEditorSlot | undefined;
}

/** The Pages and Outline tabs of an open document. */
export function readerRailTabs({ source, layout, strip, viewer, editor }: ReaderRailTabsOptions): RailTab[] {
  return [
    { id: "pages", label: message("shell.rail.pages"), panel: <PagesPanel strip={strip} layout={layout} /> },
    {
      id: "outline",
      label: message("shell.rail.outline"),
      panel: <OutlinePanel source={source} viewer={viewer} layout={layout} editor={editor} />,
    },
  ];
}
