// The desktop's edit session (E1h): the history the web reader's document worker keeps
// (@laminapdf/viewer's edit-history), over the engine process, which makes each call. The engine
// applies an op log, an undo or a redo to its model, draws the pages from the edited file from
// then on, and takes the call back itself when that fails; this keeps the steps, their labels and
// the saved point, and turns the engine's report into the viewer's `EditResult`.

import type { EditHistory, EditResult, OpenedDocument, RevertedStep, ViewerEngine } from "@laminapdf/viewer";
import {
  clearRedo,
  historyOf,
  newSession,
  orderOf,
  type PageOrder,
  placed,
  record,
  removeStep,
  restore,
  type Session,
  snapshot,
  walkTo,
} from "@laminapdf/viewer/edit-history";

/** A page as the viewer reads it. */
type PageInfo = OpenedDocument["pages"][number];

/** An op of `@laminapdf/sdk`, as the viewer's `applyEdit` takes it. */
type Op = Parameters<ViewerEngine["applyEdit"]>[0][number];

/** What the engine is asked: `EditRequest` in Rust. */
export type EditRequest =
  | { readonly kind: "apply"; readonly log: string }
  | { readonly kind: "undo" }
  | { readonly kind: "redo" };

/** A page after an edit, as the main process reports it (`PageMeta` in Rust). */
export interface EditedPage {
  readonly id: string;
  readonly width: number;
  readonly height: number;
  readonly rotate: number;
}

/** What an edit call did: `EditOutcome` in Rust. */
export interface EditOutcome {
  readonly changed: boolean;
  readonly reversal: "applied" | "partial" | "skipped" | null;
  readonly touched: readonly string[];
  readonly pageOrder: readonly string[] | null;
  readonly pages: readonly EditedPage[];
  readonly createdOutlineItems: readonly string[];
  readonly createdAnnotations: readonly string[];
  readonly outlineRepaired: boolean;
}

/** The history of a document with no edits. */
export const NO_EDITS: EditHistory = historyOf(newSession(), false);

/** Why a document takes no more edits once a failed call couldn't be taken back. */
const DIVERGED = "an edit failed and couldn't be undone: the document and the pages drawn may differ; open it again";

/** Ops that only move pages: the moved pages draw as they did, so none is drawn again. */
const MOVE_OPS: ReadonlySet<string> = new Set(["movePages", "reversePages"]);

/** One document's edit session, over `run`, which sends a call to its engine. */
export class DesktopEdits {
  readonly #run: (request: EditRequest) => Promise<EditOutcome>;
  readonly #session: Session = newSession();
  #order: PageOrder;
  #diverged = false;
  /** The calls in order: each starts once the one before it settled. */
  #queue: Promise<unknown> = Promise.resolve();
  /** The steps that changed the page list, and whether they only moved pages. */
  readonly #reorders = new Map<number, boolean>();
  /** Each step's op log, to apply again in a restarted engine. */
  readonly #logs = new Map<number, string>();

  constructor(run: (request: EditRequest) => Promise<EditOutcome>, pages: readonly PageInfo[]) {
    this.#run = run;
    this.#order = orderOf(pages.map((page) => page.id));
  }

  /** The history as the interface reads it. */
  history(): EditHistory {
    return historyOf(this.#session, this.#diverged, this.#order);
  }

  /** Applies `ops` as one transaction, recorded as a step named `label`. */
  apply(ops: readonly Op[], label: string): Promise<EditResult> {
    return this.#serial(async () => {
      this.#editable();
      const log = JSON.stringify({ v: 1, ops });
      const outcome = await this.#call({ kind: "apply", log });
      if (!outcome.changed) return { pages: [], history: this.history() };
      clearRedo(this.#session);
      const pages = pagesOf(outcome);
      const moves = ops.length > 0 && ops.every((op) => MOVE_OPS.has(op.op));
      const moved = this.#moved(pages);
      const step = record(this.#session, label, listed(pages, outcome.touched), Date.now());
      this.#logs.set(step.id, log);
      for (const id of this.#logs.keys())
        if (!this.#session.steps.some((kept) => kept.id === id)) this.#logs.delete(id);
      if (moved) this.#reorders.set(step.id, moves);
      return {
        ...this.#result(pages, moves ? [] : outcome.touched, moved),
        createdOutlineItems: outcome.createdOutlineItems,
        createdAnnotations: outcome.createdAnnotations,
        outlineRepaired: outcome.outlineRepaired,
      };
    });
  }

  /**
   * Makes the edits again in an engine that restarted and opened the file as it is on disk:
   * every step kept, then undoes those that were undone. Undefined when there were none.
   *
   * @throws when a step fails, or the depth limit dropped the oldest ones: the document is then
   *   marked diverged, since it can't be what the history says.
   */
  replay(): Promise<EditResult | undefined> {
    return this.#serial(async () => {
      const { steps, position, dropped } = this.#session;
      if (steps.length === 0) return undefined;
      let last: EditOutcome | undefined;
      try {
        if (dropped > 0) throw new Error("the oldest edits were dropped from the history");
        for (const step of steps) {
          const log = this.#logs.get(step.id);
          if (log === undefined) throw new Error("a step has no op log");
          last = await this.#run({ kind: "apply", log });
        }
        for (let i = position; i < steps.length; i++) last = await this.#run({ kind: "undo" });
      } catch (e) {
        this.#diverged = true;
        throw new Error(DIVERGED, { cause: e });
      }
      if (last === undefined) return undefined;
      const pages = pagesOf(last);
      // Every page may differ from the file as opened: all of them are drawn again.
      return this.#result(
        pages,
        pages.map((page) => page.id),
        this.#moved(pages),
      );
    });
  }

  /** Undoes (`-1`) or redoes (`+1`) one step, reading where the history stands once the calls before it are done. */
  step(delta: -1 | 1): Promise<EditResult> {
    return this.#serial(() => this.#moveTo(this.#session.position + delta));
  }

  /** Undoes or redoes steps until `position` of them are done. */
  goTo(position: number): Promise<EditResult> {
    return this.#serial(() => this.#moveTo(position));
  }

  async #moveTo(position: number): Promise<EditResult> {
    this.#editable();
    const session = this.#session;
    const kept = snapshot(session);
    const done: RevertedStep[] = [];
    const touched = new Set<string>();
    let last: EditOutcome | undefined;
    try {
      await walkTo(
        session,
        position,
        async (way) => {
          const outcome = await this.#call({ kind: way });
          // The engine has no entry where the history has a step: they disagree, which only a bug does.
          if (outcome.reversal === null) throw new Error(`the document has nothing to ${way}`);
          for (const id of outcome.touched) touched.add(id);
          last = outcome;
          return outcome.reversal;
        },
        done,
      );
    } catch (e) {
      await this.#takeBack(kept, done);
      throw e;
    }
    if (last === undefined) return { pages: [], history: this.history() };
    const pages = pagesOf(last);
    const moved = this.#moved(pages);
    const moves = moved && done.every((move) => this.#reorders.get(move.step.id) === true);
    const result = this.#result(pages, moves ? [] : [...touched], moved);
    return { ...result, reverted: done.map((move) => ({ ...move, step: placed(move.step, this.#order) })) };
  }

  /**
   * Takes back the undos and redos in `done`, newest first, and puts the history back as `kept`
   * was, less the steps the engine dropped (a skipped undo or redo leaves it no entry).
   */
  async #takeBack(kept: Session, done: readonly RevertedStep[]): Promise<void> {
    for (const move of [...done].reverse()) {
      if (move.outcome === "skipped") continue;
      try {
        const outcome = await this.#run({ kind: move.way === "undo" ? "redo" : "undo" });
        if (outcome.reversal === null || outcome.reversal === "skipped") throw new Error(DIVERGED);
      } catch (e) {
        this.#diverged = true;
        throw new Error(DIVERGED, { cause: e });
      }
    }
    restore(this.#session, kept);
    for (const move of done) {
      if (move.outcome !== "skipped") continue;
      const index = this.#session.steps.findIndex((step) => step.id === move.step.id);
      if (index >= 0) removeStep(this.#session, index, move.way);
    }
  }

  /** Sends `request`; a refusal that says the engine's document diverged marks this one too. */
  async #call(request: EditRequest): Promise<EditOutcome> {
    try {
      return await this.#run(request);
    } catch (e) {
      if (isDiverged(e)) this.#diverged = true;
      throw e;
    }
  }

  /** The result of a call that left `pages`, of which `drawn` (IDs) are to be drawn again. */
  #result(pages: readonly PageInfo[], drawn: readonly string[], moved: boolean): EditResult {
    if (moved) this.#order = orderOf(pages.map((page) => page.id));
    return { pages: listed(pages, drawn), history: this.history(), ...(moved ? { order: pages } : {}) };
  }

  /** Whether `pages` are in another order than the one known. */
  #moved(pages: readonly PageInfo[]): boolean {
    return pages.length !== this.#order.size || pages.some((page, i) => this.#order.get(page.id) !== i);
  }

  #editable(): void {
    if (this.#diverged) throw new Error(DIVERGED);
  }

  #serial<T>(call: () => Promise<T>): Promise<T> {
    const run = this.#queue.then(call);
    this.#queue = run.catch(() => undefined);
    return run;
  }
}

/** The engine's pages as the viewer reads them. */
export function pagesOf(outcome: { readonly pages: readonly EditedPage[] }): PageInfo[] {
  return outcome.pages.map((page, index) => ({
    index,
    id: page.id,
    width: page.width,
    height: page.height,
    rotation: page.rotate,
  }));
}

/** The pages of `pages` with these IDs, in page order. */
function listed(pages: readonly PageInfo[], ids: readonly string[]): PageInfo[] {
  const wanted = new Set(ids);
  return pages.filter((page) => wanted.has(page.id));
}

/** Whether `error` is the engine's refusal of a document whose edits went apart from its pages. */
function isDiverged(error: unknown): boolean {
  return (
    typeof error === "object" &&
    error !== null &&
    (error as { code?: unknown }).code === "edit-refused" &&
    (error as { message?: unknown }).message === "diverged"
  );
}
