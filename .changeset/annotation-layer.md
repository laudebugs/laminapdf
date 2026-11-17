---
"@laminapdf/viewer": patch
---

The viewer can now edit annotations on the page. `new AnnotationLayer(viewer, client)`, added with `viewer.addLayer`, puts a button over each annotation, in reading order. The document Web Worker reads each page's annotations for it, as `client.pageAnnotations(index)`.

An editing mode can select annotations with the pointer or Tab and move them by dragging or with the arrow keys (1 pt, or 10 pt with Shift). The selection shows its outline and handles. Each move is one edit and one undo step, with an `updateAnnot` for each annotation moved. Escape cancels a drag.

A page never shows an annotation twice. While a drag lasts, `viewer.hideAnnotations(page, places)` draws the page without it, and a preview follows the pointer. On release, the preview disappears in the same frame that the page's new pixels appear; `viewer.whenDrawn(page)` says when that is. Other callers can open the same gesture with `layer.startGesture(page, hide)`.
