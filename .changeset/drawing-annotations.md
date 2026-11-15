---
"@laminapdf/wasm": patch
"@laminapdf/sdk": patch
---

Draw on a page: `doc.inkAt`, `doc.shapeAt`, `doc.lineAt` and `doc.polyAt` give the edit that adds freehand ink, a square or circle, a line or arrow, or a polygon or polyline, from points in page space. Lamina PDF draws each one into the file, with solid, dashed or cloudy borders, a fill colour, every line ending the PDF standard defines, and leader lines. Ink is smoothed through its points, and a stroke can carry one width per point for a pressure-sensitive pen. `updateAnnot` erases ink strokes and adds new ones. In Rust, the same edits are `Document::ink_at`, `shape_at`, `line_at` and `poly_at`.
