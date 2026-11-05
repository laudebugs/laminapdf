// The reader's controller: it owns the lifecycle machine, asks the platform for files, and hands
// each one to the opener the shell gives it (the viewer client's open, in the web app). The
// toolbar and menus call its methods, so command ids (U1b) can call the same ones.

import type { Platform, PlatformFile } from "../platform/types.js";
import { EMPTY, type ReaderError, type ReaderEvent, type ReaderState, transition } from "./reader-machine.js";

/** A document the opener opened: its page count, and how to free it once the reader lets it go. */
export interface OpenedDocument {
  readonly pageCount: number;
  /** Frees the document (its worker's memory); called when it is closed, replaced or superseded. */
  close?(): void;
}

/** Opens `file`, with `password` when given. */
export type Opener = (file: PlatformFile, password: string | undefined) => Promise<OpenedDocument>;

/** The codes an opener's error carries when the document needs a password (ADR 0026). */
const NEEDS_PASSWORD = "password-required";
const WRONG_PASSWORD = "wrong-password";

/** Holds the reader's state for React (`getState`/`subscribe`) and runs its transitions. */
export class ReaderController {
  readonly #platform: Platform;
  readonly #opener: Opener;
  readonly #listeners = new Set<() => void>();
  #state: ReaderState = EMPTY;
  /** Counts opens, so a slower earlier one can't overwrite a later one's outcome. */
  #attempt = 0;
  /** The document that is open, freed when another replaces it or the reader closes. */
  #document: OpenedDocument | undefined;

  constructor(platform: Platform, opener: Opener) {
    this.#platform = platform;
    this.#opener = opener;
  }

  getState = (): ReaderState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Asks the platform for a document and opens it; nothing happens if the user cancels. */
  async openFromPicker(): Promise<void> {
    const [file] = await this.#platform.openFiles({ multiple: false });
    if (file !== undefined) await this.open(file);
  }

  /** Opens `file`, replacing whatever was open. */
  open(file: PlatformFile): Promise<void> {
    this.#release();
    this.#dispatch({ type: "open", file });
    return this.#attemptOpen(file, undefined);
  }

  /** Tries `password` on the document waiting for one. It is passed on, never kept. */
  submitPassword(password: string): Promise<void> {
    const state = this.#state;
    if (state.phase !== "needs-password") return Promise.resolve();
    this.#dispatch({ type: "submit-password" });
    return this.#attemptOpen(state.file, password);
  }

  /** Gives up on the document being opened, or asking for a password, and goes back to empty. */
  cancel(): void {
    this.#attempt += 1;
    this.#dispatch({ type: "cancel" });
  }

  /** Closes the open document, or the error, back to empty. */
  close(): void {
    this.#attempt += 1;
    this.#release();
    this.#dispatch({ type: "close" });
  }

  #release(): void {
    this.#document?.close?.();
    this.#document = undefined;
  }

  async #attemptOpen(file: PlatformFile, password: string | undefined): Promise<void> {
    this.#attempt += 1;
    const attempt = this.#attempt;
    let event: ReaderEvent;
    try {
      const opened = await this.#opener(file, password);
      // A later open, a cancel or a close came first: nothing will show this document.
      if (attempt !== this.#attempt) return opened.close?.();
      this.#document = opened;
      event = { type: "opened", pageCount: opened.pageCount };
    } catch (error) {
      const failure = asReaderError(error);
      if (failure.code === NEEDS_PASSWORD) event = { type: "password-required" };
      else if (failure.code === WRONG_PASSWORD) event = { type: "wrong-password" };
      else event = { type: "failed", error: failure };
    }
    if (attempt === this.#attempt) this.#dispatch(event);
  }

  #dispatch(event: ReaderEvent): void {
    const next = transition(this.#state, event);
    if (next === this.#state) return;
    this.#state = next;
    for (const listener of this.#listeners) listener();
  }
}

/**
 * An opener's error as a code and message: ADR 0026 errors carry a string `code`; anything else,
 * a DOMException's legacy numeric code included, is unknown.
 */
function asReaderError(error: unknown): ReaderError {
  const code =
    typeof error === "object" && error !== null && "code" in error && typeof error.code === "string"
      ? error.code
      : "unknown";
  const text = error instanceof Error ? error.message : String(error);
  return { code, message: text };
}
