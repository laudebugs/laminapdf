# E2c — Insert pages from other PDFs, replace pages, extract and export ranges

| Field | Value |
| --- | --- |
| ID | `E2c` |
| Status | `ongoing` |
| Plan | [E2 page organisation §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2k](../done/E2k-duplicate-pages.md), [E2p](../done/E2p-source-documents-and-copier.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2c-cross-document-pages` |
| PR title | `feat(ops): import, replace and extract pages across documents` |
| Size | L (400–800): two ops and two functions over [E2p](../done/E2p-source-documents-and-copier.md)'s copier and sources, the rules for what follows a page, and bindings |
| PR | — |

## Goal

Copy pages between documents with [E2p](../done/E2p-source-documents-and-copier.md)'s `ObjectCopier`, from sources added with its `add_source`; replace pages while keeping links and bookmarks pointing at the slot; extract selected pages and export ranges as new documents.

## Context

- **Design (Paper):** [O07 Insert pages and merge documents](../../design/paper/edit-session-organise.md#o07-insert-pages-and-merge-documents). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [05 §3.1 Merge and split](../../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)
- [ADR 0005](../../decisions/0005-document-model.md) §3, §5: fresh numbers, new IDs for imported pages
- [06 §3 group C](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms): "drop/merge annotations' `/P`, struct-tree links"

## Scope

**In scope**

- Rules for what follows a page ([E2 plan §2.2](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page)): annotations (fix `/P`, new `/NM`), links to pages that didn't come removed and reported, widget annotations carried per `Carry::fields` (renamed with [E2k](../done/E2k-duplicate-pages.md)'s `<partial>#<n>` helper by default, or linked; [E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions)), struct-tree elements dropped with `TagsDropped` until [E2o](../ready/E2o-structure-tree-merge.md) merges them
- Ops, with the payloads of [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions): `InsertPagesFrom` and `ReplacePages`; `extract_pages` and `export_ranges` run as jobs with run-to-completion forms (ADR 0028 F10)
- `extract_pages(doc, pages) -> Document` (new document whose base is empty, built in the overlay) and `export_ranges(doc, ranges) -> Vec<Document>`
- Page labels and named destinations through [E2j](../done/E2j-page-tree-and-tree-writers.md)'s shared writers: `InsertPagesFrom` and `ReplacePages` call `PageLabels::remap` (target pages keep their labels; inserted pages continue the range they land in), and `extract_pages`/`export_ranges` write the source's labels for the extracted pages with the number-tree writer; named destinations used by imported links are copied with the name-tree writer
- `ReplacePages` reports its touched widgets to [E2r](../done/E2r-planner-page-checks.md)'s checks: replacing a page that holds a signed signature widget is `Restriction::SignedWidget { field }`

**Out of scope** (and which todo covers it)

- The blob store, `add_source` and the `ObjectCopier` → [E2p](../done/E2p-source-documents-and-copier.md)
- Merging outlines, AcroForm and struct trees across whole documents; split → [E2d](../ready/E2d-merge-and-split.md)
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

- [ ] Annotation and widget handling rules; tests for `/P` fix-up
- [ ] `InsertPagesFrom`/`ReplacePages` with undo (removal of created objects; reserved numbers stay reserved)
- [ ] `extract_pages` and `export_ranges` producing deterministic output, with page labels written through E2j's number-tree writer
- [ ] Encrypted sources: decrypt on read (B2) and write unencrypted into an unencrypted target; encrypted targets wait for E11
- [ ] Bindings and examples (`examples/extract-pages.*`)

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Inserting pages from 10 corpus files into a fixture renders each imported page identically to its source in PDFium (pixel diff below threshold)
- [ ] Replace keeps outline items and links that pointed at the replaced slot pointing at the new page
- [ ] Extracted documents pass `qpdf --check` and keep page labels for the extracted range

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
