---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Set a document's page labels, the names viewers show for its pages ("Cover", "iv", "A-3"):

- `doc.setPageLabels(ranges)` replaces them in one edit, and `Transaction.setPageLabels` adds the same edit to a transaction. Each range names its first page by ID, with a style (`decimal`, `upperRoman`, `lowerRoman`, `upperLetters`, `lowerLetters`), a prefix and a first number; no ranges remove the labels.
- Ranges that don't start on the first page, are out of page order or number from 0 fail with `invalid-label-ranges`, whose `data.what` says which rule.
- `doc.pages[i].label` reads the new labels.

In Rust: the `SetPageLabels` op with `PageLabelRange`, and in `laminapdf-model` `LabelRange::new` and `PageLabels::from_ranges`.
