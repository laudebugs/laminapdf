---
"@laminapdf/viewer": minor
---

`placeBox(box, points, size, rotation)` positions an element you add to a page layer the same way the viewer positions its own layers. You size `box` in page points (`points` is the page's size in points), and one CSS transform scales it to the page's CSS size (`size`) and turns it by the view's `rotation`, so it stays over the page at every zoom.
