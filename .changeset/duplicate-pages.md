---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Duplicate pages: `doc.duplicatePages(ids, at)` copies pages to a place in the document, as one edit that undo reverts. A copy shares its page's content but gets annotations of its own, and each form field it shows becomes a new field named `<name>#2` (or the next free number) with its own value. A signature's field isn't copied. The edit's `fields` lists each field the copy renamed or left out. In a tagged document the copy gets tags of its own. Content of an element that continues onto another page stays untagged on the copy, the edit's `tagsIncomplete` lists that copy, and on a PDF/UA-1 file the save plan warns with the new code `duplicate-tags-incomplete`.

In Rust: `DuplicatePages`, with `Edited::fields` (`FieldChange`, `FieldOutcome`) and `Edited::tags_incomplete`. `laminapdf-model` gains `Document::field_name`, `Document::field_names` and `FieldNames::unique_partial`, the rule that names a copied field.
