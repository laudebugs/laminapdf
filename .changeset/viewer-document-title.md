---
"@laminapdf/viewer": patch
---

`ViewerClient.open` now returns the document's title from its metadata as `title`, when the file has one, so an app can name its window after the document.
