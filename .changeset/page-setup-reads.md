---
"@laminapdf/sdk": minor
"@laminapdf/viewer": minor
---

A document now reads back its page boxes and page labels in the form the edits take, so an app can show the current values and change them. `doc.pageBoxes(index)` returns a page's media, crop, bleed, trim and art boxes in user space, as `setPageBoxes` takes them, and says which of the last three the page sets itself. `doc.labelRanges()` returns the page labels as ranges, in the form `setPageLabels` takes, so they can be changed and written back. Both need the extract or text module; in Rust they are `Document::page_boxes` and `Document::label_ranges`.

The viewer's document worker serves both reads, as `client.pageBoxes(index)` and `client.labelRanges()`. An edit that sets the page labels, and its undo and redo, now gives every page its new label in `EditResult.relabelled`, and `client.document.pages` takes them. No page is drawn again, since labels draw nothing.
