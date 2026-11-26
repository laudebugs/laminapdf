// The desktop's edit session: the history the web reader's document worker keeps
// (@laminapdf/viewer's edit-history), over the engine process, which makes each call. The engine
// applies an op log, an undo or a redo to its model, draws the pages from the edited file from
// then on, and takes the call back itself when that fails; this keeps the steps, their labels and
// the saved point, and turns the engine's report into the viewer's `EditResult`.

import type { EditHistory, EditResult, OpenedDocument, RevertedStep, ViewerEngine } from "@laminapdf/viewer";
import {
  clearHistory,
  clearRedo,
  historyOf,
  markSaved,
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
  /** The source documents (the main process's ids) each step's ops take pages from. */
  readonly #sources = new Map<number, readonly number[]>();
  /** Gives the engine the sources a step needs again, before the step is made again. */
  readonly #restore: (sources: readonly number[]) => Promise<void>;

  constructor(
    run: (request: EditRequest) => Promise<EditOutcome>,
    pages: readonly PageInfo[],
    restore: (sources: readonly number[]) => Promise<void> = () => Promise.resolve(),
  ) {
    this.#run = run;
    this.#restore = restore;
    this.#order = orderOf(pages.map((page) => page.id));
  }

  /** The history as the interface reads it. */
  history(): EditHistory {
    return historyOf(this.#session, this.#diverged, this.#order);
  }

  /**
   * Applies `ops` as one transaction, recorded as a step named `label`. `sources` are the
   * documents (the main process's ids) the ops take pages from: kept with the step, so a restarted
   * engine is given them again before the step is made again.
   */
  apply(ops: readonly Op[], label: string, sources: readonly number[] = []): Promise<EditResult> {
    return this.#serial(async () => {
      this.#editable();
      const log = JSON.stringify({ v: 1, ops });
      const outcome = await this.#applyLog(log);
      if (!outcome.changed) return { pages: [], history: this.history() };
      clearRedo(this.#session);
      const pages = pagesOf(outcome);
      const moves = ops.length > 0 && ops.every((op) => MOVE_OPS.has(op.op));
      const moved = this.#moved(pages);
      const step = record(this.#session, label, listed(pages, outcome.touched), Date.now());
      this.#keepLog(step.id, log);
      if (sources.length > 0) this.#sources.set(step.id, sources);
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
   * The document was saved, its file now holding every step done: the history marks that point
   * saved, or starts empty when the save couldn't keep it (`historyCleared`). Runs after the calls
   * before it.
   */
  saved(historyCleared: boolean): Promise<EditHistory> {
    return this.#serial(() => {
      if (historyCleared) {
        clearHistory(this.#session);
        this.#logs.clear();
        this.#sources.clear();
        this.#reorders.clear();
      } else {
        markSaved(this.#session);
      }
      return Promise.resolve(this.history());
    });
  }

  /**
   * Makes the edits again in an engine that restarted and opened the file as it is on disk: the
   * steps after the last save (the file holds those before), then undoes those that were undone.
   * The steps the file holds can't be undone in the new engine, so they leave the history.
   * Undefined when there was nothing to make again.
   *
   * @throws when a step fails, or the file isn't a point the history can go on from (the depth
   *   limit dropped steps it lacks, or steps it holds were undone): the document is then marked
   *   diverged, since it can't be what the history says.
   */
  replay(): Promise<EditResult | undefined> {
    return this.#serial(async () => {
      const { steps, position } = this.#session;
      let last: EditOutcome | undefined;
      let inFile = 0;
      try {
        inFile = this.#stepsInFile();
        if (steps.length === inFile && position === inFile) {
          this.#forgetFirst(inFile);
          return undefined;
        }
        for (const step of steps.slice(inFile)) {
          const log = this.#logs.get(step.id);
          if (log === undefined) throw new Error("a step has no op log");
          await this.#restore(this.#sources.get(step.id) ?? []);
          last = await this.#run({ kind: "apply", log });
        }
        for (let i = position; i < steps.length; i++) last = await this.#run({ kind: "undo" });
      } catch (e) {
        this.#diverged = true;
        throw new Error(DIVERGED, { cause: e });
      }
      this.#forgetFirst(inFile);
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

  /**
   * How many of the steps kept the file on disk holds: those up to the last save.
   * @throws when the file isn't a point the history can go on from.
   */
  #stepsInFile(): number {
    const { position, dropped, saved } = this.#session;
    const inFile = saved === undefined ? -1 : saved - dropped;
    if (inFile < 0) throw new Error("the file holds no point the history can go on from");
    if (position < inFile) throw new Error("edits the file holds were undone");
    return inFile;
  }

  /** Takes the oldest `count` steps, all done, out of the history: the file holds them. */
  #forgetFirst(count: number): void {
    const session = this.#session;
    for (const step of session.steps.splice(0, count)) {
      this.#logs.delete(step.id);
      this.#sources.delete(step.id);
      this.#reorders.delete(step.id);
    }
    session.dropped += count;
    session.position -= count;
  }

  /** Sends op log `log`. */
  async #applyLog(log: string): Promise<EditOutcome> {
    try {
      return await this.#call({ kind: "apply", log });
    } catch (e) {
      // Only an op's refusal changed nothing. Any other failure came after the change, which the
      // engine took back with an undo: that undo sits on its redo stack, so the steps undone here
      // can't be redone any more. Nothing redoes the undo, and the next edit clears it.
      if (codeOf(e) !== "edit-refused") clearRedo(this.#session);
      throw e;
    }
  }

  /** Keeps step `id`'s op log, and lets go of those of the steps the history no longer has. */
  #keepLog(id: number, log: string): void {
    this.#logs.set(id, log);
    const kept = new Set(this.#session.steps.map((step) => step.id));
    for (const step of this.#logs.keys()) {
      if (kept.has(step)) continue;
      this.#logs.delete(step);
      this.#sources.delete(step);
    }
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
  return codeOf(error) === "edit-diverged";
}

/** The main process's kind of a failed call (`DesktopEngineError.code`), if it has one. */
function codeOf(error: unknown): unknown {
  return typeof error === "object" && error !== null ? (error as { code?: unknown }).code : undefined;
}
