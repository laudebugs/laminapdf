---
title: "Answer a review"
description: "Read a PDF's comments as threads, reply to a question and mark a highlight done, with the review's history kept in the file."
version: "0.5.0"
url: "https://docs.laminapdf.com/recipes/review-comments/"
---

# Answer a review

Read a PDF's comments as threads, reply to a question and mark a highlight done, with the review's history kept in the file.

You'll answer the review left on a report: reply to the note asking a question, and mark the highlighted line as done. You need this for a review tool that lists a document's comments, or a script that closes every comment a checker has handled.

`commentThreads` gives each comment as a thread: the comment, its replies in order, the status set on it and, for a highlight, the text it marks. The `addReply` and `setReviewState` edits answer a comment and set its status. Here they answer the comments left by [Mark up a page](../../pages/recipes/mark-up.md):

**TypeScript**

`packages/sdk/examples/review-comments.ts`:

```ts
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
```

**Rust**

`crates/laminapdf/examples/review_comments.rs`:

````rust
//! Answer a review: read the comments on a marked-up report, reply to a question and mark a
//! highlight done.
//!
//! The comments come back as threads, each with its replies and the status set on it, and a
//! highlight's thread carries the text it marks. A reply and a status are annotations of their
//! own, so the review keeps its history.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example review_comments -- marked.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{AddReply, Document, Op, OpenOptions, PdfDate, ReviewState, SaveMode, SaveOptions, SetReviewState};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: review_comments marked.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the review was answered, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let answered = review(std::fs::read(input)?, at)?;
    std::fs::write(out.join("review_comments.pdf"), answered)?;
    Ok(())
}

/// Who answers the review.
const ME: &str = "Grace Hopper";

/// The document with a reply to every note that asks a question, and every highlight marked
/// done, saved incrementally.
pub(crate) fn review(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let mut ops = Vec::new();
    for thread in doc.comment_threads(None)? {
        let id = thread.comment.id.parse()?;
        let asks = thread.comment.contents.as_deref().is_some_and(|c| c.ends_with('?'));
        if thread.comment.subtype == "Text" && asks {
            let mut reply = AddReply::new(id, "June's figures are in the appendix.");
            reply.author = Some(ME.into());
            reply.created = Some(at);
            ops.push(Op::from(reply));
        } else if thread.comment.subtype == "Highlight" && thread.review.as_deref() != Some("Completed") {
            // `quoted` is the text the highlight marks, for a list of what was checked.
            let mut done = SetReviewState::new(id, ReviewState::Completed);
            done.author = Some(ME.into());
            done.created = Some(at);
            ops.push(Op::from(done));
        }
    }
    // Every answer is one undo step.
    doc.apply(ops)?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}
````

**Result:** [review_comments.pdf](https://docs.laminapdf.com/examples/pdf/review_comments.pdf)

The page looks the same as before: a reply and a status draw nothing on the page. Open the result in a reader with a comments panel, such as Acrobat, and the note has a reply from Grace Hopper, and the highlight is marked Completed.

## How it works

- A reply and a status are annotations of their own, each pointing at the comment it answers, as ISO 32000-2 describes and Acrobat writes them. Changing a status adds a new one rather than replacing the old, so the file keeps the review's history. A thread's `review` is its latest `Review` status, by date.
- Each reply and status sits over its comment and draws nothing, so no reader shows a second icon on the page. It still prints, which keeps a PDF/A file valid.
- A thread can hold replies to replies, and a reply on another page than its comment. Each part says which page it is on.
- `quoted` is the text under a highlight, underline or strike-out, read from the page. It needs the extract or text module.
- Some apps leave a reply to a comment that's gone, or replies that answer each other in a ring. Each comes back as a thread of its own marked `broken`, so nothing is lost.
- The author and dates come from your code, never from a clock inside Lamina PDF, so the same edit gives the same file wherever it runs.

## What to change

- **Another status:** `"accepted"`, `"rejected"`, `"cancelled"`, `"completed"` or `"none"` set the review status. `"marked"` and `"unmarked"` set a reviewer's check mark, which a thread gives as `marked`.
- **Some pages only:** pass page indexes, `doc.commentThreads([0, 1])`, for the threads whose comments are on those pages.
- **Delete comments:** `doc.deleteAnnotations(ids)` deletes a selection in one undo step, each comment with its replies, statuses and pop-up. Select a comment and one of its replies, and both go.
- **Edit a reply:** `updateAnnot` changes a reply's text like any other annotation's.
- **A signed file:** you can answer a review on a [certified](../../pages/reference/glossary.md#docmdp) file that allows comments, or one with only approval signatures. Lamina PDF appends the answers as a new revision, and the signatures stay valid.

Next: [add links](../../pages/recipes/add-links.md), another annotation you can add to a page.
