// The snapshot behind the home page's roadmap and dashboard, progress.json in this folder, which
// `cargo xtask progress` writes (tools/xtask/src/progress.rs documents each field). Pages read it
// at build time through readProgress, which fails the build on a file this type doesn't describe.

import { count, date, list, nullable, oneOf, read, record, text } from "./check.js";

/** Todo counts: in progress means in `ongoing/` or claimed by an open pull request. */
interface Counts {
  readonly todos: number;
  readonly done: number;
  readonly inProgress: number;
  readonly ready: number;
}

/** A todo by ID and title, as the board shows it (the title may hold Markdown code spans). */
interface TodoRef {
  readonly id: string;
  readonly title: string;
}

/** One section of the todo board, such as `B` ("Library — Round B"). */
interface Round extends Counts {
  readonly letter: string;
  readonly heading: string;
}

/** One milestone of the roadmap, with the state of the todos its rounds name. */
interface Milestone {
  readonly id: string;
  readonly name: string;
  readonly rounds: string;
  readonly ships: string;
  readonly state: "done" | "under-way" | "not-started";
}

/** progress.json. */
export interface Progress {
  readonly version: 1;
  /** The day the snapshot was written, `YYYY-MM-DD`. */
  readonly date: string;
  readonly totals: Counts;
  readonly rounds: readonly Round[];
  readonly milestones: readonly Milestone[];
  readonly inProgress: {
    /** Where the claims came from: the open pull requests, or the last snapshot when they couldn't be read. */
    readonly source: "pull-requests" | "previous-snapshot" | "none";
    /** The day the open pull requests were read, or `null` if they never were. */
    readonly checked: string | null;
    readonly todos: readonly TodoRef[];
  };
  /** The board's "Can start now". */
  readonly nextUp: readonly TodoRef[];
  /** How many pull requests were merged, and the latest titles. */
  readonly merged: { readonly count: number; readonly recent: readonly string[] };
}

const COUNTS = { todos: count, done: count, inProgress: count, ready: count };
const TODO = record({ id: text, title: text });

const PROGRESS = record({
  version: oneOf(1),
  date,
  totals: record(COUNTS),
  rounds: list(record({ letter: text, heading: text, ...COUNTS })),
  milestones: list(
    record({ id: text, name: text, rounds: text, ships: text, state: oneOf("done", "under-way", "not-started") }),
  ),
  inProgress: record({
    source: oneOf("pull-requests", "previous-snapshot", "none"),
    checked: nullable(date),
    todos: list(TODO),
  }),
  nextUp: list(TODO),
  merged: record({ count, recent: list(text) }),
});

/** `value` (progress.json, parsed) as a {@link Progress}; throws, naming the field, when it isn't one. */
export function readProgress(value: unknown): Progress {
  return read<Progress>("progress.json", PROGRESS, value);
}
