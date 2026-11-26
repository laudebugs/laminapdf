---
"@laminapdf/viewer": minor
---

`client.exportComments()`, `client.previewImport()`, `client.importComments()` and `client.commentSummary()` give the open document's comments to other apps and bring theirs back, in the document worker. An import is one undoable edit that a restarted worker imports again; the summary is a new document the worker saves and lets go, so the page only holds its bytes.
