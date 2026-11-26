// The desktop's edit session over a stand-in engine: a document whose pages each have a rotation,
// with an undo and redo stack as the engine's model keeps them.

import { describe, expect, it } from "vitest";
import { DesktopEdits, type EditOutcome, type EditRequest } from "./desktop-edits.ts";
import fixture from "./edit-outcome.fixture.json" with { type: "json" };

/** A stand-in engine: three pages, `rotatePage` ops, and undo/redo stacks of rotations. */
function engine(options: { readonly failRedoAfter?: number } = {}) {
  const rotations = [0, 0, 0];
  /** The file on disk: the rotations a restarted engine opens. */
  const file = [0, 0, 0];
  const done: { page: number; before: number; after: number }[] = [];
  const undone: typeof done = [];
  const calls: EditRequest["kind"][] = [];
  let redos = 0;
  const report = (touched: string[], reversal: EditOutcome["reversal"], changed = true): EditOutcome => ({
    changed,
    reversal,
    touched,
    pageOrder: null,
    pages: rotations.map((rotate, i) => ({ id: `p${i + 1}.0`, width: 612, height: 792, rotate })),
    createdOutlineItems: [],
    createdAnnotations: [],
    outlineRepaired: false,
  });
  /** Applies the log's one `rotatePage` op. */
  const apply = (log: string): EditOutcome => {
    const { ops } = JSON.parse(log) as { ops: { args: { page: string; rotate: number } }[] };
    const args = ops[0]?.args;
    if (args === undefined) throw new Error("no op");
    const page = Number(args.page.slice(1, -2)) - 1;
    const before = rotations[page] ?? 0;
    if (before === args.rotate) return report([], null, false);
    rotations[page] = args.rotate;
    done.push({ page, before, after: args.rotate });
    undone.length = 0;
    return report([`p${page + 1}.0`], null);
  };
  /** Undoes or redoes the newest entry of its stack. */
  const revert = (way: "undo" | "redo"): EditOutcome => {
    if (way === "redo" && options.failRedoAfter !== undefined && redos++ >= options.failRedoAfter)
      throw new Error("render-failed");
    const [from, to] = way === "undo" ? [done, undone] : [undone, done];
    const entry = from.pop();
    if (entry === undefined) return report([], null, false);
    rotations[entry.page] = way === "undo" ? entry.before : entry.after;
    to.push(entry);
    return report([`p${entry.page + 1}.0`], "applied");
  };
  const run = (request: EditRequest): Promise<EditOutcome> => {
    calls.push(request.kind);
    try {
      return Promise.resolve(request.kind === "apply" ? apply(request.log) : revert(request.kind));
    } catch (e) {
      return Promise.reject(e);
    }
  };
  const pages = rotations.map((rotation, index) => ({
    index,
    id: `p${index + 1}.0`,
    width: 612,
    height: 792,
    rotation,
  }));
  /** A restarted engine: the file as it is on disk, with no history. */
  const restart = () => {
    rotations.splice(0, rotations.length, ...file);
    done.length = 0;
    undone.length = 0;
  };
  /** A save: the file on disk is the document now, and the engine keeps its history. */
  const save = () => {
    file.splice(0, file.length, ...rotations);
  };
  return { run, pages, rotations, calls, restart, save };
}

const turn = (page: number, rotate = 90) => [{ op: "rotatePage", args: { page: `p${page}.0`, rotate } }] as never;

describe("the desktop edit session", () => {
  it("records an edit as a step with its pages, and undo and redo move through it", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    const applied = await edits.apply(turn(2), "Rotate page 2");
    const undone = await edits.step(-1);
    const redone = await edits.step(1);
    expect({
      pages: applied.pages.map((page) => [page.index, page.rotation]),
      steps: redone.history.steps.map((step) => [step.label, step.pages]),
      undone: [undone.history.position, undone.history.unsaved, undone.pages[0]?.rotation],
      redone: [redone.history.position, redone.history.unsaved, redone.reverted?.[0]?.way],
    }).toEqual({
      pages: [[1, 90]],
      steps: [["Rotate page 2", [1]]],
      undone: [0, 0, 0],
      redone: [1, 1, "redo"],
    });
  });

  it("records no step for an edit that changed nothing", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    const result = await edits.apply(turn(1, 0), "Rotate page 1");
    expect([result.pages, result.history.steps]).toEqual([[], []]);
  });

  it("goes to a step with one call per step, and drops the steps undone on a new edit", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    await edits.apply(turn(1), "one");
    await edits.apply(turn(2), "two");
    await edits.apply(turn(3), "three");
    const back = await edits.goTo(1);
    const next = await edits.apply(turn(1, 180), "four");
    expect({
      back: [back.history.position, back.pages.map((page) => page.id)],
      rotations: fake.rotations,
      steps: next.history.steps.map((step) => step.label),
    }).toEqual({ back: [1, ["p2.0", "p3.0"]], rotations: [180, 0, 0], steps: ["one", "four"] });
  });

  it("takes back the moves already made when one fails half way, and keeps the history as it was", async () => {
    const fake = engine({ failRedoAfter: 1 });
    const edits = new DesktopEdits(fake.run, fake.pages);
    await edits.apply(turn(1), "one");
    await edits.apply(turn(2), "two");
    await edits.goTo(0);
    await expect(edits.goTo(2)).rejects.toThrow("render-failed");
    expect([edits.history().position, fake.rotations, fake.calls.slice(-3)]).toEqual([
      0,
      [0, 0, 0],
      ["redo", "redo", "undo"],
    ]);
  });

  it("runs calls in order: an undo sent with an edit waits for it", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    const both = Promise.all([edits.apply(turn(1), "one"), edits.step(-1)]);
    const [, undone] = await both;
    expect([undone.history.position, fake.rotations[0]]).toEqual([0, 0]);
  });

  it("gives a restarted engine the sources a step took pages from before it makes the step again", async () => {
    const fake = engine();
    const given: (readonly number[])[] = [];
    const edits = new DesktopEdits(fake.run, fake.pages, (sources) => {
      given.push(sources);
      return Promise.resolve();
    });
    await edits.apply(turn(1), "from a file", [7]);
    await edits.apply(turn(2), "plain");
    fake.restart();
    await edits.replay();
    expect(given).toEqual([[7], []]);
  });

  it("marks the document diverged when a source can't be given again", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages, () => Promise.reject(new Error("gone")));
    await edits.apply(turn(1), "from a file", [7]);
    fake.restart();
    await expect(edits.replay()).rejects.toThrow("couldn't be undone");
    expect(edits.history().diverged).toBe(true);
  });

  it("makes the edits again in a restarted engine, then undoes the ones that were undone", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    await edits.apply(turn(1), "one");
    await edits.apply(turn(2), "two");
    await edits.step(-1);
    fake.restart();
    const replayed = await edits.replay();
    const redone = await edits.step(1);
    expect([replayed?.history.position, replayed?.pages.length, redone.history.position, fake.rotations]).toEqual([
      1,
      3,
      2,
      [90, 90, 0],
    ]);
  });

  it("counts no edit unsaved once saved, and the next edit as the one unsaved", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    await edits.apply(turn(1), "one");
    await edits.apply(turn(2), "two");
    fake.save();
    const saved = await edits.saved(false);
    await edits.apply(turn(3), "three");
    expect([saved.unsaved, saved.steps.length, edits.history().unsaved]).toEqual([0, 2, 1]);
  });

  it("starts the history again after a save that couldn't keep it", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    await edits.apply(turn(1), "one");
    fake.save();
    const saved = await edits.saved(true);
    expect([saved.unsaved, saved.steps.length, saved.position]).toEqual([0, 0, 0]);
  });

  it("after a save, makes only the later edits again in a restarted engine, and the saved ones leave the history", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    await edits.apply(turn(1), "one");
    fake.save();
    await edits.saved(false);
    await edits.apply(turn(2), "two");
    fake.restart();
    const replayed = await edits.replay();
    const history = edits.history();
    expect([fake.rotations, history.steps.map((step) => step.label), history.unsaved, replayed?.pages.length]).toEqual([
      [90, 90, 0],
      ["two"],
      1,
      3,
    ]);
  });

  it("after a save with nothing since, makes nothing again in a restarted engine and keeps no saved step", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    await edits.apply(turn(1), "one");
    fake.save();
    await edits.saved(false);
    fake.restart();
    const replayed = await edits.replay();
    expect([replayed, edits.history().steps.length, edits.history().unsaved, fake.calls.at(-1)]).toEqual([
      undefined,
      0,
      0,
      "apply",
    ]);
  });

  it("marks the document diverged when a saved edit was undone before the engine restarted", async () => {
    const fake = engine();
    const edits = new DesktopEdits(fake.run, fake.pages);
    await edits.apply(turn(1), "one");
    fake.save();
    await edits.saved(false);
    await edits.step(-1);
    fake.restart();
    await expect(edits.replay()).rejects.toThrow("couldn't be undone");
    expect(edits.history().diverged).toBe(true);
  });

  it("drops the steps undone when an edit fails after changing the document, as the engine's redo stack does", async () => {
    const fake = engine();
    let failWith: string | undefined;
    const run = (request: EditRequest) =>
      failWith !== undefined && request.kind === "apply"
        ? Promise.reject(Object.assign(new Error(failWith), { code: failWith }))
        : fake.run(request);
    const edits = new DesktopEdits(run, fake.pages);
    await edits.apply(turn(1), "one");
    await edits.step(-1);
    failWith = "edit-refused";
    await expect(edits.apply(turn(2), "two")).rejects.toThrow();
    const afterRefusal = edits.history().steps.length;
    failWith = "render-failed";
    await expect(edits.apply(turn(2), "two")).rejects.toThrow();
    expect([afterRefusal, edits.history().steps.length]).toEqual([1, 0]);
  });

  it("reads the outcome the main process sends (src-tauri's test sends the same file)", () => {
    const sample = {
      changed: true,
      reversal: "partial",
      touched: [],
      pageOrder: null,
      pages: [],
      createdOutlineItems: [],
      createdAnnotations: [],
      outlineRepaired: false,
    } satisfies Record<keyof EditOutcome, unknown>;
    expect([Object.keys(fixture).sort(), ["applied", "partial", "skipped"].includes(fixture.reversal)]).toEqual([
      Object.keys(sample).sort(),
      true,
    ]);
  });
});
