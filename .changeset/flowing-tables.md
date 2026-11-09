---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Rust flows can now hold tables that run across pages. A table's header rows repeat at the top of every page it continues on, no body row is split between pages, and its footer rows, such as a total, stay on the page of the last body row. Columns are fixed widths, shares of the free width, or as wide as their widest cell, worked out once so they line up on every page. Cells hold paragraphs and can span columns, align their text and add padding; tables can have a frame, rules and a shaded header. The output is tagged as one table across all its pages, and the repeated header rows are marked as page furniture, so screen readers and text extraction read the header once. Tagged documents can also use the `THead`, `TBody` and `TFoot` structure types, which `@laminapdf/sdk` now accepts as tag names.
