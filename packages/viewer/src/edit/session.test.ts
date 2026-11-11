import { describe, expect, it } from "vitest";
import { type EditHistory, type EditResult, EMPTY_HISTORY, type OpenedDocument } from "../protocol.js";
import { EditSession, type EditSessionHost } from "./session.js";

/** A worker's session in memory: steps by label, and calls in the order they ran. */
class FakeWorker implements EditSessionHost {
  labels: string[] = [];
  position = 0;
  readonly calls: string[] = [];
  /** The next call's outcome for undo and redo. */
  outcome: "applied" | "partial" | "skipped" = "applied";
  fail = false;

  history(): EditHistory {
    const steps = this.labels.map((label, i) => ({ id: i + 1, label, pages: [0], at: 0 }));
    return { ...EMPTY_HISTORY, steps, position: this.position, unsaved: this.position };
  }

  async applyEdit(_ops: unknown, { label }: { label: string }): Promise<EditResult> {
    this.calls.push(`apply ${label}`);
    await Promise.resolve();
    if (this.fail) throw new Error("the edit failed");
    this.labels = [...this.labels.slice(0, this.position), label];
    this.position += 1;
    return { pages: [], history: this.history() };
  }

  async undoEdit(): Promise<EditResult> {
    return this.goToStep(this.position - 1);
  }

  async redoEdit(): Promise<EditResult> {
    return this.goToStep(this.position + 1);
  }

  async goToStep(position: number): Promise<EditResult> {
    this.calls.push(`go ${position}`);
    await Promise.resolve();
    const way = position < this.position ? "undo" : "redo";
    const step = this.history().steps[way === "undo" ? this.position - 1 : this.position];
    this.position = position;
    const reverted = step === undefined ? [] : [{ step, way, outcome: this.outcome } as const];
    return { pages: [], history: this.history(), reverted };
  }

  async editHistory(): Promise<EditHistory> {
    return this.history();
  }
}

const DOCUMENT = {
  id: 1,
  pageCount: 1,
  pages: [],
  needsPassword: false,
  encrypted: false,
} as unknown as OpenedDocument;

function open(worker = new FakeWorker()): { session: EditSession; worker: FakeWorker } {
  const session = new EditSession(worker, () => 0);
  session.reset(DOCUMENT);
  return { session, worker };
}

describe("the edit session store", () => {
  it("is blocked with no document open", () => {
    expect(new EditSession(new FakeWorker()).getState().blocked).toBe("no-document");
  });

  it("is blocked for an encrypted document", () => {
    const session = new EditSession(new FakeWorker());
    session.reset({ ...DOCUMENT, encrypted: true });
    expect(session.getState().blocked).toBe("encrypted");
  });

  it("refuses an edit while blocked", async () => {
    const session = new EditSession(new FakeWorker());
    await expect(session.apply([], "Rotate page 1")).rejects.toThrow(/no-document/);
  });

  it("takes the history an edit answers", async () => {
    const { session } = open();
    await session.apply([], "Rotate page 1");
    expect(session.getState().history.steps.map((s) => s.label)).toEqual(["Rotate page 1"]);
  });

  it("runs calls one after another in the order they were made", async () => {
    const { session, worker } = open();
    await Promise.all([session.apply([], "a"), session.apply([], "b"), session.undo(), session.undo()]);
    expect(worker.calls).toEqual(["apply a", "apply b", "go 1", "go 0"]);
  });

  it("is busy while a call runs and idle after", async () => {
    const { session } = open();
    const running = session.apply([], "a");
    const busy = session.getState().busy;
    await running;
    expect([busy, session.getState().busy]).toEqual([true, false]);
  });

  it("redoes a step an undo just queued, however quickly the keys come", async () => {
    const { session, worker } = open();
    await session.apply([], "a");
    await Promise.all([session.undo(), session.redo()]);
    expect([worker.calls, session.getState().history.position]).toEqual([["apply a", "go 0", "go 1"], 1]);
  });

  it("does nothing on undo with nothing done", async () => {
    const { session, worker } = open();
    await session.undo();
    expect(worker.calls).toEqual([]);
  });

  it("does nothing on redo with nothing undone", async () => {
    const { session, worker } = open();
    await session.apply([], "a");
    await session.redo();
    expect(worker.calls).toEqual(["apply a"]);
  });

  it("keeps a partial undo's notice until the next edit", async () => {
    const { session, worker } = open();
    await session.apply([], "a");
    worker.outcome = "partial";
    await session.undo();
    const notice = session.getState().notice?.outcome;
    await session.apply([], "b");
    expect([notice, session.getState().notice]).toEqual(["partial", undefined]);
  });

  it("tells its listeners what an undo did", async () => {
    const { session } = open();
    const heard: string[] = [];
    session.onReverted((steps) => heard.push(...steps.map((s) => `${s.way} ${s.step.label}`)));
    await session.apply([], "a");
    await session.undo();
    expect(heard).toEqual(["undo a"]);
  });

  it("reads the worker's history again after a failed edit", async () => {
    const { session, worker } = open();
    await session.apply([], "a");
    await session.undo();
    worker.labels = [];
    worker.fail = true;
    await expect(session.apply([], "b")).rejects.toThrow();
    expect(session.getState().history.steps).toEqual([]);
  });

  it("tells its listeners which call failed, with an edit's label", async () => {
    const { session, worker } = open();
    const heard: string[] = [];
    session.onFailed(({ call, label }) => heard.push(`${call} ${label ?? ""}`.trim()));
    worker.fail = true;
    await expect(session.apply([], "Rotate page 1")).rejects.toThrow();
    expect(heard).toEqual(["apply Rotate page 1"]);
  });

  it("blocks editing once the worker says the document diverged", () => {
    const { session } = open();
    session.follow({ ...EMPTY_HISTORY, diverged: true });
    expect(session.getState().blocked).toBe("diverged");
  });

  it("drops an answer for a document since replaced", async () => {
    const { session } = open();
    const running = session.apply([], "a");
    session.reset(DOCUMENT);
    await expect(running).rejects.toThrow(/closed/);
  });

  it("goes to a step in one call", async () => {
    const { session, worker } = open();
    await session.apply([], "a");
    await session.apply([], "b");
    await session.goTo(0);
    expect([worker.calls.at(-1), session.getState().history.position]).toEqual(["go 0", 0]);
  });
});
