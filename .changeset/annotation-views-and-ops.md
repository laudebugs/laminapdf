---
"@laminapdf/sdk": patch
---

Documents can now list a page's annotations and change or delete them. `doc.pageAnnotations(index)` returns each annotation in page space: its ID, subtype, rectangle, text, author, dates, flags, colour, reply and group links, and its own geometry (highlight areas, ink strokes, a polygon's vertices, a line, a callout). The new `updateAnnot` op changes an annotation's text, author, subject, flags or position, keeping its appearance; `deleteAnnot` deletes it together with its pop-up, its replies and the rest of its group, and the edit lists what went with it in `alsoDeleted`. A locked annotation refuses the change.
