// Back and Forward through the open tab's reading history. The desktop's webview has no address
// bar or browser history, so the commands, toolbar buttons and mouse Back/Forward buttons step
// the viewer's `NavigationHistory` directly.

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
export function historyTargetLabel(entry: HistoryState["back"]): string | undefined {
  return entry === undefined ? undefined : String(entry.point.page + 1);
}

/** Whether an open modal dialog blocks background mouse navigation. */
function hasOpenModal(): boolean {
  return [...document.querySelectorAll('[aria-modal="true"]')].some((el) => el.closest("[data-exiting]") === null);
}

/**
 * Handles mouse Back/Forward buttons (buttons 3 and 4) for `viewer`, ignoring events inside
 * editable fields, dialogs, or when any modal is open.
 */
export function handleHistoryMouseUp(event: MouseEvent, history: Pick<Viewer["history"], "back" | "forward">): void {
  if (event.defaultPrevented || (event.button !== 3 && event.button !== 4)) return;
  if (
    hasOpenModal() ||
    (event.target instanceof Element &&
      (isEditable(event.target) || event.target.closest('[role="dialog"], [role="alertdialog"]') !== null))
  ) {
    return;
  }
  event.preventDefault();
  if (event.button === 3) history.back();
  else history.forward();
}

/** Command handlers for `nav.back` and `nav.forward`, always present so shortcuts stay visible on the start screen. */
export function historyActions(
  history: Pick<Viewer["history"], "back" | "forward"> | undefined,
): NonNullable<CommandContext["actions"]> {
  return {
    "nav.back": () => {
      history?.back();
    },
    "nav.forward": () => {
      history?.forward();
    },
  };
}

/**
 * Wires the open tab's `NavigationHistory` to `ReaderTopBar`, `nav.back`/`nav.forward`, and
 * mouse Back/Forward buttons (buttons 3 and 4).
 */
export function useDesktopHistory(viewer: Viewer | undefined): DesktopHistory {
  const state = useStoreState(viewer?.history ?? NO_HISTORY, (s) => s);
  const actions = useMemo(() => historyActions(viewer?.history), [viewer]);

  useEffect(() => {
    if (viewer === undefined) return;
    const onMouseUp = (event: MouseEvent) => handleHistoryMouseUp(event, viewer.history);
    window.addEventListener("mouseup", onMouseUp);
    return () => window.removeEventListener("mouseup", onMouseUp);
  }, [viewer]);

  const history =
    viewer === undefined
      ? undefined
      : { back: historyTargetLabel(state.back), forward: historyTargetLabel(state.forward) };
  return { history, actions };
}
