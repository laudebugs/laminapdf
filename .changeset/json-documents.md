---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Rust callers can now describe flowing pages as JSON (TypeScript support follows), in the format `laminapdf.document/1`, and lay them out with `Builder::document`. A document names its page, its text styles and its blocks (paragraphs, headings, lists, tables, figures, a contents page and more), and its fonts and images by name, with their bytes given beside it. Its JSON Schema ships in `docs/schemas/document.schema.json`.

A template is a document with placeholders that `flow::document::fill` replaces from data: `$data` for a value, `$each` to repeat a block or table row per item, `$item` for a value of the current item, and `$if` to keep a block only when a value is there. Templates have no expressions, so the data arrives formatted.

A document with an unknown key, a missing one or a value of the wrong kind fails with `invalid-document`, which names the value's place in `data.pointer` and suggests the closest key in `didYouMean`. A template whose data lacks a value fails with `template-data-missing`, and data of the wrong kind with `template-data-type`.
