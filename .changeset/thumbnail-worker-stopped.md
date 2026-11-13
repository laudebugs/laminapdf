---
"@laminapdf/viewer": patch
---

The thumbnail strip no longer raises "the worker stopped" as an unhandled error when the document worker crashes, is stopped or is terminated while a thumbnail is drawing. The thumbnail keeps its placeholder. The client's `crashed` and `restarted` events already report a crash or a restart.
