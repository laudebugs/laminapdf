---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

A file attachment's file can be replaced in place: `updateAnnot` with a `fileAttachment` patch's new `file` (its bytes by their blob ID, name and type) writes the new file into the same annotation. The annotation keeps its place, icon, replies, note, author and dates; before, the only way was to delete it, replies included, and attach the file again.
