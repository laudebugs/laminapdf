---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Crop pages and set page boxes. `doc.cropPages(ids, { top, right, bottom, left })` crops pages by margins of the page as it shows, so one set crops pages of mixed sizes and rotations by the same visible amount. `doc.setPageBoxes(ids, { media, crop, bleed, trim, art })` writes or removes (`null`) any of a page's five boxes in its own coordinates; each must lie inside the media box. A crop only hides content: the edit's `croppedContentRemains` lists the pages whose hidden text is still in the file, where extraction finds it.

In Rust: the ops `CropPages` (with `CropMargins`) and `SetPageBoxes` (with `PageBoxes`), and `Applied::cropped_content_remains` and `Edited::cropped_content_remains`.
