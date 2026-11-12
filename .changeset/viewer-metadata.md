---
"@laminapdf/viewer": minor
---

`client.metadata()` reads the open document's metadata as its edits leave it: the title, authors, subject, keywords, the creating and producing applications and the dates, from the Info dictionary and the XMP metadata together, with the fields where they disagree. A properties form can show it, then apply its changes with `viewer.edits.apply` as `setMetadata` and `removeMetadata` ops.
