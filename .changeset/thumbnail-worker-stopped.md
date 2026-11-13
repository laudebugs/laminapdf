---
"@laminapdf/viewer": patch
---

The thumbnail strip no longer raises "the worker stopped" as an unhandled error when the document worker crashes or is stopped while a thumbnail is drawing. The thumbnail keeps its placeholder; the client's `crashed` and `restarted` events already report the loss.
