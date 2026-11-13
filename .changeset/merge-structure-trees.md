---
"@laminapdf/sdk": minor
"@laminapdf/wasm": minor
---

Copied and merged pages keep their tags. When you copy pages into a tagged document with `insertPagesFrom` or `replacePages`, or merge into one with `mergeDocuments`, the elements that hold their content join the document's structure tree, so a screen reader still reads them in order. A merge puts each file's elements under a `Part` element named after it, and `Document.merge` makes a tagged file when every input is tagged. A role, class or element ID the document already uses another way comes in as `<name>#2`, listed in the edit's `structureRenamed`. Copying pages into a file that claims PDF/UA-1 from one that doesn't warns with the new code `imported-pdfua-unchecked`. `extractPages` and `split` keep the tags of a tagged document too.

In Rust: `Applied::structure_renamed` and `Edited::structure_renamed` with `StructureRename` and `StructureName`, `PendingChanges::pdfua_unchecked`, and `ConformanceWarning::ImportedPdfuaUnchecked`. `laminapdf-model`'s `StructTree` reads `/ClassMap`, `/IDTree` and `/Namespaces`, and each `StructElem` its `/ID` and classes.
