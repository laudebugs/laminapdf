// doc.commentThreads(), doc.deleteAnnotations() and the addReply and setReviewState ops over
// fixtures/annotations/review.pdf (written by tools/fixtures/annotations.py), as
// crates/laminapdf/tests/comments.rs reads them.
import { Document, init, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(async () => {
  await init();
  await loadExtract();
});

const open = () => Document.open(read("fixtures/annotations/review.pdf"));

describe("commentThreads", () => {
  it("gives each comment with its replies, states and quoted text", () => {
    using doc = open();
    const thread = doc.commentThreads().find((t) => t.comment.id === "a10.0");
    expect([
      thread?.quoted,
      thread?.review,
      thread?.marked,
      thread?.replies.map((r) => [r.page, r.annotation.contents]),
    ]).toEqual([
      "Quoted words",
      "Accepted",
      "Marked",
      [
        [0, "Agreed"],
        [0, "Thanks"],
        [1, "From page 2"],
      ],
    ]);
  });

  it("flags broken threads and keeps them", () => {
    using doc = open();
    const broken = doc
      .commentThreads()
      .filter((t) => t.broken)
      .map((t) => [t.comment.id, t.broken]);
    expect(broken).toEqual([
      ["a17.0", "missing-parent"],
      ["a18.0", "cycle"],
    ]);
  });

  it("filters by the comment's page", () => {
    using doc = open();
    expect([doc.commentThreads([0]).length, doc.commentThreads([1]).length]).toEqual([5, 0]);
  });
});

describe("replies, states and deleting", () => {
  it("adds a reply and a state that read back in the thread", () => {
    using doc = open();
    doc.apply([
      { op: "addReply", args: { parent: "a16.0", contents: "On it", author: "Ada" } },
      { op: "setReviewState", args: { target: "a16.0", state: "cancelled", author: "Ada" } },
    ]);
    const thread = doc.commentThreads().find((t) => t.comment.id === "a16.0");
    expect([thread?.replies.at(-1)?.annotation.contents, thread?.review]).toEqual(["On it", "Cancelled"]);
  });

  it("deletes a comment and its reply as one undo step", () => {
    using doc = open();
    doc.deleteAnnotations(["a11.0", "a10.0"]);
    const after = doc.commentThreads().length;
    doc.undo();
    expect([after, doc.commentThreads().length]).toEqual([4, 5]);
  });
});
