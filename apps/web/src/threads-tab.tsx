// The Threads tab's panel (O13), loaded with later.ts once a file with article threads is open
// (#380): the threads the engine read on open, the part followed last, and going to a part.

import { RAIL_DRAWER_QUERY, type ShellLayout, stepThreadPart, useMediaQuery, useStoreState } from "@laminapdf/ui";
import { ThreadsPanel } from "@laminapdf/ui/dialogs";
import type { ReactNode } from "react";
import { pageLabel } from "./outline.ts";
import type { Engine } from "./session.ts";

/**
 * The file's article threads, and Previous part / Next part through them. Where the rail covers
 * the page (a drawer), going to a part closes it, as the other tabs do.
 */
export function ThreadsTab({ engine, layout }: { engine: Engine; layout: ShellLayout }): ReactNode {
  const covers = useMediaQuery(RAIL_DRAWER_QUERY);
  const threads = useStoreState(engine, (s) => s.threads);
  const current = useStoreState(engine, (s) => s.threadPart);
  const pages = useStoreState(engine, (s) => s.pages);
  return (
    <ThreadsPanel
      threads={threads}
      current={current}
      pageName={(index) => pageLabel(pages, index)}
      step={(direction) => stepThreadPart(threads, current, direction)}
      onGo={(part) => {
        void engine.goToThreadPart(part);
        if (covers) layout.setRailOpen(false);
      }}
    />
  );
}
