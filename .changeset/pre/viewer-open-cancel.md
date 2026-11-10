---
"@laminapdf/viewer": patch
---

A cancelled `open()` no longer leaves the page and the document Web Worker on different documents. The cancel can arrive after the Web Worker already opened the new file. `open()` then resolves with that document instead of rejecting, so renders keep working.
