---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
---

On a file that claims PDF/A, `doc.plan()` now warns `pdfa-color-space` when an image or a page of another PDF you added paints in a colour space the file's output intent doesn't allow, such as a CMYK photo in a file with an sRGB intent. The warning names the pages (`pages`) and the image stamps (`annotations`). Gray images, and images or pages whose colours carry an ICC profile, fit any intent. Like the other PDF/A warnings, it never blocks a save.
