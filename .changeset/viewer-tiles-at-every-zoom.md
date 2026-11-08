---
"@laminapdf/render": patch
"@laminapdf/viewer": patch
---

The viewer is sharp at every zoom level. It sized each page's canvas and sharp tiles from the page's CSS width times the pixel ratio, while the renderer sizes the page in 32-bit floats, so at most zoom levels the right-hand tile came out one pixel wider than the page. The renderer refused it, and the rest of the page stayed blurry. `@laminapdf/render` now exports `pagePixels`, the page's size in pixels at a scale as its planner computes it, and the viewer sizes canvases and tiles with it.
