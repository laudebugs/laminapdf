// The reader's controller: it owns the lifecycle machine, asks the platform for files, and hands
// each one to the opener the shell gives it (the viewer client's open, in the web app). The
// toolbar and menus call its methods, so registered commands can call the same ones.

import type { Platform, PlatformFile } from "../platform/types.js";
import type { InteractionMode } from "../tools/tools.js";
import { EMPTY, type ReaderError, type ReaderEvent, type ReaderState, transition } from "./reader-machine.js";

/** A document the opener opened: its page count, and how to free it once the reader lets it go. */
export interface OpenedDocument {
  readonly pageCount: number;
  /** Frees the document (its worker's memory); called when it is closed, replaced or superseded. */
  close?(): void;
}

/** Opens `file`, with `password` when given. */
export type Opener = (file: PlatformFile, password: string | undefined) => Promise<OpenedDocument>;

/**
 * Asked before the open document goes (closed, or replaced by another): resolves true to let it go,
 * false to keep it. The editor asks the user when there are unsaved changes.
 */
export type CloseGuard = () => Promise<boolean>;

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
  #guard: CloseGuard | undefined;

  constructor(platform: Platform, opener: Opener) {
    this.#platform = platform;
    this.#opener = opener;
  }

  /** The reader's current state. */
  getState = (): ReaderState => this.#state;

  /** Calls `listener` after each transition; returns the function that stops it. */
  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Asks the platform for a document and opens it; nothing happens if the user cancels. */
  async openFromPicker(): Promise<void> {
    const [file] = await this.#platform.openFiles({ multiple: false });
    if (file !== undefined) await this.open(file);
  }

  /**
   * Sets what is asked before the user's open or close lets the open document go (undefined:
   * nothing). Returns the function that removes it.
   */
  setCloseGuard(guard: CloseGuard | undefined): () => void {
    this.#guard = guard;
    return () => {
      if (this.#guard === guard) this.#guard = undefined;
    };
  }

  /** The guard to ask before the open document goes; undefined with no guard or no document open. */
  #guardNow(): CloseGuard | undefined {
    return this.#state.phase === "open" ? this.#guard : undefined;
  }

  /**
   * Opens `file`, replacing whatever was open, once the close guard lets the open document go;
   * `{ discard: true }` skips the guard (the shell reopening the same file, a backend switch).
   */
  open(file: PlatformFile, options: { readonly discard?: boolean } = {}): Promise<void> {
    const guard = options.discard === true ? undefined : this.#guardNow();
    // Without a guard to ask, the open starts at once: the state is `opening` when this returns.
    if (guard !== undefined) return guard().then((leave) => (leave ? this.#openNow(file) : undefined));
    return this.#openNow(file);
  }

  #openNow(file: PlatformFile): Promise<void> {
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

  /** Reads pointer input on the open document as interaction mode `mode` (a registered tool's name). */
  setMode(mode: InteractionMode): void {
    this.#dispatch({ type: "set-mode", mode });
  }

  /** Closes the document as the user asked, once the close guard lets it go; resolves whether it closed. */
  async requestClose(): Promise<boolean> {
    const guard = this.#guardNow();
    if (guard !== undefined && !(await guard())) return false;
    this.close();
    return true;
  }

  /**
   * Closes the document, whether open, opening, asking for a password or failed, back to empty,
   * without asking the close guard: for closes the reader didn't ask for, such as a backend
   * switch reopening the same file. A close the reader asks for goes through `requestClose()`.
   */
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
  const data = typeof error === "object" && error !== null && "data" in error ? error.data : undefined;
  return typeof data === "object" && data !== null
    ? { code, message: text, data: data as Readonly<Record<string, unknown>> }
    : { code, message: text };
}
