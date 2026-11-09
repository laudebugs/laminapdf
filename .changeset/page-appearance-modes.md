---
"@laminapdf/render": minor
"@laminapdf/render-hayro": minor
"@laminapdf/render-pdfium": minor
"@laminapdf/viewer": minor
"@laminapdf/wasm": minor
---

Pages can be drawn in Sepia, Dark or High contrast instead of their own colours. Photos and annotations keep theirs, and so does text under a highlight.

- **Render request:** `RenderRequest.appearance` is `"original"` (the default), `"sepia"`, `"dark"` or `{ kind: "high-contrast", ink, paper }`, for both backends.
- **Colour maps:** `recolourRgb` and `recolourTile` in `@laminapdf/render` give the same bytes as the Rust crate's.
- **Viewer:** `setAppearance()` redraws the pages in view in the new colours, keeping the scroll position. The thumbnails follow.
