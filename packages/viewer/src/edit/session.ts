// The edit session as the page sees it (E1 plan §5): a store of the open document's history, which
// the worker holds, and the calls that change it. Calls run one after another in the order they
// were made, so three quick presses of Undo undo three steps; each redraws the pages it touched
// before the next starts. The store's state is what an interface reads: the steps, whether a call
// is running, why editing is blocked, and the last undo or redo that restored only part of a step,
// which stays until the next edit (a toast is never the only place it shows). Saves queue with the
// calls too (E1 plan §7): a save writes a copy, the caller writes it where the file lives, and only
// then does the document take it as saved; edits wait until it is done.

import type { Op, PlannedSave, SaveOptions } from "@laminapdf/sdk";
import {
  type EditHistory,
  type EditResult,
  EMPTY_HISTORY,
  type OpenedDocument,
  type RevertedStep,
  type SavedCopy,
} from "../protocol.js";

/** Why the document can't be edited now. */
export type EditBlock =
  /** No document is open. */
  | "no-document"
  /** It is encrypted: its edits can't be written yet (E11 adds encrypting on save). */
  | "encrypted"
  /** An edit failed and couldn't be undone: it must be opened again. */
  | "diverged";

/** What the edit session's interface reads. */
export interface EditSessionState {
  readonly history: EditHistory;
  /** A call is running, or waiting for the one before it. */
  readonly busy: boolean;
  /** Why editing is blocked; undefined when it isn't. */
  readonly blocked: EditBlock | undefined;
  /** The last undo or redo that left part of a step (or all of it) alone, until the next edit. */
  readonly notice: RevertedStep | undefined;
  /** How long the last call took from the call to its pages redrawn, in ms (E1 plan §6's measure). */
  readonly lastMs: number | undefined;
  /** A save is writing: edits, undo and redo wait for it, and the interface says "Saving…". */
  readonly saving: boolean;
}

/** What the session drives: the viewer's edit calls, which redraw the pages they touch. */
export interface EditSessionHost {
  applyEdit(ops: readonly Op[], options: { readonly label: string }): Promise<EditResult>;
  undoEdit(): Promise<EditResult>;
  redoEdit(): Promise<EditResult>;
  goToStep(position: number): Promise<EditResult>;
  /** The worker's history now, read after a call failed (a failed edit can clear the steps undone). */
  editHistory(): Promise<EditHistory>;
  /** How a save with `options` would go. */
  planSave(options: SaveOptions): Promise<PlannedSave>;
  /** A copy of the document as a save with `options` writes it; the document stays unsaved. */
  copyEdits(options: SaveOptions): Promise<SavedCopy>;
  /** Makes the copy `token` names the saved file, once it is written. */
  adoptEdits(token: number): Promise<EditHistory>;
}

/** Called after an undo or redo, with each step it took back or made again. */
export type RevertListener = (reverted: readonly RevertedStep[]) => void;

/** A call of the session that failed: which, the edit's label for an edit, and why. */
export interface EditFailure {
  readonly call: "apply" | "undo" | "redo" | "goTo";
  readonly label?: string;
  readonly error: unknown;
}

/** Called when a call failed, so the interface can say so: nothing else reports it. */
export type FailureListener = (failure: EditFailure) => void;

const CLOSED: EditSessionState = {
  history: EMPTY_HISTORY,
  busy: false,
  blocked: "no-document",
  notice: undefined,
  lastMs: undefined,
  saving: false,
};

/**
 * A session with no document, which can do nothing: what an interface reads while none is open,
 * in place of a store of its own.
 */
export const CLOSED_SESSION = {
  /** The closed state: no steps, nothing unsaved, edits blocked. */
  getState: (): EditSessionState => CLOSED,
  /** Never calls `listener`: a closed session doesn't change. */
  subscribe:
    (_listener: () => void): (() => void) =>
    () =>
      undefined,
  /** Does nothing. */
  undo: (): Promise<undefined> => Promise.resolve(undefined),
  /** Does nothing. */
  redo: (): Promise<undefined> => Promise.resolve(undefined),
  /** Does nothing. */
  goTo: (_position: number): Promise<undefined> => Promise.resolve(undefined),
} as const;

/** The open document's edit session (`viewer.edits`). */
export class EditSession {
  readonly #host: EditSessionHost;
  readonly #listeners = new Set<() => void>();
  readonly #reverted = new Set<RevertListener>();
  readonly #failed = new Set<FailureListener>();
  #state: EditSessionState = CLOSED;
  /** The calls waiting, the newest last: each starts when the one before it settles. */
  #queue: Promise<unknown> = Promise.resolve();
  #waiting = 0;
  /** Counts documents, so an answer for one since replaced changes nothing. */
  #turn = 0;
  readonly #now: () => number;

  constructor(host: EditSessionHost, now: () => number = () => performance.now()) {
    this.#host = host;
    this.#now = now;
  }

  /** The session now: its history, whether a call is running, and why edits are blocked, if they are. */
  getState = (): EditSessionState => this.#state;

  /** Calls `listener` whenever the state changes; returns the function that stops it. */
  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Calls `listener` after each undo or redo, with what it did; returns the function that stops it. */
  onReverted(listener: RevertListener): () => void {
    this.#reverted.add(listener);
    return () => this.#reverted.delete(listener);
  }

  /**
   * Calls `listener` when a call fails (the edit, undo, redo or move changed nothing); returns the
   * function that stops it. The call's promise rejects too.
   */
  onFailed(listener: FailureListener): () => void {
    this.#failed.add(listener);
    return () => this.#failed.delete(listener);
  }

  /** Starts over for `document` (undefined: none is open). */
  reset(document: OpenedDocument | undefined): void {
    this.#turn += 1;
    this.#set({
      history: EMPTY_HISTORY,
      blocked: document === undefined ? "no-document" : document.encrypted ? "encrypted" : undefined,
      notice: undefined,
      lastMs: undefined,
      saving: false,
    });
  }

  /**
   * Takes a history the worker gave outside a call of this session: a restarted worker's replay,
   * or an empty one when the edits were lost.
   */
  follow(history: EditHistory): void {
    this.#set({ history, ...(history.diverged ? { blocked: "diverged" as const } : {}) });
  }

  /**
   * How a save with `options` would go: its constraint, the signatures it would break, its mode
   * and the conformance warnings. Read after the calls before it.
   */
  plan(options: SaveOptions): Promise<PlannedSave> {
    return this.#queued(() => this.#host.planSave(options));
  }

  /**
   * Saves: a copy with `options`, which `write` puts where the file lives; once that worked, the
   * document takes the copy as its saved file and the history marks the point saved. A failed
   * plan, copy or write rejects, and the document stays unsaved. `saving` is set meanwhile, and
   * edits wait.
   */
  save(options: SaveOptions, write: (copy: SavedCopy) => Promise<void>): Promise<SavedCopy> {
    const turn = this.#turn;
    return this.#queued(async () => {
      this.#set({ saving: true });
      try {
        const copy = await this.#host.copyEdits(options);
        await write(copy);
        const history = await this.#host.adoptEdits(copy.token);
        if (turn === this.#turn) this.#set({ history });
        return copy;
      } finally {
        if (turn === this.#turn) this.#set({ saving: false });
      }
    });
  }

  /** A copy of the document as a save with `options` writes it, which leaves it unsaved (Save a copy). */
  copy(options: SaveOptions): Promise<SavedCopy> {
    return this.#queued(() => this.#host.copyEdits(options));
  }

  /** Runs `call` after the calls before it, for this document only, unless editing is blocked by then. */
  #queued<R>(call: () => Promise<R>): Promise<R> {
    const turn = this.#turn;
    const run = this.#queue.then(() => {
      if (turn !== this.#turn) throw new Error("the document was closed");
      const { blocked } = this.#state;
      if (blocked !== undefined) throw new Error(`the document can't be saved: ${blocked}`);
      return call();
    });
    this.#queue = run.catch(() => undefined);
    return run;
  }

  /** Applies `ops` as one step named `label` ("Rotate page 5"). Rejects as the edit failed. */
  apply(ops: readonly Op[], label: string): Promise<EditResult> {
    return this.#run(() => this.#host.applyEdit(ops, { label }), true, { call: "apply", label });
  }

  /** Undoes the newest done step; resolves undefined when there was none. */
  undo(): Promise<EditResult | undefined> {
    return this.#run(() => (this.#state.history.position > 0 ? this.#host.undoEdit() : undefined), false, {
      call: "undo",
    });
  }

  /** Redoes the oldest undone step; resolves undefined when there was none. */
  redo(): Promise<EditResult | undefined> {
    // Read once the calls before it are done: an undo queued just before makes a step to redo.
    return this.#run(
      () => {
        const { history } = this.#state;
        return history.position < history.steps.length ? this.#host.redoEdit() : undefined;
      },
      false,
      { call: "redo" },
    );
  }

  /** Undoes or redoes steps until `position` of them are done, in one call. */
  goTo(position: number): Promise<EditResult | undefined> {
    return this.#run(
      () => (position === this.#state.history.position ? undefined : this.#host.goToStep(position)),
      false,
      { call: "goTo" },
    );
  }

  /**
   * Runs `call` after the calls before it, unless editing is blocked by then; takes its history.
   * An edit (`clears`) drops the last partial undo's notice.
   */
  #run<R extends EditResult | undefined>(
    call: () => Promise<R> | R,
    clears: boolean,
    what: Omit<EditFailure, "error">,
  ): Promise<R> {
    const turn = this.#turn;
    this.#waiting += 1;
    this.#set({ busy: true });
    const run = this.#queue.then(async () => {
      const { blocked } = this.#state;
      if (turn !== this.#turn) throw new Error("the document was closed");
      if (blocked !== undefined) throw new Error(`the document can't be edited: ${blocked}`);
      const started = this.#now();
      try {
        const result = await call();
        if (result !== undefined && turn === this.#turn) this.#took(result, clears, this.#now() - started);
        return result;
      } catch (error) {
        await this.#failedCall(turn, { ...what, error });
        throw error;
      }
    });
    const settled = run.finally(() => {
      this.#waiting -= 1;
      if (this.#waiting === 0) this.#set({ busy: false });
    });
    this.#queue = settled.catch(() => undefined);
    return run;
  }

  /** After a failed call: the worker's history read again, and the failure told to the listeners. */
  async #failedCall(turn: number, failure: EditFailure): Promise<void> {
    await this.#reread(turn);
    if (turn !== this.#turn) return;
    for (const listener of [...this.#failed]) listener(failure);
  }

  /** Reads the worker's history after a failed call; a document that diverged blocks editing. */
  async #reread(turn: number): Promise<void> {
    try {
      const history = await this.#host.editHistory();
      if (turn === this.#turn) this.follow(history);
    } catch {
      // The document went (closed, or the worker restarted): its own reset or replay follows.
    }
  }

  #took(result: EditResult, clears: boolean, ms: number): void {
    const reverted = result.reverted ?? [];
    const notice = [...reverted].reverse().find((step) => step.outcome !== "applied");
    this.#set({
      history: result.history,
      lastMs: ms,
      ...(clears ? { notice: undefined } : notice === undefined ? {} : { notice }),
      ...(result.history.diverged ? { blocked: "diverged" as const } : {}),
    });
    if (reverted.length > 0) for (const listener of [...this.#reverted]) listener(reverted);
  }

  #set(next: Partial<EditSessionState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of [...this.#listeners]) listener();
  }
}
