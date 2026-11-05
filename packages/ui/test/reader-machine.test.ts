import { describe, expect, it } from "vitest";
import { EMPTY, type ReaderEvent, type ReaderState, transition } from "../src/state/reader-machine.ts";
import { fakeFile } from "./fakes.ts";

const file = fakeFile();
const other = fakeFile("other.pdf");
const error = { code: "invalid-header", message: "not a PDF" };

const STATES: Record<ReaderState["phase"], ReaderState> = {
  empty: EMPTY,
  opening: { phase: "opening", file },
  "needs-password": { phase: "needs-password", file, wrong: false },
  open: { phase: "open", file, pageCount: 3, mode: "select" },
  error: { phase: "error", file, error },
};

const EVENTS: Record<ReaderEvent["type"], ReaderEvent> = {
  open: { type: "open", file: other },
  opened: { type: "opened", pageCount: 7 },
  "password-required": { type: "password-required" },
  "submit-password": { type: "submit-password" },
  "wrong-password": { type: "wrong-password" },
  failed: { type: "failed", error },
  cancel: { type: "cancel" },
  close: { type: "close" },
  "set-mode": { type: "set-mode", mode: "select" },
};

/** Every transition that changes the state, by state then event; the rest leave it as it is. */
const CHANGES: Partial<Record<ReaderState["phase"], Partial<Record<ReaderEvent["type"], ReaderState>>>> = {
  opening: {
    opened: { phase: "open", file, pageCount: 7, mode: "select" },
    "password-required": { phase: "needs-password", file, wrong: false },
    "wrong-password": { phase: "needs-password", file, wrong: true },
    failed: { phase: "error", file, error },
    cancel: EMPTY,
  },
  "needs-password": {
    "submit-password": { phase: "opening", file },
    cancel: EMPTY,
  },
  open: { close: EMPTY },
  error: { close: EMPTY },
};

describe("the reader machine", () => {
  const phases = Object.keys(STATES) as ReaderState["phase"][];
  const types = Object.keys(EVENTS).filter((type) => type !== "open") as ReaderEvent["type"][];

  it.each(phases)("opens a new document from %s, replacing what was there", (phase) => {
    expect(transition(STATES[phase], EVENTS.open)).toEqual({ phase: "opening", file: other });
  });

  for (const phase of phases) {
    for (const type of types) {
      const expected = CHANGES[phase]?.[type];
      if (expected === undefined) {
        it(`leaves ${phase} as it is on ${type}`, () => {
          expect(transition(STATES[phase], EVENTS[type])).toBe(STATES[phase]);
        });
      } else {
        it(`goes from ${phase} to ${expected.phase} on ${type}`, () => {
          expect(transition(STATES[phase], EVENTS[type])).toEqual(expected);
        });
      }
    }
  }

  it("keeps the document through a wrong password and the retry", () => {
    let state = transition(EMPTY, { type: "open", file });
    for (const event of [EVENTS["password-required"], EVENTS["submit-password"], EVENTS["wrong-password"]]) {
      state = transition(state, event);
    }
    expect(state).toEqual({ phase: "needs-password", file, wrong: true });
    state = transition(transition(state, EVENTS["submit-password"]), EVENTS.opened);
    expect(state).toEqual({ phase: "open", file, pageCount: 7, mode: "select" });
  });
});
