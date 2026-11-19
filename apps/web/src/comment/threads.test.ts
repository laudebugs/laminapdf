import type { CommentThread } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { counts, matches, NO_FILTER, shownThreads, type ThreadView, threadViews } from "./threads.ts";

/** An annotation as `commentThreads` gives it, with only what the panel reads. */
function annotation(id: string, subtype: string, extra: Record<string, unknown> = {}) {
  return { id, subtype, color: [], contents: undefined, author: undefined, modified: undefined, ...extra };
}

/** A thread on `page` of comment `id`. */
function thread(id: string, page: number, subtype: string, extra: Record<string, unknown> = {}, comment = {}) {
  return {
    page,
    comment: annotation(id, subtype, comment),
    replies: [],
    group: [],
    states: [],
    ...extra,
  } as unknown as CommentThread;
}

const THREADS: ThreadView[] = threadViews([
  thread(
    "a1",
    0,
    "Highlight",
    { quoted: "Repairs backlog" },
    { author: "Ada", contents: "Check the total", modified: "D:20261101" },
  ),
  thread(
    "a2",
    2,
    "Square",
    {
      review: "Accepted",
      states: [
        {
          page: 2,
          annotation: annotation("s1", "Text", { author: "Ben", state: { model: "Review", state: "Accepted" } }),
        },
      ],
      replies: [{ page: 2, annotation: annotation("r1", "Text", { author: "Ben", contents: "Moved the west team" }) }],
    },
    { author: "Ben", contents: "", modified: "D:20261103" },
  ),
  thread(
    "a3",
    1,
    "Text",
    { review: "Completed", broken: "missing-parent" },
    {
      author: "Ada",
      contents: "Sign here",
      modified: "D:20261102",
    },
  ),
]);

describe("threadViews", () => {
  it("reads each thread's status, who set it, its kind and whether it is broken", () => {
    expect(THREADS.map((t) => [t.id, t.status, t.statusBy, t.kind, t.broken])).toEqual([
      ["a1", "open", undefined, "markup", false],
      ["a2", "accepted", "Ben", "drawings", false],
      ["a3", "done", undefined, "notes", true],
    ]);
  });
});

describe("matches", () => {
  it("finds a word in the note, a reply, the quoted text or the author, ignoring case", () => {
    const [first, second] = THREADS as [ThreadView, ThreadView];
    expect([
      matches(first, "TOTAL"),
      matches(first, "backlog"),
      matches(second, "west team"),
      matches(second, "ben"),
      matches(first, "west"),
    ]).toEqual([true, true, true, true, false]);
  });
});

describe("shownThreads", () => {
  it("keeps the statuses, authors, kinds and pages a filter names, with the search", () => {
    const ids = (filter: Partial<typeof NO_FILTER>, query = "") =>
      shownThreads(THREADS, { ...NO_FILTER, ...filter }, query, "page").map((t) => t.id);
    expect([
      ids({ statuses: new Set(["open", "done"]) }),
      ids({ authors: new Set(["Ada"]), kinds: new Set(["notes"]) }),
      ids({ pages: [1, 2] }),
      ids({}, "sign"),
    ]).toEqual([["a1", "a3"], ["a3"], ["a2", "a3"], ["a3"]]);
  });

  it("sorts newest first, or by author, keeping document order for ties", () => {
    const order = (sort: "newest" | "author") => shownThreads(THREADS, NO_FILTER, "", sort).map((t) => t.id);
    expect([order("newest"), order("author")]).toEqual([
      ["a2", "a3", "a1"],
      ["a1", "a3", "a2"],
    ]);
  });
});

describe("counts", () => {
  it("counts the threads of each status, author and kind for the filter menu", () => {
    const c = counts(THREADS);
    expect([c.statuses.get("open"), c.authors.get("Ada"), c.kinds.get("markup")]).toEqual([1, 2, 1]);
  });
});
