---
"@laminapdf/viewer": minor
---

Bring pages in from other PDFs and write pages out as new files.

`client.addSource(bytes)` makes another PDF a source of the open document, so `insertPagesFrom`, `replacePages` and `mergeDocuments` can copy its pages. If the worker restarts, the client adds the file again before it replays those edits. A file opened with a password can't be added again, so its edits are lost then. `previewSource` reads a file's pages and claims before they come in, and `renderSource` draws them as thumbnails. `writeParts` writes pages of the open document as new files, or as one zip of them, and `mergeFiles` merges files into a new document.
