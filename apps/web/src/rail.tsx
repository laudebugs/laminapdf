// The rail's tabs for an open document (R06): Pages, the viewer's thumbnails strip, and Outline,
// the document's outline as a tree, read from the document worker the first time the tab shows.
// The outline marks the section at the top of the view, worked out at most every 150 ms while the
// view scrolls. Where the rail covers the page (a drawer, or the phone's Contents sheet), going
// somewhere from it closes it, so the page shows. While find is open, a Results tab lists its
// matches. Once later.ts has loaded, the Outline tab's editing part (bookmarks.tsx, O11) adds Edit
// bookmarks and reads the outline again after every edit, undo or redo.

import {
  currentOutlineItem,
  EmptyState,
  FindResults,
  message,
  OutlineTree,
  type OutlineTreeItem,
  RAIL_DRAWER_QUERY,
  type RailTab,
  type ShellLayout,
  useMediaQuery,
  useStoreState,
} from "@laminapdf/ui";
import type { BookmarkItem } from "@laminapdf/ui/editing";
import type { ThumbnailStrip, Viewer } from "@laminapdf/viewer";
import { type ComponentType, type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { resultHeadings } from "./find-headings.ts";
import { downThePage, TOP_PROBE } from "./outline.ts";
import type { Engine } from "./session.ts";

/** How often the current section is worked out while the view moves, in ms. */
const SECTION_EVERY = 150;

/** The outline's items before it is read. */
const NO_ITEMS: readonly OutlineTreeItem[] = [];

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
      timer ??= setTimeout(update, SECTION_EVERY);
    });
    return () => {
      off();
      clearTimeout(timer);
    };
  }, [viewer, items]);
  return current;
}

/**
 * The Outline tab's editing part, once later.ts has loaded: it shows the read view (`view`) with an
 * Edit bookmarks button, or the bookmark editor, and reads the outline again after every edit,
 * undo or redo.
 */
export type OutlineEditor = ComponentType<OutlineEditingProps>;

/** What the Outline tab hands its editing part. */
export interface OutlineEditingProps {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly items: readonly BookmarkItem[];
  readonly current: string | undefined;
  readonly onNavigate: (id: string) => void;
  /** The read view: the tree, or the empty state. */
  readonly view: ReactNode;
}

/** The outline, read when the tab first shows; with the editor once it has loaded. */
function OutlinePanel({
  engine,
  viewer,
  layout,
  editor,
}: {
  engine: Engine;
  viewer: Viewer;
  layout: ShellLayout;
  editor: OutlineEditor | undefined;
}): ReactNode {
  const outline = useStoreState(engine, (s) => s.outline);
  const closeIfCovering = useCloseAfterJump(layout);
  // Read once per document: a document opened while the tab shows clears it, so it reads again.
  useEffect(() => {
    if (outline === undefined) void engine.loadOutline();
  }, [engine, outline]);
  const items = typeof outline === "object" ? outline.items : NO_ITEMS;
  const current = useCurrentSection(viewer, items);
  const go = (id: string) => {
    void engine.goToOutlineItem(id);
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
  const Editor = editor;
  return Editor === undefined ? (
    view
  ) : (
    <Editor engine={engine} viewer={viewer} items={items} current={current} onNavigate={go} view={view} />
  );
}

/** Find's results, headed by page label and outline section; the outline is read for the headings. */
function ResultsPanel({ engine, viewer, layout }: { engine: Engine; viewer: Viewer; layout: ShellLayout }): ReactNode {
  const pages = useStoreState(engine, (s) => s.pages);
  const outline = useStoreState(engine, (s) => s.outline);
  const closeIfCovering = useCloseAfterJump(layout);
  useEffect(() => {
    if (outline === undefined) void engine.loadOutline();
  }, [engine, outline]);
  const items = typeof outline === "object" ? outline.items : NO_ITEMS;
  const heading = useMemo(() => resultHeadings(pages, items), [pages, items]);
  return <FindResults find={viewer.find} heading={heading} onNavigate={closeIfCovering} />;
}

/** The rail's tabs while a document is open; none otherwise. */
export function railTabs(
  engine: Engine,
  layout: ShellLayout,
  view:
    | {
        readonly strip: ThumbnailStrip | undefined;
        readonly viewer: Viewer | undefined;
        /** Find is open: the Results tab shows. */
        readonly finding: boolean;
        /** The bookmark editor, once later.ts has loaded. */
        readonly editor: OutlineEditor | undefined;
      }
    | undefined,
): RailTab[] {
  const strip = view?.strip;
  const viewer = view?.viewer;
  if (strip === undefined || viewer === undefined) return [];
  return [
    { id: "pages", label: message("shell.rail.pages"), panel: <PagesPanel strip={strip} layout={layout} /> },
    {
      id: "outline",
      label: message("shell.rail.outline"),
      panel: <OutlinePanel engine={engine} viewer={viewer} layout={layout} editor={view?.editor} />,
    },
    // Find's contextual tab, while find is open (R05); closing find takes it away.
    ...(view?.finding === true
      ? [
          {
            id: "results",
            label: message("shell.rail.results"),
            panel: <ResultsPanel engine={engine} viewer={viewer} layout={layout} />,
          },
        ]
      : []),
  ];
}
