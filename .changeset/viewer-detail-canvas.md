---
"@laminapdf/viewer": minor
---

The viewer keeps the page in view sharp at every zoom up to 800 %. Each page's canvas now holds at most 8 million pixels; a larger page keeps it as a backdrop, and a detail canvas over the part in view takes the sharp tiles at the full pixel ratio. The detail canvas's size follows the viewport rather than the zoom, so the canvases fit the same memory as before. `createViewer` takes a `detailPool` option, and the render stats count `detailCanvases`.
