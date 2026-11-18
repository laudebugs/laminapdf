---
"@laminapdf/viewer": minor
---

`placeBox(box, points, size, rotation)` places a page layer's own element over a page as the viewer's layers place theirs. You size the element in page points, and one CSS transform scales and turns it to the page's size and rotation, so what a layer puts in points stays over the page at every zoom.
