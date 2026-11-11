import { describe, expect, it } from "vitest";
import { clearRedo, historyOf, markSaved, newSession, record, removeStep, stepBack, stepForward } from "./session.js";

/** A session with `n` steps done, labelled "1", "2", …. */
function withSteps(n: number, depth?: number) {
  const session = newSession(depth);
  for (let i = 1; i <= n; i++) record(session, String(i), [i - 1], i);
  return session;
}

const labels = (session: ReturnType<typeof newSession>) => session.steps.map((s) => s.label);

describe("the edit session's history", () => {
  it("starts with nothing to undo and no unsaved change", () => {
    expect(historyOf(newSession(), false)).toEqual({ steps: [], position: 0, dropped: 0, unsaved: 0, diverged: false });
  });

  it("counts each step as an unsaved change", () => {
    expect(historyOf(withSteps(3), false).unsaved).toBe(3);
  });

  it("is clean again once every step is undone", () => {
    const session = withSteps(2);
    stepBack(session, "applied");
    stepBack(session, "applied");
    expect([session.position, historyOf(session, false).unsaved]).toEqual([0, 0]);
  });

  it("keeps undone steps for redo until a new step clears them", () => {
    const session = withSteps(3);
    stepBack(session, "applied");
    const kept = labels(session);
    record(session, "4", [0], 4);
    expect([kept, labels(session)]).toEqual([
      ["1", "2", "3"],
      ["1", "2", "4"],
    ]);
  });

  it("redoes the oldest undone step", () => {
    const session = withSteps(3);
    stepBack(session, "applied");
    stepBack(session, "applied");
    expect(stepForward(session, "applied").step.label).toBe("2");
  });

  it("drops the oldest step past the depth", () => {
    const session = withSteps(4, 3);
    expect([labels(session), session.dropped]).toEqual([["2", "3", "4"], 1]);
  });

  it("keeps 500 steps by default and drops the 501st oldest", () => {
    const session = withSteps(501);
    expect([session.steps.length, session.steps[0]?.label, session.dropped]).toEqual([500, "2", 1]);
  });

  it("counts a document whose saved state was dropped as unsaved even with every kept step undone", () => {
    const session = withSteps(4, 3);
    for (let i = 0; i < 3; i++) stepBack(session, "applied");
    expect(historyOf(session, false).unsaved).toBe(1);
  });

  it("measures the distance from the last save", () => {
    const session = withSteps(2);
    markSaved(session);
    record(session, "3", [0], 3);
    stepBack(session, "applied");
    stepBack(session, "applied");
    expect(historyOf(session, false).unsaved).toBe(1);
  });

  it("loses the saved state when a new step clears the steps that led to it", () => {
    const session = withSteps(2);
    markSaved(session);
    stepBack(session, "applied");
    clearRedo(session);
    expect(session.saved).toBeUndefined();
  });

  it("marks a step partly undone", () => {
    const session = withSteps(1);
    expect(stepBack(session, "partial").step.partial).toBe(true);
  });

  it("clears the partial mark when a later undo restores everything", () => {
    const session = withSteps(1);
    stepBack(session, "partial");
    stepForward(session, "applied");
    expect(session.steps[0]?.partial).toBeUndefined();
  });

  it("removes a step whose undo restored nothing, and stays dirty", () => {
    const session = withSteps(2);
    stepBack(session, "skipped");
    expect([labels(session), session.position, historyOf(session, false).unsaved]).toEqual([["1"], 1, 1]);
  });

  it("removes a step whose redo restored nothing, staying where it was", () => {
    const session = withSteps(2);
    stepBack(session, "applied");
    stepForward(session, "skipped");
    expect([labels(session), session.position]).toEqual([["1"], 1]);
  });

  it("shifts the saved position past a removed step", () => {
    const session = withSteps(3);
    markSaved(session);
    removeStep(session, 0, "undo");
    expect(session.saved).toBe(2);
  });

  it("refuses to undo with nothing done", () => {
    expect(() => stepBack(newSession(), "applied")).toThrow();
  });
});
