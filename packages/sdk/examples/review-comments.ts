// Answer a review: read the comments on a marked-up report, reply to a question and mark a
// highlight done.
// The comments come back as threads, each with its replies and the status set on it, and a
// highlight's thread carries the text it marks. A reply and a status are annotations of their
// own, so the review keeps its history.
import { Document, type Op } from "@laminapdf/sdk";

/** Who answers the review. */
const ME = "Grace Hopper";

/**
 * The document with a reply to every note that asks a question, and every highlight marked
 * done, saved incrementally. `at` is when the review was answered, as a PDF date. Needs
 * `init({ extract: true })`.
 */
export function reviewComments(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const ops: Op[] = [];
  for (const thread of doc.commentThreads()) {
    const { id, subtype, contents } = thread.comment;
    if (subtype === "Text" && contents?.endsWith("?")) {
      const reply = { parent: id, contents: "June's figures are in the appendix.", author: ME, created: at };
      ops.push({ op: "addReply", args: reply });
    } else if (subtype === "Highlight" && thread.review !== "Completed") {
      // `quoted` is the text the highlight marks, for a list of what was checked.
      ops.push({ op: "setReviewState", args: { target: id, state: "completed", author: ME, created: at } });
    }
  }
  // Every answer is one undo step.
  doc.apply(ops);
  return doc.save({ mode: "incremental" });
}
