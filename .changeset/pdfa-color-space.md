---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
---

On a file that claims PDF/A, `doc.plan()` now warns `pdfa-color-space` when an image or page you added paints in a colour space the output intent doesn't allow. A CMYK photo in a file with an RGB intent is one example. The warning names the pages (`pages`) and the image stamps (`annotations`). Greyscale images, and images or pages whose colours carry an ICC profile, fit any intent. Like the other PDF/A warnings, it never blocks a save.
