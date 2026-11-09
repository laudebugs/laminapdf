---
"@laminapdf/viewer": patch
---

Zoomed out far enough for more than three pages to show, every page now turns sharp. Before, the first three did and the rest stayed blurry for good, because the canvas pool's rounding to whole pixels counted as a cap and every page waited for one of three detail canvases.
