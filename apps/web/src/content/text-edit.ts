// Edit text (Paper A04): the runs of text a page shows, which are picked, which is being typed
// into, and the one op each edit makes. The content controller owns the pointer and the layer;
// this owns the picks and the typing. An edit renames the runs it touches, so each change reads
// the page again and the pick follows its runs by where they stand. A run is a line, or a segment
// of one in a single font and size: the core edits and styles whole runs, so there is no change of
// part of one. Apart from the DOM, so Node tests it.

import type { ContentTextRun, Op } from "@laminapdf/sdk";
import { laterMessage } from "@laminapdf/ui/editing";
import type { Box, Point } from "./geometry.ts";
import {
  cycleRuns,
  editTextOp,
  followRuns,
  hitRun,
  isEditable,
  lastFirst,
  type PlanHost,
  planText,
  runBox,
  runsWithin,
  type StyleChange,
  setTextStyleOp,
  styleOf,
  styleStep,
  type TextPlan,
} from "./text-runs.ts";

/** What Edit text needs of Edit mode. */
export interface TextEditHost extends PlanHost {
  /** The runs of text page `page` shows. */
  readonly textRuns: (page: number) => Promise<ContentTextRun[]>;
  readonly pageId: (page: number) => string;
  /** Applies `ops` as one step named `label` and reads `pages` again; whether it was made. */
  readonly apply: (ops: readonly Op[], label: string, pages: readonly number[]) => Promise<boolean>;
  /** Runs `work` once every change before it is made and its page read again. */
  readonly enqueue: <T>(work: () => Promise<T>) => Promise<T>;
  /** As `apply`, for work already running in the queue. */
  readonly applyNow: (ops: readonly Op[], label: string, pages: readonly number[]) => Promise<boolean>;
  /** The pick, the hover or the typing changed: draw it, tell the inspector. */
  readonly changed: () => void;
  /** A pick was made: take the selection off added objects and objects the file had. */
  readonly clearOthers: () => void;
  /** Gives the focus back to the page once typing ends. */
  readonly focusPage: () => void;
  /** Says something short that isn't part of the inspector. */
  readonly say: (message: string) => void;
}

/** A pick: one or more runs of one page. */
export interface TextPick {
  readonly page: number;
  readonly runs: readonly ContentTextRun[];
}

/** The line being typed into. */
export interface TextTyping {
  readonly page: number;
  readonly run: ContentTextRun;
  /** What the run said when typing began. */
  readonly original: string;
  readonly text: string;
  /** Bumped when the text is set from outside the box (Revert), so the box shows it. */
  readonly version: number;
  /** What the engine said of `text`, once planned. */
  readonly plan: TextPlan | undefined;
  /** The text whose plan has been planned. */
  readonly planned: string | undefined;
  /** The text a substitution was shown for: Enter saves it only once its notice was shown. */
  readonly shown: string | undefined;
}

/** How long typing is quiet before what it says is planned, in milliseconds. */
const PLAN_AFTER_MS = 200;

/** Edit text over one open document. */
export class TextEditor {
  readonly #host: TextEditHost;
  readonly #runs = new Map<number, readonly ContentTextRun[]>();
  readonly #old = new Set<number>();
  #pick: TextPick | undefined;
  #hover: { readonly page: number; readonly run: ContentTextRun } | undefined;
  #typing: TextTyping | undefined;
  #committing: Promise<boolean> | undefined;
  #timer: ReturnType<typeof setTimeout> | undefined;
  #version = 0;

  constructor(host: TextEditHost) {
    this.#host = host;
  }

  get pick(): TextPick | undefined {
    return this.#pick;
  }

  get typing(): TextTyping | undefined {
    return this.#typing;
  }

  /** The runs of `page`, once read. */
  on(page: number): readonly ContentTextRun[] {
    return this.#runs.get(page) ?? [];
  }

  /** Whether `page` has been read, and not changed since. */
  has(page: number): boolean {
    return this.#runs.has(page) && !this.#old.has(page);
  }

  /** Marks what was read of `pages` as old. */
  invalidate(pages: readonly number[]): void {
    for (const page of pages) this.#old.add(page);
  }

  /** Reads `pages` again; the pick keeps its runs, found by where they stand. */
  async read(pages: readonly number[]): Promise<void> {
    await Promise.all(pages.map((page) => this.#readPage(page)));
    const pick = this.#pick;
    if (pick !== undefined && pages.includes(pick.page)) {
      const now = followRuns(pick.runs, this.on(pick.page));
      this.#pick = now.length === 0 ? undefined : { page: pick.page, runs: now };
      const typing = this.#typing;
      if (typing !== undefined) {
        const found = followRuns([typing.run], this.on(typing.page))[0];
        this.#typing = found === undefined ? undefined : { ...typing, run: found };
      }
    }
    this.#host.changed();
  }

  /** Reads `page` unless it is read already. */
  async ensure(page: number): Promise<void> {
    if (!this.has(page)) await this.read([page]);
  }

  async #readPage(page: number): Promise<void> {
    const runs = await this.#host.textRuns(page).catch((error: unknown) => {
      console.error("textRuns failed", error);
      return undefined;
    });
    if (runs === undefined) return;
    this.#old.delete(page);
    this.#runs.set(page, runs);
  }

  /** Forgets every pick and the typing; used when Edit mode is left. */
  reset(): void {
    this.#pick = undefined;
    this.#hover = undefined;
    this.#typing = undefined;
    this.#runs.clear();
    this.#old.clear();
    if (this.#timer !== undefined) clearTimeout(this.#timer);
    this.#timer = undefined;
  }

  // Picking.

  /** Clears the pick and drops typing that isn't committed; true if there was a pick. */
  clear(): boolean {
    const had = this.#pick !== undefined || this.#typing !== undefined;
    this.#pick = undefined;
    this.#typing = undefined;
    if (had) this.#host.changed();
    return had;
  }

  /** Picks `runs` of `page` (or clears the pick). */
  choose(page: number, runs: readonly ContentTextRun[]): void {
    if (runs.length === 0) {
      this.clear();
      return;
    }
    this.#pick = { page, runs };
    this.#typing = undefined;
    this.#host.clearOthers();
    this.#host.changed();
  }

  /** The pointer moved over `page`: outlines the run a press would pick. */
  hoverAt(page: number, point: Point): void {
    const found = hitRun(this.on(page), point);
    if (found === this.#hover?.run) return;
    this.#hover = found === undefined ? undefined : { page, run: found };
    this.#host.changed();
  }

  /** Takes the hover outline away. */
  unhover(): void {
    if (this.#hover === undefined) return;
    this.#hover = undefined;
    this.#host.changed();
  }

  /**
   * A press with the Edit text tool: Shift adds the run under it to the pick (or takes it out);
   * otherwise the run is picked and typed into. A press over nothing clears the pick.
   */
  async press(page: number, point: Point, shift: boolean): Promise<void> {
    await this.commit("away");
    await this.ensure(page);
    const hit = hitRun(this.on(page), point);
    this.#hover = undefined;
    if (hit === undefined) {
      if (!shift) this.clear();
      return;
    }
    if (shift) {
      const held = this.#pick?.page === page ? this.#pick.runs : [];
      const has = held.some((r) => r === hit);
      this.choose(page, has ? held.filter((r) => r !== hit) : [...held, hit]);
      return;
    }
    this.choose(page, [hit]);
    if (isEditable(hit)) this.edit(page, hit);
  }

  /** A double-click with Select: types into the run under `point`; whether there was one. */
  async open(page: number, point: Point): Promise<boolean> {
    await this.ensure(page);
    const hit = hitRun(this.on(page), point);
    if (hit === undefined) return false;
    this.choose(page, [hit]);
    if (isEditable(hit)) this.edit(page, hit);
    return true;
  }

  /** Picks every run inside `box` that can be edited and types into the first. */
  async editWithin(page: number, box: Box): Promise<boolean> {
    await this.ensure(page);
    const runs = runsWithin(this.on(page), box).filter(isEditable);
    const [first] = runs;
    if (first === undefined) {
      this.#host.say(laterMessage("content.text.unavailable"));
      return false;
    }
    this.choose(page, runs);
    this.edit(page, first);
    return true;
  }

  /** The runs inside `box` that can't be edited: why Edit text on a block says no. */
  refusedWithin(page: number, box: Box): ContentTextRun | undefined {
    return runsWithin(this.on(page), box).find((r) => !isEditable(r));
  }

  /** Steps the pick to the next (or previous) run that can be edited. */
  cycle(page: number, step: 1 | -1): void {
    const current = this.#pick?.page === page ? this.#pick.runs[0] : undefined;
    const next = cycleRuns(this.on(page), current, step);
    if (next !== undefined) this.choose(page, [next]);
  }

  // Typing.

  /** Types into `run`, which is picked alone. */
  edit(page: number, run: ContentTextRun): void {
    if (!isEditable(run)) return;
    this.#version += 1;
    this.#pick = { page, runs: [run] };
    this.#typing = {
      page,
      run,
      original: run.text,
      text: run.text,
      version: this.#version,
      plan: undefined,
      planned: undefined,
      shown: undefined,
    };
    this.#host.changed();
  }

  /** Types into the one picked run, if it can be edited. */
  editPicked(): void {
    const pick = this.#pick;
    const [only] = pick?.runs ?? [];
    if (pick !== undefined && pick.runs.length === 1 && only !== undefined) this.edit(pick.page, only);
  }

  /** What the box says now: the plan follows it a moment later. */
  typed(text: string): void {
    const typing = this.#typing;
    if (typing === undefined || typing.text === text) return;
    this.#typing = { ...typing, text };
    if (this.#timer !== undefined) clearTimeout(this.#timer);
    this.#timer = setTimeout(() => void this.planNow(), PLAN_AFTER_MS);
  }

  /** Plans the text as it stands, so the notice says what saving it would do. */
  async planNow(): Promise<TextPlan | undefined> {
    const typing = this.#typing;
    if (typing === undefined || typing.text === typing.original) return undefined;
    const text = typing.text;
    const plan = await planText(this.#host, typing.page, typing.run, text);
    const now = this.#typing;
    if (now === undefined || now.text !== text) return plan;
    this.#typing = { ...now, plan, planned: text, shown: plan.kind === "substituted" ? text : now.shown };
    this.#host.changed();
    return plan;
  }

  /** Puts the run's original text back into the box. */
  revert(): void {
    const typing = this.#typing;
    if (typing === undefined) return;
    this.#version += 1;
    this.#typing = {
      ...typing,
      text: typing.original,
      version: this.#version,
      plan: undefined,
      planned: undefined,
      shown: undefined,
    };
    this.#host.changed();
  }

  /** Ends typing without an edit. */
  cancel(): void {
    if (this.#typing === undefined) return;
    if (this.#timer !== undefined) clearTimeout(this.#timer);
    this.#typing = undefined;
    this.#host.changed();
    this.#host.focusPage();
  }

  /**
   * Ends typing, making the edit if the text changed: one `editText` op and one step. `why` says
   * how it ended. A substitution is shown before it is saved, so Enter, Tab and Done stay open
   * the first time, with the notice; leaving the box saves it and says so. Whether typing ended.
   */
  commit(why: "enter" | "away" | "done"): Promise<boolean> {
    this.#committing ??= this.#commit(why).finally(() => {
      this.#committing = undefined;
    });
    return this.#committing;
  }

  async #commit(why: "enter" | "away" | "done"): Promise<boolean> {
    const typing = this.#typing;
    if (typing === undefined) return true;
    if (this.#timer !== undefined) clearTimeout(this.#timer);
    const text = typing.text;
    if (text === typing.original) {
      this.cancel();
      return true;
    }
    const plan = await planText(this.#host, typing.page, typing.run, text);
    const now = this.#typing;
    if (now === undefined || now.text !== text) return false;
    const shown = now.shown;
    this.#typing = { ...now, plan, planned: text, shown };
    if (plan.kind === "refused" || plan.kind === "missing") {
      this.#host.changed();
      return false;
    }
    if (plan.kind === "substituted" && shown !== text && why !== "away") {
      this.#typing = { ...this.#typing, shown: text };
      this.#host.changed();
      return false;
    }
    const fonts = plan.kind === "substituted" ? plan.fonts : undefined;
    const label = laterMessage(text === "" ? "content.text.step.delete" : "content.text.step.edit");
    const op = editTextOp(this.#host.pageId(typing.page), typing.run.run, text, fonts);
    const made = await this.#host.apply([op], label, [typing.page]);
    if (!made) return false;
    if (plan.kind === "substituted") this.#host.say(substitutedMessage(plan));
    this.#typing = undefined;
    this.#host.changed();
    this.#host.focusPage();
    return true;
  }

  /** Saves the line being typed and types into the next (or previous) one that can be edited. */
  async step(page: number, step: 1 | -1): Promise<void> {
    if (!(await this.commit("enter"))) return;
    const current = this.#pick?.page === page ? this.#pick.runs[0] : undefined;
    const next = cycleRuns(this.on(page), current, step);
    if (next === undefined) return;
    this.choose(page, [next]);
    this.edit(page, next);
  }

  // Changes to the picked runs.

  /** Removes the picked runs' text as one step: each run's text becomes empty. */
  async deleteText(): Promise<boolean> {
    const pick = this.#pick;
    if (pick === undefined) return false;
    const runs = pick.runs.filter(isEditable);
    if (runs.length === 0) return false;
    this.#typing = undefined;
    const ops = lastFirst(runs).map((run) => editTextOp(this.#host.pageId(pick.page), run.run, ""));
    const made = await this.#host.apply(ops, laterMessage("content.text.step.delete"), [pick.page]);
    if (made) this.clear();
    return made;
  }

  /** Restyles the picked runs as one step. Typing that is still open is saved first. */
  async style(change: StyleChange): Promise<boolean> {
    if (this.#typing !== undefined && !(await this.commit("done"))) return false;
    return this.#host.enqueue(async () => {
      const pick = this.#pick;
      if (pick === undefined) return false;
      const runs = pick.runs.filter(isEditable);
      if (runs.length === 0) return false;
      const op = setTextStyleOp(
        this.#host.pageId(pick.page),
        runs.map((r) => r.run),
        styleOf(change),
      );
      return this.#host.applyNow([op], laterMessage(`content.text.step.${styleStep(change)}`), [pick.page]);
    });
  }

  // What the layer and the inspector show.

  /** The boxes outlined as editable text on `page` (every run that can be edited). */
  outlines(page: number): readonly Box[] {
    return this.on(page)
      .filter(isEditable)
      .map((r) => runBox(r));
  }

  /** The hovered run. */
  get hover(): { readonly page: number; readonly run: ContentTextRun } | undefined {
    return this.#hover;
  }

  /** What a screen reader hears of the pick. */
  announcement(): string {
    const pick = this.#pick;
    const [first] = pick?.runs ?? [];
    if (pick === undefined || first === undefined) return "";
    if (pick.runs.length > 1)
      return laterMessage("content.text.selectedMany", { count: pick.runs.length, page: pick.page + 1 });
    return laterMessage("content.text.selected", {
      text: first.text.length > 60 ? `${first.text.slice(0, 60)}…` : first.text,
      page: pick.page + 1,
      line: first.line + 1,
    });
  }
}

/** The words that name a substitution already made: "“č” drawn in Helvetica". */
export function substitutedMessage(plan: Extract<TextPlan, { kind: "substituted" }>): string {
  const characters = [...new Set(plan.substitutions.flatMap((s) => [...s.text].filter((c) => c.trim() !== "")))]
    .map((c) => `“${c}”`)
    .join(" ");
  const font = plan.substitutions[0]?.font ?? "";
  return laterMessage("content.text.substituted", { characters, font });
}
