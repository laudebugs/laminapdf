// The reader's lifecycle as a small state machine: empty → opening → (needs-password →) open, or
// error. Hand-written rather than a library: six states and nine events fit in one switch, each
// transition is a plain function of state and event, and every one has a unit test.

import type { PlatformFile } from "../platform/types.js";

/** What the reader is doing. A password is never part of it: it goes to the opener and no further. */
export type ReaderState =
  | { readonly phase: "empty" }
  | { readonly phase: "opening"; readonly file: PlatformFile }
  | { readonly phase: "needs-password"; readonly file: PlatformFile; readonly wrong: boolean }
  | { readonly phase: "open"; readonly file: PlatformFile; readonly pageCount: number; readonly mode: InteractionMode }
  | { readonly phase: "error"; readonly file: PlatformFile; readonly error: ReaderError };

/** How pointer input on the page is read. Only selection in v0.1. */
export type InteractionMode = "select";

/** Why a document couldn't open: its error code (ADR 0026), its message for the log and its `data`. */
export interface ReaderError {
  readonly code: string;
  readonly message: string;
  /** The error's code-specific fields (`fetch-failed`'s `reason` …), when it has them. */
  readonly data?: Readonly<Record<string, unknown>>;
}

/** What can happen to the reader. */
export type ReaderEvent =
  | { readonly type: "open"; readonly file: PlatformFile }
  | { readonly type: "opened"; readonly pageCount: number }
  | { readonly type: "password-required" }
  | { readonly type: "submit-password" }
  | { readonly type: "wrong-password" }
  | { readonly type: "failed"; readonly error: ReaderError }
  | { readonly type: "cancel" }
  | { readonly type: "close" }
  | { readonly type: "set-mode"; readonly mode: InteractionMode };

/** The reader before any document. */
export const EMPTY: ReaderState = { phase: "empty" };

/** The state after `event`; the same state when `event` doesn't apply to it. */
export function transition(state: ReaderState, event: ReaderEvent): ReaderState {
  // Opening a document replaces whatever was there, and closing ends it, from any state.
  if (event.type === "open") return { phase: "opening", file: event.file };
  if (event.type === "close") return state.phase === "empty" ? state : EMPTY;
  switch (state.phase) {
    case "empty":
      return state;
    case "opening":
      return fromOpening(state, event);
    case "needs-password":
      if (event.type === "submit-password") return { phase: "opening", file: state.file };
      if (event.type === "cancel") return EMPTY;
      return state;
    case "open":
      if (event.type === "set-mode") return event.mode === state.mode ? state : { ...state, mode: event.mode };
      return state;
    case "error":
      return state;
  }
}

function fromOpening(state: ReaderState & { phase: "opening" }, event: ReaderEvent): ReaderState {
  switch (event.type) {
    case "opened":
      return { phase: "open", file: state.file, pageCount: event.pageCount, mode: "select" };
    case "password-required":
      return { phase: "needs-password", file: state.file, wrong: false };
    case "wrong-password":
      return { phase: "needs-password", file: state.file, wrong: true };
    case "failed":
      return { phase: "error", file: state.file, error: event.error };
    case "cancel":
      return EMPTY;
    default:
      return state;
  }
}
