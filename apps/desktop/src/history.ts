// Back and Forward through the open tab's reading history: the desktop's webview has no address
// bar and no browser history of its own, so both commands and the top bar's buttons step the
// viewer's `NavigationHistory` directly, and mouse back/forward buttons do the same.

import { type CommandContext, type ExternalStore, isEditable, useStoreState } from "@laminapdf/ui";
import type { HistoryState, Viewer } from "@laminapdf/viewer";
import { useEffect, useMemo } from "react";

const EMPTY_HISTORY: HistoryState = { current: undefined, back: undefined, forward: undefined };
const NO_HISTORY: ExternalStore<HistoryState> = {
  getState: () => EMPTY_HISTORY,
  subscribe: () => () => undefined,
};

/** Back and Forward labels for `ReaderTopBar`, and their command handlers. */
export interface DesktopHistory {
  /** Page labels for the top bar's Back and Forward buttons, or undefined when no document is open. */
  readonly history: { readonly back: string | undefined; readonly forward: string | undefined } | undefined;
  /** Command handlers for `nav.back` and `nav.forward`. */
  readonly actions: NonNullable<CommandContext["actions"]>;
}

/** Formats a history entry's target page as a 1-based page number. */
function targetLabel(entry: HistoryState["back"]): string | undefined {
  return entry === undefined ? undefined : String(entry.point.page + 1);
}

/**
 * Wires the open tab's `NavigationHistory` to `ReaderTopBar`, `nav.back`/`nav.forward`, and
 * mouse Back/Forward buttons (buttons 3 and 4).
 */
export function useDesktopHistory(viewer: Viewer | undefined): DesktopHistory {
  const state = useStoreState(viewer?.history ?? NO_HISTORY, (s) => s);
  const actions = useMemo<NonNullable<CommandContext["actions"]>>(() => {
    if (viewer === undefined) return {};
    return {
      "nav.back": () => {
        viewer.history.back();
      },
      "nav.forward": () => {
        viewer.history.forward();
      },
    };
  }, [viewer]);

  useEffect(() => {
    if (viewer === undefined) return;
    const onMouseUp = (event: MouseEvent) => {
      if (event.button !== 3 && event.button !== 4) return;
      if (event.target instanceof Element && (isEditable(event.target) || event.target.closest('[role="dialog"]'))) {
        return;
      }
      const moved = event.button === 3 ? viewer.history.back() : viewer.history.forward();
      if (moved) event.preventDefault();
    };
    window.addEventListener("mouseup", onMouseUp);
    return () => window.removeEventListener("mouseup", onMouseUp);
  }, [viewer]);

  const history =
    viewer === undefined ? undefined : { back: targetLabel(state.back), forward: targetLabel(state.forward) };
  return { history, actions };
}
