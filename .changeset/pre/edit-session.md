---
"@laminapdf/viewer": minor
---

An edit session for the open document. `viewer.edits` holds its history: each step with its label, pages and time, how many steps are done, and how many changes are unsaved. It runs `apply(ops, label)`, `undo()`, `redo()` and `goTo(step)` one after another, and each redraws only the pages it touched. An undo that could restore only part of a step says so, and `onFailed` reports a call that failed. The history keeps 500 steps. If the document worker restarts, it opens the file again and replays the session's steps, so nothing is lost; when the replay couldn't reproduce the document, it says the edits were lost. An encrypted document can't be edited yet, and says so when it opens (`encrypted`). When the zoom fits the width or the page, a page you turn is fitted again.
