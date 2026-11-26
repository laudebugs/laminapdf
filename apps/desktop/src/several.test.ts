// The question about several files opened together, over a stand-in for the main process's answer.

import { ToastQueue } from "@laminapdf/ui";
import { describe, expect, it } from "vitest";
import { SeveralAsk } from "./several.tsx";

const ASKED = { request: 5, names: ["Annual report.pdf", "Notes.pdf"] };

/** A question whose answers are recorded, and which the main process answers `done` (or fails). */
function question(done: boolean | Error = true) {
  const answers: [number, string, string | undefined][] = [];
  const ask = new SeveralAsk((request, answer, suggested) => {
    answers.push([request, answer, suggested]);
    return done instanceof Error ? Promise.reject(done) : Promise.resolve(done);
  }, new ToastQueue());
  return { ask, answers };
}

describe("the question about several files", () => {
  it("shows the files' names until it is answered", () => {
    const { ask } = question();
    ask.ask(ASKED);
    expect(ask.getState()).toMatchObject({ request: 5, names: ASKED.names, busy: false, failed: false });
  });

  it("opens each file, and the question goes", async () => {
    const { ask, answers } = question();
    ask.ask(ASKED);
    await ask.choose("each");
    expect([answers, ask.getState()]).toEqual([[[5, "each", undefined]], undefined]);
  });

  it("merges, suggesting a name from the first file, and the question goes", async () => {
    const { ask, answers } = question();
    ask.ask(ASKED);
    await ask.choose("merge");
    expect([answers, ask.getState()]).toEqual([[[5, "merge", "Annual report and others.pdf"]], undefined]);
  });

  it("stays open when the person cancels the save dialog of the merge", async () => {
    const { ask } = question(false);
    ask.ask(ASKED);
    await ask.choose("merge");
    expect(ask.getState()).toMatchObject({ busy: false, failed: false });
  });

  it("says the merge failed and lets the person open each file instead", async () => {
    const { ask, answers } = question(new Error("encrypted"));
    ask.ask(ASKED);
    await ask.choose("merge");
    const failed = ask.getState();
    await ask.choose("each");
    expect([failed?.failed, answers.map((a) => a[1]), ask.getState()]).toEqual([true, ["merge", "each"], undefined]);
  });

  it("opens none when the person cancels", async () => {
    const { ask, answers } = question();
    ask.ask(ASKED);
    await ask.choose("cancel");
    expect([answers[0]?.[1], ask.getState()]).toEqual(["cancel", undefined]);
  });
});
