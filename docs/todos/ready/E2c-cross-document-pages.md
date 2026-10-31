# E2c — Insert pages from other PDFs, replace pages, extract and export ranges

| Field | Value |
| --- | --- |
| ID | `E2c` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2b](./E2b-page-structure-ops.md) |
| Blocks | [E2d](./E2d-merge-and-split.md), [E2g](./E2g-page-labels.md), [E2h](./E2h-page-organiser-ui.md), [E16f](./E16f-imposition.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2c-cross-document-pages` |
| PR title | `feat(ops): import, replace and extract pages across documents` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Copy pages between documents: deep-copy a page and everything reachable from it into the target's overlay, renumbered with fresh object numbers and deduplicated resources; replace pages while keeping links and bookmarks pointing at the slot; extract selected pages and export ranges as new documents.

## Context

- **Design (Paper):** [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [05 §3.1 Merge and split](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)
- [ADR 0005](../../decisions/0005-document-model.md) §3, §5: fresh numbers, new IDs for imported pages
- [06 §3 group C](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms): "drop/merge annotations' `/P`, struct-tree links"

## Scope

**In scope**

- `ObjectCopier` in `lamina-model`: deep copy across documents with cycle handling, `Limits`, and resource dedupe by stream hash
- Rules for what follows a page: annotations (fix `/P`, new `/NM`), widget annotations (fields detached or carried per E2d merge rules), struct-tree parents dropped (warning) unless E2d merge
- Ops: `InsertPagesFrom { source: SourceDocId, pages, at }`, `ReplacePages { targets, source, pages }`
- `extract_pages(doc, pages) -> Document` (new document whose base is empty, built in the overlay) and `export_ranges(doc, ranges) -> Vec<Document>`
- Page labels and named destinations through [E2b](E2b-page-structure-ops.md)'s shared writers: `InsertPagesFrom` and `ReplacePages` call `PageLabels::remap` (target pages keep their labels; inserted pages continue the range they land in), and `extract_pages`/`export_ranges` write the source's labels for the extracted pages with the number-tree writer; named destinations used by imported links are copied with the name-tree writer
- `ReplacePages` reuses E2b's signed-widget check: replacing a page that holds a signed signature widget is `Violation::SignedWidgetPage`
- Source documents referenced by a session-level `SourceDocId`, bytes held in the blob store (ADR 0006 §1 `BlobId`)

**Out of scope** (and which todo covers it)

- Merging outlines, AcroForm and struct trees across whole documents; split → [E2d](E2d-merge-and-split.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.02` insert pages from other PDFs; `C.05` extract selected pages; `C.16` replace pages, preserving the rest; `H.10` export pages/ranges
- **Crates and packages:** `lamina-model` (copier), `lamina-ops`, `lamina-save` (new-document save), bindings
- **`Op` variants:** `InsertPagesFrom`, `ReplacePages` (class `PageStructure`)
- **`SaveConstraint`:** `Free` → incremental. Signed: page insertion/deletion/reordering is class `PageStructure`. Under a certification signature it is a DocMDP violation at every level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → blocked by default and allowed only by the explicit invalidation override (the warning names the signatures, ADR 0007 §3). With approval signatures only (no certification) it is `AppendOnly`: saved as an incremental update that keeps their `/ByteRange` intact (the signature panel, E10h, reports the later page change); it is a violation only when it changes a field locked by FieldMDP/`/Lock` (e.g. deleting or duplicating a page that carries a locked field's widget). Extraction writes a new document (full write, `Free`)
- **Web vs desktop:** identical; desktop accepts files dropped from Finder/Explorer as sources, web from the file picker
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.02` → `round-trip` (core, pdf); `C.05` → `round-trip` (core, pdf); `C.16` → `round-trip` (core, pdf); `H.10` → `write` (core, pdf)

## Tasks

- [ ] Implement `ObjectCopier` with an old→new number map, recursion limit and stream-hash dedupe
- [ ] Annotation and widget handling rules; tests for `/P` fix-up
- [ ] `InsertPagesFrom`/`ReplacePages` with undo (removal of created objects; reserved numbers stay reserved)
- [ ] `extract_pages` and `export_ranges` producing deterministic output, with page labels written through E2b's number-tree writer
- [ ] Encrypted sources: decrypt on read (B2) and write unencrypted into an unencrypted target; encrypted targets wait for E11
- [ ] Bindings and examples (`examples/extract-pages.*`)

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Inserting pages from 10 corpus files into a fixture renders each imported page identically to its source in PDFium (pixel diff below threshold)
- [ ] Resources shared by several imported pages are copied once (object count test)
- [ ] Replace keeps outline items and links that pointed at the replaced slot pointing at the new page
- [ ] Extracted documents pass `qpdf --check` and keep page labels for the extracted range

## Post-merge checks

None.

## Tests

- Integration tests in `crates/lamina/tests/pages_cross_doc.rs`
- Render diff (PDFium) per imported page
- Fuzz target for the copier over corpus pairs

## Docs to update

- API docs: importing and extracting pages
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Huge shared resources (fonts) duplicated across many imports: dedupe by hash may be slow; cache hashes per source

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
