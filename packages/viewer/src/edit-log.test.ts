import { describe, expect, it } from "vitest";
import { EditLog } from "./edit-log.js";
import type { EditHistory, EditResult, EditStep } from "./protocol.js";

const step = (id: number): EditStep => ({ id, label: `step ${id}`, pages: [0], at: id });

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
});
