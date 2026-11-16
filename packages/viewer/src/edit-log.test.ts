import { describe, expect, it } from "vitest";
import { EditLog } from "./edit-log.js";
import type { EditHistory, EditResult, EditStep } from "./protocol.js";

const step = (id: number): EditStep => ({ id, label: `step ${id}`, pages: [0], pageIds: ["p1.0"], at: id });

function result(ids: readonly number[], position = ids.length, dropped = 0): EditResult {
  const history: EditHistory = { steps: ids.map(step), position, dropped, unsaved: position, diverged: false };
  return { pages: [], history };
}

const rotate = (n: number) => [{ op: "rotatePage", args: { page: `p${n}.0`, rotate: 90 } }] as never;

describe("the page's edit log", () => {
  it("is empty until an edit", () => {
    expect(new EditLog().empty).toBe(true);
  });

  it("replays every step and undoes the undone ones", () => {
    const log = new EditLog();
    log.take(result([1]), { ops: rotate(1), label: "one" });
    log.take(result([1, 2]), { ops: rotate(2), label: "two" });
    log.take(result([1, 2], 1));
    expect(log.replay()).toEqual({
      steps: [
        { ops: rotate(1), label: "step 1", at: 1 },
        { ops: rotate(2), label: "step 2", at: 2 },
      ],
      undone: 1,
      sources: [],
    });
  });

  it("forgets steps a new edit cleared", () => {
    const log = new EditLog();
    log.take(result([1]), { ops: rotate(1), label: "one" });
    log.take(result([1], 0));
    log.take(result([2]), { ops: rotate(2), label: "two" });
    expect(log.replay()?.steps.map((s) => s.label)).toEqual(["step 2"]);
  });

  it("keeps the steps the depth limit dropped, which are still in the document", () => {
    const log = new EditLog();
    log.take(result([1]), { ops: rotate(1), label: "one" });
    log.take(result([2], 1, 1), { ops: rotate(2), label: "two" });
    expect(log.replay()?.steps.length).toBe(2);
  });

  it("takes the new worker's IDs after a replay", () => {
    const log = new EditLog();
    log.take(result([7]), { ops: rotate(1), label: "one" });
    log.replayed(result([1]));
    log.take(result([1, 2]), { ops: rotate(2), label: "two" });
    expect(log.replay()?.steps.length).toBe(2);
  });

  it("won't replay once an undo restored only part of a step, or none of it", () => {
    for (const outcome of ["partial", "skipped"] as const) {
      const log = new EditLog();
      log.take(result([1]), { ops: rotate(1), label: "one" });
      const undone = result([1], 0);
      log.take({ ...undone, reverted: [{ step: undone.history.steps[0] as EditStep, way: "undo", outcome }] });
      expect(log.replay()).toBeUndefined();
    }
  });

  it("replays again after a reset", () => {
    const log = new EditLog();
    const undone = result([1], 0);
    log.take({ ...undone, reverted: [{ step: undone.history.steps[0] as EditStep, way: "undo", outcome: "partial" }] });
    log.reset();
    log.take(result([2]), { ops: rotate(2), label: "two" });
    expect(log.replay()?.steps.length).toBe(1);
  });

  it("starts over when reset", () => {
    const log = new EditLog();
    log.take(result([1]), { ops: rotate(1), label: "one" });
    log.reset();
    expect([log.empty, log.history.steps]).toEqual([true, []]);
  });

  describe("after a save", () => {
    it("replays only the steps since the save, onto the saved file", () => {
      const log = new EditLog();
      log.take(result([1]), { ops: rotate(1), label: "one" });
      log.saved(result([1]).history);
      log.take(result([1, 2]), { ops: rotate(2), label: "two" });
      log.take(result([1, 2, 3]), { ops: rotate(3), label: "three" });
      log.take(result([1, 2, 3], 2));
      expect(log.replay()).toEqual({
        steps: [
          { ops: rotate(2), label: "step 2", at: 2 },
          { ops: rotate(3), label: "step 3", at: 3 },
        ],
        undone: 1,
        sources: [],
      });
    });

    it("has nothing to replay when nothing changed since", () => {
      const log = new EditLog();
      log.take(result([1]), { ops: rotate(1), label: "one" });
      log.saved(result([1]).history);
      expect(log.empty).toBe(true);
    });

    it("won't replay once an undo went back past the save", () => {
      const log = new EditLog();
      log.take(result([1]), { ops: rotate(1), label: "one" });
      log.saved(result([1]).history);
      log.take(result([1], 0));
      log.take(result([2]), { ops: rotate(2), label: "two" });
      expect(log.replay()).toBeUndefined();
    });
  });

  describe("with source documents", () => {
    const source = "a".repeat(64);
    const insert = [{ op: "insertPagesFrom", args: { source, pages: ["p3.0"], at: "end" } }] as never;
    const merge = [{ op: "mergeDocuments", args: { sources: [{ source }], at: "end" } }] as never;
    const bytes = new Uint8Array([37, 80, 68, 70]);

    it("adds each source a step names before the steps", () => {
      for (const ops of [insert, merge]) {
        const log = new EditLog();
        log.addedSource(source, bytes, false);
        log.take(result([1]), { ops, label: "insert" });
        expect(log.replay()?.sources).toEqual([bytes]);
      }
    });

    it("lets a source go once no step names it", () => {
      const log = new EditLog();
      log.addedSource(source, bytes, false);
      log.take(result([1]), { ops: insert, label: "insert" });
      log.take(result([1], 0));
      log.take(result([2]), { ops: rotate(2), label: "rotate" });
      expect(log.replay()).toEqual({ steps: [{ ops: rotate(2), label: "step 2", at: 2 }], undone: 0, sources: [] });
    });

    it("won't replay a step from an encrypted source, whose password it never keeps", () => {
      const log = new EditLog();
      log.addedSource(source, bytes, true);
      log.take(result([1]), { ops: insert, label: "insert" });
      expect(log.replay()).toBeUndefined();
    });

    it("won't replay a step from a source it never saw added", () => {
      const log = new EditLog();
      log.take(result([1]), { ops: insert, label: "insert" });
      expect(log.replay()).toBeUndefined();
    });
  });
});
