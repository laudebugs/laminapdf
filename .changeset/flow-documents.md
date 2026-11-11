---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

`builder.flow(document, { data, fonts, images })` lays out a `laminapdf.document/1` document as tagged pages that flow, in the text module. The document is plain objects, typed by `FlowDocument`, so your editor catches a misspelt key or an unknown block `type`. A template with placeholders is a `FlowTemplate`: given `data`, `flow` fills it first, and a value the data lacks fails with `template-data-missing` and its pointer. The fonts and images the document names come by name as `Font`s and `Image`s. The new `invoice-from-json` example fills a two-page invoice from JSON and writes the same bytes as the Rust one.
