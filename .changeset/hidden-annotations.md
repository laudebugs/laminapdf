---
"@laminapdf/wasm": minor
"@laminapdf/render": patch
"@laminapdf/render-pdfium": patch
"@laminapdf/render-hayro": patch
---

A render can now leave out chosen annotations, so an editor can draw the annotation it moves without the page showing it twice. `RenderRequest.hiddenAnnotations` lists the annotations to leave out by their place in the page's `/Annots`, the `index` that `doc.pageAnnotations` reports, and only that render leaves them out. The place names the annotation, not its name (`/NM`), which two annotations may share. PDFium sets the annotation's Hidden flag for the render and clears it afterwards. hayro draws every annotation or none, so it draws the tile again without annotations and copies that over each hidden annotation's rectangle; an annotation that overlaps a hidden one is left out inside that rectangle too.

In Rust, `RenderRequest` gains `hidden_annotations` and `with_hidden_annotations`, and is no longer `Copy`: clone a request you reuse. The hayro module's `HayroDocument.render` in `@laminapdf/wasm` takes the hidden places as a `Uint32Array` before the output buffer.
