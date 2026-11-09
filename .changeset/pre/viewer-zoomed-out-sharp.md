---
"@laminapdf/viewer": patch
---

When you zoom out far enough that more than three pages show, every page now turns sharp. Before, the first three did and the rest stayed blurry for good, because the canvas pool's rounding to whole pixels counted as a cap and every page waited for one of three detail canvases.
