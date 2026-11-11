---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Undo and redo say which pages to draw again. `doc.undo()` and `doc.redo()` now return `{ outcome, touchedPages }` instead of the outcome alone, so a viewer can redraw only the pages the undone or redone edit changed. `doc.apply(ops)` also says whether the edit `changed` anything: an edit that only sets values the document already had leaves nothing to undo. `doc.isEncrypted` tells you whether the file is encrypted. The Rust `Document::undo` and `redo` return the same `Reverted` record, and `Edited` has `changed`.
