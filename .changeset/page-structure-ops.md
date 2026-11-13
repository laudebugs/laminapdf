---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Organise a document's pages: insert blank pages, delete pages, move them, rotate them (to an angle, or by an angle from where each one stands) and reverse their order, each as one edit that undo reverts. Pages keep their IDs while they move. Deleting a page sends the bookmarks, links, named destinations and opening view that went to it to the nearest page left, showing the whole page. Ask instead and they lose their destination. Either way, the edit lists each one. The accessibility tags and form fields that only a deleted page used go with it; signature fields stay, so their signatures still count when you save. Page labels stay with their pages. Deleting every page is refused with the new code `no-pages-left`. A blank-page count or size out of range is refused with `invalid-op`.

In Rust: `InsertBlankPages`, `DeletePages`, `MovePages`, `RotatePages` and `ReversePages`, applied with `Document::apply`; `Edited` gains `page_order` and `references`. `PageSize` moves to `laminapdf-model`, and `laminapdf-build` re-exports it, so its path through the builder and the façade is unchanged. Rotating the page that holds a signed signature's widget now makes the save a conflict, as deleting that page does.
