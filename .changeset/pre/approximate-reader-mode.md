---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Reader mode for every PDF. When a file has no tags, or its tags fail reader mode's checks, `doc.reflow()` reads the page layout instead of refusing: `source` is `"approximate"` and `reason` says why the tags weren't used. A scanned page, or one whose column order extraction doubts, comes back as a `page-image` block with `why` (`"scanned"` or `"uncertain"`) and the page's text for searching. Tables and pictures are crops, and their captions are kept with them. Only a file with nothing to show has `available: false`, with the reason `"nothing"`.
