---
"@laminapdf/viewer": patch
---

After the document worker crashes and the viewer restarts it, the pages and thumbnails that were being drawn at that moment are drawn again. Before, they stayed blank, and the thumbnail strip reported "the worker stopped" to the page as an unhandled error. A render ended on purpose with `stop()` still shows as failed.
