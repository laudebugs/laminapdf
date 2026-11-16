// The Pages group that Print and Export share: every page, the page in view, or a typed range,
// and the range's error, which choosing again or typing clears.

import { useState } from "react";
import type { PagesMode } from "./print-pages.js";

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
