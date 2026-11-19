---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

`updateAnnot` can now replace an attached file in place. Give the `fileAttachment` patch a new `file` (its blob ID, name and type), and the annotation keeps its place, icon, replies, note, author and dates. Before, you had to delete the annotation, replies included, and attach the file again.
