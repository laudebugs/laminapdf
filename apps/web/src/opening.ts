// Next and Previous thread part (O13) while the open document has article threads: without
// threads the commands have no handler and stay out of view.

import { stepThreadPart, useStoreState } from "@laminapdf/ui";
import { useMemo } from "react";
import type { Engine } from "./session.ts";

/** The Next and Previous thread part handlers, while the open document has threads. */
export type ThreadActions = Readonly<Partial<Record<"nav.next-thread-part" | "nav.previous-thread-part", () => void>>>;

/** No handlers: the commands stay out of view. */
const NO_THREAD_ACTIONS: ThreadActions = {};

/** Next and Previous thread part over the open document's threads. */
export function useThreadActions(engine: Engine): ThreadActions {
  const threads = useStoreState(engine, (s) => s.threads);
  return useMemo(() => {
    if (threads.length === 0) return NO_THREAD_ACTIONS;
    const step = (direction: 1 | -1) => () => {
      const part = stepThreadPart(threads, engine.getState().threadPart, direction);
      if (part !== undefined) void engine.goToThreadPart(part);
    };
    return { "nav.next-thread-part": step(1), "nav.previous-thread-part": step(-1) };
  }, [engine, threads]);
}
