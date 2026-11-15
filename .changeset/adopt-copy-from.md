---
"@laminapdf/sdk": patch
---

In Rust, `Document::adopt_copy_from` makes a saved copy the document's file when the copy is read through a byte source rather than held in memory. An app that appends an incremental save to a large file in place can then adopt the file without reading all of it.
