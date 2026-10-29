# lamina-model

Typed, tolerant views over a PDF's COS graph and the stable `LaminaId`s that ops, undo and sync address ([ADR 0005](../../docs/decisions/0005-document-model.md) §4–§5). Part of lamina's pure core: bytes in, bytes out.

- `Document`: a `lamina_cos::Document` (it dereferences to one) plus the document-level `IdAllocator`.
- `Pages` / `Page`: boundary boxes as normalised `Rect`s (crop, bleed, trim and art boxes clipped and defaulted as §14.11.2 says), `Rotation`, user unit, resources, contents and annotation references, page labels.
- `Outline` / `OutlineItem`: titles (PDFDocEncoding, UTF-16BE, UTF-8), nesting, open state, destinations resolved to `PageId` + view (explicit, `/Dests`, `/Names /Dests`, `GoTo` actions), action kinds.
- `Info` with dates as `PdfDate` (raw string kept when invalid); `NameTree`, `NumberTree`; `PageLabels`.
- `LaminaId` (`p12.0`, path IDs such as `p5.0/Kids/3` for direct dictionaries) and the typed `PageId`, `OutlineItemId`, `AnnotId`, `FieldId`.

Per-field tolerance follows [ADR 0003](../../docs/decisions/0003-reader-tolerance-and-limits.md): a malformed field is absent (or its default) plus a warning on the document (`invalid-box`, `invalid-rotate`, `name-tree-node-skipped`, `outline-cycle`, `outline-item-skipped`, `invalid-date`, `invalid-page-label`); strict mode turns each into `Error::Strict`. Tree walks are iterative with visited sets and skip subtrees deeper than `Limits::max_tree_depth`.

Tests: `tests/views.rs`, `ids.rs`, `limits.rs`, `corpus.rs` (pypdf's facts in `fixtures/corpus/expected.json`) and `differential.rs` (pypdf over the corpus and fixed-seed mutants, `fixtures/differential/`).
