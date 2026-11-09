---
"@laminapdf/wasm": minor
---

The Rust façade reads a document's outline (`Document::outline()`) and named destinations (`Document::named_destination(name)`), each target a page index and a view in page space. The extract and text modules give them to JavaScript as `WasmDocument.outline()` and `namedDestination(name)`.
