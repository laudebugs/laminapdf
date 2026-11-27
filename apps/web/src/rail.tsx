// The rail's tabs for an open document (R06): the shared Pages and Outline tabs (`@laminapdf/ui`'s
// `readerRailTabs`, which the desktop uses too), and the tabs only the web has. Once later.ts has
// loaded, the Outline tab's editing part (bookmarks.tsx, O11) adds Edit bookmarks and reads the
// outline again after every edit, undo or redo. While find is open, a Results tab lists its
// matches. A file with article threads gets a Threads tab (O13), which follows them part by part; a
// file with attachments, or any file in Organise, an Attachments tab (O14).

import {
  FindResults,
  message,
  type OutlineEditorSlot,
  type OutlineTreeItem,
  type RailTab,
  readerRailTabs,
  type ShellLayout,
  useCloseAfterJump,
  useStoreState,
} from "@laminapdf/ui";
import type { ThumbnailStrip, Viewer } from "@laminapdf/viewer";
import { type ComponentType, lazy, type ReactNode, Suspense, useEffect, useMemo } from "react";
import type { AttachmentsContext } from "./editing.tsx";
import { resultHeadings } from "./find-headings.ts";
import type { Engine } from "./session.ts";

/** The outline's items before it is read. */
const NO_ITEMS: readonly OutlineTreeItem[] = [];

/**
 * The Outline tab's editing part, once later.ts has loaded: it shows the read view (`view`) with an
 * Edit bookmarks button, or the bookmark editor, and reads the outline again after every edit,
 * undo or redo.
 */
export type OutlineEditor = ComponentType<OutlineEditingProps>;

/** What the Outline tab hands its editing part, which reads the view and the outline from the engine. */
export interface OutlineEditingProps {
  readonly engine: Engine;
  readonly current: string | undefined;
  readonly onNavigate: (id: string) => void;
  /** The read view: the tree, or the empty state. */
  readonly view: ReactNode;
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

/** The Threads tab's id in the rail. */
const THREADS_TAB = "threads";

/** The Threads tab's panel, loaded with later.ts once a file with threads is open (#380). */
const LazyThreadsTab = lazy(() => import("./later.ts").then((m) => ({ default: m.ThreadsTab })));

/** The Attachments tab's id in the rail. */
const ATTACHMENTS_TAB = "attachments";

/** The Attachments tab's panel (O14), loaded with later.ts once a document is open (#380). */
const LazyAttachmentsTab = lazy(() => import("./later.ts").then((m) => ({ default: m.AttachmentsTab })));

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
        /** The file has article threads: the Threads tab shows. */
        readonly threaded: boolean;
        /** The file carries attachments: the Attachments tab shows, as it does in Organise. */
        readonly attached: boolean;
        /** What the Attachments tab works with. */
        readonly attachments: AttachmentsContext;
      }
    | undefined,
): RailTab[] {
  const strip = view?.strip;
  const viewer = view?.viewer;
  if (strip === undefined || viewer === undefined) return [];
  const Editor = view?.editor;
  const editor: OutlineEditorSlot | undefined =
    Editor === undefined ? undefined : (props) => <Editor engine={engine} {...props} />;
  return [
    ...readerRailTabs({ source: engine, layout, strip, viewer, editor }),
    // Only for a file with article threads (O13).
    ...(view?.threaded === true
      ? [
          {
            id: THREADS_TAB,
            label: message("shell.rail.threads"),
            panel: (
              <Suspense fallback={null}>
                <LazyThreadsTab engine={engine} layout={layout} />
              </Suspense>
            ),
          },
        ]
      : []),
    // For a file with attachments, and in Organise to add some (O14: Read hides an empty tab).
    ...(view?.attached === true || view?.attachments.organising === true
      ? [
          {
            id: ATTACHMENTS_TAB,
            label: message("shell.rail.attachments"),
            panel: (
              <Suspense fallback={null}>
                <LazyAttachmentsTab engine={engine} layout={layout} {...view.attachments} />
              </Suspense>
            ),
          },
        ]
      : []),
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
