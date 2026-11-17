---
"@laminapdf/viewer": patch
"@laminapdf/wasm": patch
"@laminapdf/sdk": patch
---

The annotation layer can now resize annotations. Pass `resize` (`{ can, ops }`) to `new AnnotationLayer(…)`, and one selected annotation that `can` accepts gets live handles. Each handle takes the pointer over 24 CSS px. Dragging one resizes the annotation from that handle, and Shift keeps its proportions. Alt+arrows resize from the bottom-right handle, by 1 pt or by 10 pt with Shift. The layer stretches the annotation with its box, and your `ops` turns the stretched copy into the edit. `AnnotationWords` gains `resize`, the history label of a resize.

Each annotation's button is now at least 24 CSS px a side, so a small mark such as a caret still has a target the pointer can reach.

A square's or circle's record now carries the box it runs along: `geometry` is `{ kind: "inner", rect }` (`AnnotationGeometry::Inner` in Rust), inside its rectangle by the stroke's margin (`/RD`). An editor can resize the shape from that box, so the margin doesn't grow with each resize.
