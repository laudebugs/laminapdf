---
"@laminapdf/sdk": patch
"@laminapdf/wasm": patch
"@laminapdf/render": patch
"@laminapdf/render-hayro": patch
"@laminapdf/render-pdfium": patch
"@laminapdf/viewer": patch
---

`FlowTemplate` is now an interface rather than a type alias, with the same keys and the same checks, so the API reference names it where `builder.flow()` takes it instead of spelling out every key of every block. Its reference page explains the placeholders, with an example. Every type declaration in the TypeScript reference is now a code block, one key or union case to a line.
