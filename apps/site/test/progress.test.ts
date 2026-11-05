// The committed snapshot against its type: the home page reads it at build time, so a file that
// `cargo xtask progress` and readProgress disagree about fails here first.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { readProgress } from "../src/data/progress.js";

const committed: unknown = JSON.parse(readFileSync(new URL("../src/data/progress.json", import.meta.url), "utf8"));

/** The fields the tests break, loosely typed so a test can set a wrong value. */
interface Loose {
  version: unknown;
  date: unknown;
  merged?: unknown;
  nextUp: [{ title: unknown }];
  rounds: [{ done: unknown }];
}

/** The committed file with `change` applied to a copy. */
function changed(change: (file: Loose) => void): unknown {
  const copy = structuredClone(committed) as Loose;
  change(copy);
  return copy;
}

describe("progress.json", () => {
  it("is what readProgress describes", () => {
    const progress = readProgress(committed);
    expect(progress.version).toBe(1);
    expect(progress.totals.todos).toBe(progress.totals.done + progress.totals.inProgress + progress.totals.ready);
    expect(progress.rounds.reduce((sum, round) => sum + round.todos, 0)).toBe(progress.totals.todos);
    expect(progress.milestones.map((milestone) => milestone.id)[0]).toBe("M0");
  });

  it("names no pull request and no person", () => {
    const text = readFileSync(new URL("../src/data/progress.json", import.meta.url), "utf8");
    expect(text).not.toMatch(/#\d|github\.com|Co-Authored|Claude-Session/);
  });

  it("fails on a file the type doesn't describe, naming the field", () => {
    expect(() => readProgress(changed((f) => (f.version = 2)))).toThrow("the file.version is not 1");
    expect(() => readProgress(changed((f) => (f.date = "5 November")))).toThrow("the file.date is not a date");
    expect(() => readProgress(changed((f) => delete f.merged))).toThrow("the file.merged is not an object");
    expect(() => readProgress(changed((f) => (f.nextUp[0].title = 3)))).toThrow("the file.nextUp[0].title");
    expect(() => readProgress(changed((f) => (f.rounds[0].done = -1)))).toThrow(
      "the file.rounds[0].done is not a count",
    );
  });
});
