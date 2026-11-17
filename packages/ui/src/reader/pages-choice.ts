// The Pages group that Print and Export share: every page, the page in view, or a typed range,
// and the range's error, which choosing again or typing clears.

import { useState } from "react";
import { parsePrintPages } from "./print-pages.js";

/** Which pages a dialog's Pages group names: every page, the one in view, or a typed range. */
export type PagesMode = "all" | "current" | "range";

/** What a dialog with a Pages group knows about the document. */
export interface PagesContext {
  readonly pageCount: number;
  /** The page in view, 0-based. */
  readonly currentPage: number;
  readonly labels?: readonly string[] | undefined;
}

/** The 0-based pages `mode` names, or the range's error. */
export function chosenPages(
  mode: PagesMode,
  range: string,
  context: PagesContext,
): { pages: readonly number[] } | { error: string } {
  if (mode === "all") return { pages: Array.from({ length: context.pageCount }, (_, i) => i) };
  if (mode === "current") return { pages: [context.currentPage] };
  const result = parsePrintPages(range, context.pageCount, context.labels);
  return result.ok ? { pages: result.pages } : { error: result.error };
}

/** The page in view as the document names it: its label, else its number. */
export function currentLabel(context: Pick<PagesContext, "currentPage" | "labels">): string {
  return context.labels?.[context.currentPage] ?? String(context.currentPage + 1);
}

/** The Pages group's state, and what changes it. */
export interface PagesChoice {
  readonly mode: PagesMode;
  readonly range: string;
  readonly error: string | undefined;
  setMode(mode: PagesMode): void;
  /** The range field took focus: its radio is chosen, and an error under it stays. */
  chooseRange(): void;
  setRange(range: string): void;
}

/** The Pages group's state, with `showError` for a submit and `reset` for each opening. */
export function usePagesChoice(): PagesChoice & {
  showError(error: string | undefined): void;
  reset(mode: PagesMode, range?: string): void;
} {
  const [mode, setModeState] = useState<PagesMode>("all");
  const [range, setRangeState] = useState("");
  const [error, setError] = useState<string | undefined>();
  return {
    mode,
    range,
    error,
    setMode: (next) => {
      setModeState(next);
      setError(undefined);
    },
    chooseRange: () => setModeState("range"),
    setRange: (next) => {
      setRangeState(next);
      setModeState("range");
      setError(undefined);
    },
    showError: setError,
    reset: (next, text) => {
      setModeState(next);
      if (text !== undefined) setRangeState(text);
      setError(undefined);
    },
  };
}
