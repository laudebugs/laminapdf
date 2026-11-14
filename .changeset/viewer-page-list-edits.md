---
"@laminapdf/viewer": minor
---

The viewer shows edits that insert, delete or move pages. Before, the document worker refused any edit that changed the page count. Now an edit, undo or redo that changes the page list returns the pages in their new order (`EditResult.order`). The view, its thumbnails, find, Back and Forward and the edit history follow each page to its new place. A moved page keeps the pictures already drawn of it, and a deleted page's place goes to its nearest surviving page. Each history step lists its pages' IDs (`EditStep.pageIds`), and names them where they are when the history is read. The viewer fires a `reorder` event when the page list changes.
