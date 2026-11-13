---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Organise a document's pages: insert blank pages, delete pages, move them, rotate them (to an angle, or turned from their own) and reverse their order, each as one edit that undo reverts. Pages keep their IDs while they move. Deleting a page points the bookmarks, links, named destinations and opening view that went to it at the nearest page left, with the whole page in view, or takes away their destination when you ask, and the edit lists each one. Structure elements whose content was only on deleted pages are removed, and so are the form fields whose widgets were only on them. Page labels stay with their pages. Deleting every page is refused with the new code `no-pages-left`, and a blank-page count or size out of range with `invalid-op`.

In Rust: `InsertBlankPages`, `DeletePages`, `MovePages`, `RotatePages` and `ReversePages`, applied with `Document::apply`; `Edited` gains `page_order` and `references`. `PageSize` moves to `laminapdf-model`, and `laminapdf-build` re-exports it, so its path through the builder and the façade is unchanged. Rotating the page that holds a signed signature's widget now makes the save a conflict, as deleting that page does.
