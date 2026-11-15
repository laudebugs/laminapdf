---
"@laminapdf/wasm": patch
"@laminapdf/sdk": patch
---

Read a review's comments as threads, reply to them and set their status.

`doc.commentThreads(pages)` gives each comment with its replies in order, the annotations grouped with it, the review states set on it and the latest of each, and the text a highlight marks; a reply to a missing comment or a ring of replies comes back as a thread of its own marked `broken`. The `addReply` and `setReviewState` ops answer a comment or record its status as annotations of their own, as Acrobat does, so the thread keeps its history, and `doc.deleteAnnotations(ids)` deletes a selection in one undo step. The Rust crates have the same: `Document::comment_threads`, `delete_annotations`, `AddReply` and `SetReviewState`.
