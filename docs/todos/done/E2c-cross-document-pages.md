# E2c — Insert pages from other PDFs, replace pages, extract and export ranges

| Field | Value |
| --- | --- |
| ID | `E2c` |
| Status | `done` |
| Plan | [E2 page organisation §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2k](./E2k-duplicate-pages.md), [E2p](./E2p-source-documents-and-copier.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2c-cross-document-pages` |
| PR title | `feat(ops): import, replace and extract pages across documents` |
| Size | L (400–800): two ops and two functions over [E2p](../done/E2p-source-documents-and-copier.md)'s copier and sources, the rules for what follows a page, and bindings |
| PR | [laudebugs/laminapdf#544](https://github.com/laudebugs/laminapdf/pull/544) |

## Goal

Copy pages between documents with [E2p](../done/E2p-source-documents-and-copier.md)'s `ObjectCopier`, from sources added with its `add_source`; replace pages while keeping links and bookmarks pointing at the slot; extract selected pages and export ranges as new documents.

## Context

- **Design (Paper):** [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [05 §3.1 Merge and split](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)
- [ADR 0005](../../decisions/0005-document-model.md) §3, §5: fresh numbers, new IDs for imported pages
- [06 §3 group C](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms): "drop/merge annotations' `/P`, struct-tree links"

## Scope

**In scope**

- Rules for what follows a page ([E2 plan §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page)): annotations (fix `/P`, new `/NM`), links to pages that didn't come removed and reported, widget annotations carried per `Carry::fields` (renamed with [E2k](../done/E2k-duplicate-pages.md)'s `<partial>#<n>` helper by default, or linked; [E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions)), struct-tree elements dropped with `TagsDropped` until [E2o](../ongoing/E2o-structure-tree-merge.md) merges them
- Ops, with the payloads of [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions): `InsertPagesFrom` and `ReplacePages`; `extract_pages` and `export_ranges` run as jobs with run-to-completion forms (ADR 0028 F10)
- `extract_pages(doc, pages) -> Document` (new document whose base is empty, built in the overlay) and `export_ranges(doc, ranges) -> Vec<Document>`
- Page labels and named destinations through [E2j](../done/E2j-page-tree-and-tree-writers.md)'s shared writers: `InsertPagesFrom` and `ReplacePages` call `PageLabels::remap` (target pages keep their labels; inserted pages continue the range they land in), and `extract_pages`/`export_ranges` write the source's labels for the extracted pages with the number-tree writer; named destinations used by imported links are copied with the name-tree writer
- `ReplacePages` reports its touched widgets to [E2r](../done/E2r-planner-page-checks.md)'s checks: replacing a page that holds a signed signature widget is `Restriction::SignedWidget { field }`

**Out of scope** (and which todo covers it)

- The blob store, `add_source` and the `ObjectCopier` → [E2p](../done/E2p-source-documents-and-copier.md)
- Merging outlines, AcroForm and struct trees across whole documents; split → [E2d](../done/E2d-merge-and-split.md)
- The CLI's page commands (`laminapdf pages delete`, `extract`, `reorder`, `insert`, `merge`, `split`) → [T6a](../ready/T6a-cli-page-commands.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.02` insert pages from other PDFs; `C.05` extract selected pages; `C.16` replace pages, preserving the rest; `H.10` export pages/ranges
- **Crates and packages:** `laminapdf-ops`, `laminapdf-save` (new-document save), bindings; the copier is E2p's
- **`Op` variants:** `InsertPagesFrom`, `ReplacePages` (class `PageStructure`)
- **`SaveConstraint`:** class `PageStructure`; the verdicts per constraint are the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint). Extraction writes a new document (a full write, `Free`)
- **Web vs desktop:** identical; desktop accepts files dropped from Finder/Explorer as sources, web from the file picker
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.02` → `round-trip` (core, pdf); `C.05` → `write` (core, pdf); `C.16` → `round-trip` (core, pdf); `H.10` → `write` (core, pdf)

## Tasks

- [x] Annotation and widget handling rules; tests for `/P` fix-up
- [x] `InsertPagesFrom`/`ReplacePages` with undo (removal of created objects; reserved numbers stay reserved)
- [x] `extract_pages` and `export_ranges` producing deterministic output, with page labels written through E2j's number-tree writer
- [x] Encrypted sources: decrypt on read (B2) and write unencrypted into an unencrypted target; encrypted targets wait for E11
- [x] Bindings and examples (`examples/extract-pages.*`)

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [x] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [x] Inserting pages from 10 corpus files into a fixture renders each imported page identically to its source in PDFium (pixel diff below threshold)
- [x] Replace keeps outline items and links that pointed at the replaced slot pointing at the new page
- [x] Extracted documents pass `qpdf --check` and keep page labels for the extracted range

## Post-merge checks

None.

## Tests

- Integration tests in `crates/laminapdf/tests/pages_cross_doc.rs`
- Render diff (PDFium) per imported page
- The copier's fuzz target is E2p's

## Docs to update

- API docs: importing and extracting pages
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **The ops** (`crates/laminapdf-ops/src/pages/from.rs`): `InsertPagesFrom { source, pages, at, carry }` and `ReplacePages { targets, source, pages, carry }`, class `PageStructure`, no rewrite. `Tx` holds a read-only borrow of the session's sources, which `Op::apply_to` passes on; an op naming a source the session lacks is `unknown-source`. `ReplacePages` puts each copy before its target, hands the target's references (outline items, links, named destinations, `GoTo` actions, `/OpenAction`) to the copy with `/Fit` through E2b's `cleanup::destinations`, then runs `DeletePages` on the targets, so leftovers are deleted under `retarget`, structure and widgets are cleaned and `Op::page_changes` names the targets `Deleted` for E2r's `SignedWidget` and FieldMDP checks. Copies left over follow the last copy.
- **What follows a page** (`import.rs`): one `ObjectCopier` call copies every chosen page, so links between them land on the copies. Then each copy loses `/StructParents` and `/B`, and its annotations get `/P`, a new `/NM` and no `/StructParent`. A link whose explicit destination came out `null`, or whose named destination names a page left behind, is removed (`Applied::links_removed`). A kept named destination is copied into the target's `/Names /Dests` tree, as `<name>#<n>` when the name is taken. `Carry::annotations` `false` keeps only widgets. Fields are fitted per root field of the source (`page_widgets` and the `/Parent` chain): kids that didn't come along are pruned, a free name is kept and listed in `/AcroForm /Fields` (created when the target has no form), a taken one (`FieldNames::clashes`, which also counts prefixes) is renamed with `unique_partial`, or with `FieldClash::Link` its widgets become kids of the target's field of that name when both are terminal with one `/FT` and `/V` and the target's has widget kids (`FieldOutcome::Linked`; otherwise renamed). Signature widgets are left off (`NotCopied`). The catalog's `/Version` rises to the source's when that is newer.
- **After the copy** (`import.rs`, `dests.rs`): every action on a copied annotation or page is fitted, `/AA` triggers and `/Next` chains included (a `GoTo` to a page left behind is dropped; a link whose action goes nowhere goes); a destination named by a name object is written as a string, which names the copy in `/Names /Dests` (ISO 32000-2 §12.3.2.4), and a copied destination's indirect operands become values; the target's form gets the source's `/DA` when it has none and its `/DR` entries under names it lacks (`ObjectCopier::copy_value`); a field joined with `FieldClash::Link` is reported in `Applied::widgets` for the planner's field-lock checks; then a sweep deletes every object the import created that nothing reaches (annotations left off, signature dictionaries, removed links), so a save never writes them, and takes `/StructParents` and `/StructParent` off copied Form XObjects.
- **Tags:** copies of tagged pages are `Applied::tags_dropped`; `PendingChanges::tags_dropped` keeps them, and `pending_conformance_warnings` adds the new `imported-tags-dropped` (opWarnings, with its error page) on a file whose packet claims PDF/UA-1 while a copy is still a page.
- **New documents** (`extract.rs`): `Document::extract_pages` copies into a catalog and empty page tree written over `laminapdf_cos::Document::empty` (same version and limits), signature fields unsigned (`/V` removed), the source's labels for the chosen pages through `PageLabels::remap` and the number-tree writer; `export_ranges` maps it over ranges. A new document's save is a full write; extracting from an encrypted file writes a plain one.
- **Model** (`laminapdf-model`): `FieldNames::clashes`, `Document::field_by_name` (over the walk `field_names` now shares) and `Outline::named_destination_value`.
- **Façade:** `add_source`, `source_pages`, `extract_pages`, `export_ranges`, `export_ranges_with(&CancelToken)` and `ExportJob` (one range per unit; a range's pages are copied in one call so their links survive, which is why extraction itself isn't stepped page by page: **deviation**); `Edited::links_removed` and `tags_dropped`; re-exports `InsertPagesFrom`, `ReplacePages`, `Carry`, `FieldClash`, `SourceId`.
- **WASM and SDK:** `WasmDocument.addSource` answers `{ id, pages }` (one export instead of two keeps the extract module's glue within its 5,000-byte budget), `extractPages`; `EditedJs.linksRemoved`, `tagsDropped` and the `linked` outcome. `@laminapdf/sdk`'s `Document.addSource` (returns `AddedSource`), `insertPagesFrom`, `replacePages`, `extractPages`, `exportRanges`; the `Transaction` has the two ops; the worker client serves all but the two that return documents. **Deviation:** as in E2b, the methods sit on the document, not under `doc.pages`. Sizes after the change, gzipped: base 649,781, extract 968,009, text 1,638,088 bytes; the SDK's JS 23,005.
- **Examples and docs:** `insert_pages` (a page of `references.pdf` into `forms.pdf`; its `name` field becomes `name#2`) and `extract_pages` (pages 3 to 5 of `labels.pdf`, labelled iii, 1, 2), each with a TypeScript twin of the same bytes, byte and render goldens; the recipe "Copy pages between PDFs" and a paragraph in "Edit a PDF".
- **Tests:** `crates/laminapdf-ops/tests/import.rs` (placement, `/P`, links kept and removed, named destinations renamed, field names, linking, signatures, tags and the PDF/UA warning, undo and redo of both ops, unknown sources and pages, an encrypted source, extraction with labels and unsigned signatures, export), `pages_wire.rs` (two frozen samples and rejects), `crates/laminapdf-save/tests/page_ops.rs` (each op's verdict on the signed fixtures, and incremental saves that keep the signed bytes), the round-trip suite (`insertPagesFrom` and `replacePages` on every fixture, C.02 and C.16; every fixture's last page extracted and its pages split in two, C.05 and H.10, checked by qpdf, pdf.js and PDFium), `crates/laminapdf/tests/pages_cross_doc.rs`, `crates/laminapdf-render-pdfium/tests/import.rs` (pages from ten corpus files render pixel for pixel as their source in hayro and PDFium) and `packages/sdk/test/pages.test.ts`.
