---
"@laminapdf/viewer": minor
---

The viewer shows edits that insert, delete or move pages. Before, the document worker refused any edit that changed the page count. Now an edit, undo or redo that changes the page list returns the pages in their new order (`EditResult.order`). The view and its thumbnails follow each page to its new place, as do find, Back and Forward, and the edit history. A moved page keeps its pictures, and what the viewer kept for a deleted page, such as history entries or find's current match, moves to its nearest surviving page. Each history step lists its pages' IDs (`EditStep.pageIds`), so the history names a step's pages by where they are now. The viewer fires a `reorder` event when the page list changes.
