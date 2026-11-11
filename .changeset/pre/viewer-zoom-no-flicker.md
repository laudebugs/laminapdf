---
"@laminapdf/viewer": patch
---

The page no longer flickers while zooming. Each pause in a pinch or ctrl+wheel zoom repainted the page for its new zoom with the blurry low-resolution pass, throwing away the sharp pixels it showed; the repaint now keeps those pixels, scaled, until the sharp tiles for the new zoom replace them.
