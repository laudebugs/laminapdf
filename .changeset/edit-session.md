---
"@laminapdf/viewer": minor
---

An edit session for the open document. `viewer.edits` holds its history (each step with its label, pages and time, how many are done, and how many changes are unsaved) and runs `apply(ops, label)`, `undo()`, `redo()` and `goTo(step)` one after another, each redrawing only the pages it touched. An undo that could restore only part of a step says so. The history keeps 500 steps. If the document worker restarts, it opens the file again and applies the session's edits again, so nothing is lost. An encrypted document can't be edited yet, and says so when it opens (`encrypted`). A page turned while the zoom fits the width or the page now fits again.
