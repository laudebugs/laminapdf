// The ops a Comments panel thread runs (Paper C04): a reply, a status, Mark as checked and
// deleting threads. Each reply and status is a reply annotation, as Acrobat writes them, so other
// apps show the thread (ISO 32000-2 §12.5.6.2, `/IRT`, `/StateModel`).

import type { Op } from "@laminapdf/sdk";
import type { StatusChoice } from "@laminapdf/ui/editing";
import { deleteAnnotOp, pdfNow } from "./ops.ts";

/** The review state `status` sets: "Done" is `Completed`, "None" clears the status. */
const REVIEW: Record<StatusChoice, "none" | "accepted" | "rejected" | "cancelled" | "completed"> = {
  none: "none",
  accepted: "accepted",
  rejected: "rejected",
  cancelled: "cancelled",
  done: "completed",
};

/** Who did it and when, as each op records it. */
function by(author: string | undefined, when: Date) {
  return { ...(author === undefined ? {} : { author }), created: pdfNow(when) };
}

/** A reply `text` to comment `parent`. */
export function replyOp(parent: string, text: string, author: string | undefined, when = new Date()): Op {
  return { op: "addReply", args: { parent, contents: text, ...by(author, when) } };
}

/** `status` set on each of `targets`. */
export function statusOps(
  targets: readonly string[],
  status: StatusChoice,
  author: string | undefined,
  when = new Date(),
): Op[] {
  return targets.map((target) => ({
    op: "setReviewState",
    args: { target, state: REVIEW[status], ...by(author, when) },
  }));
}

/** Mark as checked turned on or off on `target`. */
export function checkedOp(target: string, checked: boolean, author: string | undefined, when = new Date()): Op {
  return { op: "setReviewState", args: { target, state: checked ? "marked" : "unmarked", ...by(author, when) } };
}

/** `ids` deleted, each with its replies. */
export function deleteOps(ids: readonly string[]): Op[] {
  return ids.map(deleteAnnotOp);
}
