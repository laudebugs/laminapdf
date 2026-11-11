// The edit session as the page sees it (E1 plan §5): a store of the open document's history, which
// the worker holds, and the calls that change it. Calls run one after another in the order they
// were made, so three quick presses of Undo undo three steps; each redraws the pages it touched
// before the next starts. The store's state is what an interface reads: the steps, whether a call
// is running, why editing is blocked, and the last undo or redo that restored only part of a step,
// which stays until the next edit (a toast is never the only place it shows).

import type { Op } from "@laminapdf/sdk";
import {
  type EditHistory,
  type EditResult,
  EMPTY_HISTORY,
  type OpenedDocument,
  type RevertedStep,
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
}

/** What the session drives: the viewer's edit calls, which redraw the pages they touch. */
export interface EditSessionHost {
  applyEdit(ops: readonly Op[], options: { readonly label: string }): Promise<EditResult>;
  undoEdit(): Promise<EditResult>;
  redoEdit(): Promise<EditResult>;
  goToStep(position: number): Promise<EditResult>;
  /** The worker's history now, read after a call failed (a failed edit can clear the steps undone). */
  editHistory(): Promise<EditHistory>;
}

/** Called after an undo or redo, with each step it took back or made again. */
export type RevertListener = (reverted: readonly RevertedStep[]) => void;

const CLOSED: EditSessionState = {
  history: EMPTY_HISTORY,
  busy: false,
  blocked: "no-document",
  notice: undefined,
  lastMs: undefined,
};

/** The open document's edit session (`viewer.edits`). */
export class EditSession {
  readonly #host: EditSessionHost;
  readonly #listeners = new Set<() => void>();
  readonly #reverted = new Set<RevertListener>();
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

  getState = (): EditSessionState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Calls `listener` after each undo or redo, with what it did; returns the function that stops it. */
  onReverted(listener: RevertListener): () => void {
    this.#reverted.add(listener);
    return () => this.#reverted.delete(listener);
  }

  /** Starts over for `document` (undefined: none is open). */
  reset(document: OpenedDocument | undefined): void {
    this.#turn += 1;
    this.#set({
      history: EMPTY_HISTORY,
      blocked: document === undefined ? "no-document" : document.encrypted ? "encrypted" : undefined,
      notice: undefined,
      lastMs: undefined,
    });
  }

  /**
   * Takes a history the worker gave outside a call of this session: a restarted worker's replay,
   * or an empty one when the edits were lost.
   */
  follow(history: EditHistory): void {
    this.#set({ history, ...(history.diverged ? { blocked: "diverged" as const } : {}) });
  }

  /** Applies `ops` as one step named `label` ("Rotate page 5"). Rejects as the edit failed. */
  apply(ops: readonly Op[], label: string): Promise<EditResult> {
    return this.#run(() => this.#host.applyEdit(ops, { label }), true);
  }

  /** Undoes the newest done step; resolves undefined when there was none. */
  undo(): Promise<EditResult | undefined> {
    return this.#run(() => (this.#state.history.position > 0 ? this.#host.undoEdit() : undefined), false);
  }

  /** Redoes the oldest undone step; resolves undefined when there was none. */
  redo(): Promise<EditResult | undefined> {
    const { history } = this.#state;
    return this.#run(() => (history.position < history.steps.length ? this.#host.redoEdit() : undefined), false);
  }

  /** Undoes or redoes steps until `position` of them are done, in one call. */
  goTo(position: number): Promise<EditResult | undefined> {
    return this.#run(
      () => (position === this.#state.history.position ? undefined : this.#host.goToStep(position)),
      false,
    );
  }

  /**
   * Runs `call` after the calls before it, unless editing is blocked by then; takes its history.
   * An edit (`clears`) drops the last partial undo's notice.
   */
  #run<R extends EditResult | undefined>(call: () => Promise<R> | R, clears: boolean): Promise<R> {
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
        await this.#reread(turn);
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
