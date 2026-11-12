---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Groundwork for copying pages between documents:

- `unknown-source` is a new error code: an op or a call named a source document that wasn't added to the session.

In Rust: `laminapdf_ops::Document::add_source(bytes, password)` opens another PDF as a source document and returns its `SourceId`, the SHA-256 of its bytes, so the same file has the same ID on every device. `source_document(id)` returns it, and `blobs()` the session's `BlobStore`. `laminapdf_model::ObjectCopier` copies pages, and everything they reach, from one document into another, sharing what several pages use, within `Limits`. The ops that use them, inserting pages from another file and merging, come next.
