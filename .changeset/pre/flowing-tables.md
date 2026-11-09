---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Rust flows can now hold tables that run across pages. A table's header rows repeat at the top of every page it continues on, no body row is split between pages, and its footer rows, such as a total, stay on the page of the last body row. Each column takes a fixed width, a share of the free width, or its widest cell's width, and the widths are worked out once so the columns line up on every page. Cells hold paragraphs, can span columns and align their text; a table sets its cells' padding and can have a frame, rules and a shaded header. The output is tagged as one table across all its pages, and the repeated header rows are marked as page furniture, so screen readers and text extraction read the header once. Tagged documents can also use the `THead`, `TBody` and `TFoot` structure types, which `@laminapdf/sdk` now accepts as tag names.
