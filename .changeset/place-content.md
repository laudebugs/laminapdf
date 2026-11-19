---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Place images, shapes and pages of other PDFs on a page as page content, then move, fade, crop, restack or remove them, even after the file is saved and reopened. Unlike a stamp or a comment, the object is part of the page: it prints and every reader draws it. `doc.content.add(index, object)` adds one under an ID of your own: a JPEG or PNG (a PNG keeps its transparency), a line, rectangle, ellipse or polygon, or a page of another PDF, kept as vectors and text. `doc.content.update(index, id, changes)` and `doc.content.remove(index, id)` change and remove an object, and `doc.content.added(index)` lists what a page holds, with each object's box. In a tagged file an object becomes a figure with its alternate text, or an artifact when it is decoration.

Four new codes say what went wrong: `content-id-in-use`, `unknown-content-object`, `content-object-invalid` (bytes that aren't a JPEG, a PNG or a PDF with the page named) and `content-needs-alt-text` (a picture without alternate text in a PDF/UA-1 file). The `pdfa-transparency` warning now also lists the pages with see-through objects you added.
