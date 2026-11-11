import { ToastQueue } from "@laminapdf/ui";
import type { EditFailure, EditSessionState, RevertedStep } from "@laminapdf/viewer";
import { describe, expect, it } from "vitest";
import { failureText, showEditToasts, type ToastedSession } from "./edit-toasts.ts";

const STEP = { id: 1, label: "Rotate page 5", pages: [4], at: 0 };

/** A session whose history and events a test drives. */
class FakeSession implements ToastedSession {
  #state = {
    history: { steps: [STEP], position: 1, dropped: 0, unsaved: 1, diverged: false },
  } as unknown as EditSessionState;
  readonly #listeners = new Set<() => void>();
  readonly #reverted = new Set<(r: readonly RevertedStep[]) => void>();
  readonly #failed = new Set<(f: EditFailure) => void>();
  readonly calls: string[] = [];
  getState = () => this.#state;
  subscribe = (l: () => void) => {
    this.#listeners.add(l);
    return () => this.#listeners.delete(l);
  };
  onReverted = (l: (r: readonly RevertedStep[]) => void) => {
    this.#reverted.add(l);
    return () => this.#reverted.delete(l);
  };
  onFailed = (l: (f: EditFailure) => void) => {
    this.#failed.add(l);
    return () => this.#failed.delete(l);
  };
  undo = async () => {
    this.calls.push("undo");
    return undefined;
  };
  redo = async () => {
    this.calls.push("redo");
    return undefined;
  };
  /** The history moves to `position`, then the session reports the step it moved over. */
  move(position: number, way: "undo" | "redo", outcome: RevertedStep["outcome"] = "applied"): void {
    this.#state = { ...this.#state, history: { ...this.#state.history, position } };
    for (const l of [...this.#listeners]) l();
    for (const l of [...this.#reverted]) l([{ step: STEP, way, outcome }]);
  }
  /** A new edit: the history changes, with no undo or redo to report. */
  edit(): void {
    this.#state = { ...this.#state, history: { ...this.#state.history, position: 1 } };
    for (const l of [...this.#listeners]) l();
  }
  fail(failure: EditFailure): void {
    for (const l of [...this.#failed]) l(failure);
  }
}

const texts = (toasts: ToastQueue) => toasts.entries().map((e) => e.options.message);

describe("the edit session's toasts", () => {
  it("say what an undo did and offer Redo", async () => {
    const session = new FakeSession();
    const toasts = new ToastQueue();
    showEditToasts(session, toasts);
    session.move(0, "undo");
    toasts.entries()[0]?.options.action?.onPress();
    expect([texts(toasts), session.calls]).toEqual([["Undid “Rotate page 5”"], ["redo"]]);
  });

  it("show one edit toast at a time: the next replaces it", () => {
    const session = new FakeSession();
    const toasts = new ToastQueue();
    showEditToasts(session, toasts);
    session.move(0, "undo");
    session.move(1, "redo");
    expect(texts(toasts)).toEqual(["Redid “Rotate page 5”"]);
  });

  it("go when the history moves another way, so their action never acts on another step", () => {
    const session = new FakeSession();
    const toasts = new ToastQueue();
    showEditToasts(session, toasts);
    session.move(0, "undo");
    session.edit();
    expect(texts(toasts)).toEqual([]);
  });

  it("go with their document", () => {
    const session = new FakeSession();
    const toasts = new ToastQueue();
    const stop = showEditToasts(session, toasts);
    session.move(0, "undo");
    stop();
    expect(texts(toasts)).toEqual([]);
  });

  it("say which call failed", () => {
    const session = new FakeSession();
    const toasts = new ToastQueue();
    showEditToasts(session, toasts);
    session.fail({ call: "apply", label: "Rotate page 5", error: new Error("no") });
    expect(texts(toasts)).toEqual(["Couldn't rotate page 5. Try again."]);
  });

  it("name every failed call", () => {
    expect(["undo", "redo", "goTo"].map((call) => failureText({ call: call as EditFailure["call"] }))).toEqual([
      "Couldn't undo. Try again.",
      "Couldn't redo. Try again.",
      "Couldn't go to that step. Try again.",
    ]);
  });
});
