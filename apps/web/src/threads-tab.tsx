// The Threads tab's panel (O13), loaded with later.ts once a file with article threads is open
// (#380): the threads the engine read on open, the part followed last, and going to a part
// (thread-parts.ts).

import { RAIL_DRAWER_QUERY, type ShellLayout, stepThreadPart, useMediaQuery, useStoreState } from "@laminapdf/ui";
import { ThreadsPanel } from "@laminapdf/ui/dialogs";
import type { ReactNode } from "react";
import { pageLabel } from "./outline.ts";
import type { Engine } from "./session.ts";
import { goToThreadPart, useFollowedPart } from "./thread-parts.ts";

/**
 * The file's article threads, and Previous part / Next part through them. Where the rail covers
 * the page (a drawer), going to a part closes it, as the other tabs do.
 */
export function ThreadsTab({ engine, layout }: { engine: Engine; layout: ShellLayout }): ReactNode {
  const covers = useMediaQuery(RAIL_DRAWER_QUERY);
  const threads = useStoreState(engine, (s) => s.threads);
  const current = useFollowedPart(threads);
  const viewer = useStoreState(engine, (s) => s.viewer);
  const pages = useStoreState(engine, (s) => s.pages);
  return (
    <ThreadsPanel
      threads={threads}
      current={current}
      pageName={(index) => pageLabel(pages, index)}
      step={(direction) => stepThreadPart(threads, current, direction)}
      onGo={(part) => {
        if (viewer !== undefined) goToThreadPart(viewer, threads, part);
        if (covers) layout.setRailOpen(false);
      }}
    />
  );
}
