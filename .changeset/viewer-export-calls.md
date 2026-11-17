---
"@laminapdf/viewer": patch
---

Adds the export dialog's worker calls: `extractText` (plain text, Markdown or JSON with positions), `extractPictures`, `exportSummary`, `linksAndBookmarks` (JSON or CSV) and `imagesToPdf`, each reading the document as its edits leave it. `exportPage` and `exportTiff` take `annotations: false` to leave comments and form fields out. `ZipWriter` writes a stored zip on the page a file at a time, as `Blob` parts. The worker's zip of split parts now refuses past its limits with `limit-exceeded` instead of a `RangeError`.
