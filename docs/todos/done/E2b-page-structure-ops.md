# E2b — Page-structure ops: insert blank, delete, reorder, rotate, reverse

| Field | Value |
| --- | --- |
| ID | `E2b` |
| Status | `done` |
| Plan | [E2 page organisation §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B8a](./B8a-structure-tree.md), [B8b](./B8b-conformance-profiles-and-verapdf.md), [E1c](./E1c-metadata-ops.md), [E2j](./E2j-page-tree-and-tree-writers.md), [E2r](./E2r-planner-page-checks.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2b-page-structure-ops` |
| PR title | `feat(ops): page structure ops with reference cleanup` |
| Size | L (400–800): five ops over E2j's page-tree editor, the reference cleanup and one new code, with bindings; the planner's checks are [E2r](../done/E2r-planner-page-checks.md)'s and duplicating is [E2k](../ready/E2k-duplicate-pages.md)'s |
| PR | [#534](https://github.com/laudebugs/laminapdf/pull/534) |

## Goal

Implement the single-document page ops as view-level mapping of `PageId`s (no re-parse of content) over [E2j](../done/E2j-page-tree-and-tree-writers.md)'s page-tree editor: insert blank pages, delete, move (reorder), rotate (batch, generalising B3's `RotatePage`) and reverse, with cleanup of references to deleted pages (outline items, link destinations, named destinations, struct-tree `/Pg`, `/OpenAction`). Each op reports the widgets it touches, which [E2r](../done/E2r-planner-page-checks.md)'s checks judge. Duplicating pages is [E2k](../ready/E2k-duplicate-pages.md).

## Context

- **Design (Paper):** [O06 Organise mode](../../design/paper/edit-session-organise.md#o06-organise-mode). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation)
- [ADR 0005](../../decisions/0005-document-model.md) §5: `MovePages` keeps every `PageId`; new pages get new IDs
- [06 §3 group C](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms): mechanisms (`/Kids`, `/Count`, inherited attributes)
- [01 §10 Document logical structure](../../research/pdf/01-file-structure.md#10-document-logical-structure-77): page tree and inheritance
- ISO 32000-2 §7.7.3 (page tree, inheritable attributes), §12.3.2 (destinations)

## Scope

**In scope**

- Ops, with the payloads of [E2 plan §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions): `InsertBlankPages`, `DeletePages`, `MovePages`, `RotatePages`, `ReversePages`. `RotatePage` stays as it is
- Each op edits the tree through E2j's page-tree editor and calls E2j's `PageLabels::remap`, so every surviving page keeps its label and inserted blank pages continue the range they land in
- Reference cleanup on delete per the [E2 plan's decision](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions): `retarget` (default) sends each destination naming a deleted page to the nearest surviving page (next, else previous) with `/Fit`; `remove` removes it. It covers outline items, named destinations (name tree and PDF 1.1 `/Dests`, written with E2j's name-tree writer), links and `GoTo` actions on surviving pages and `/OpenAction`, found through E2j's `PageReferences`. Structure elements whose content was all on deleted pages are removed with their `/ParentTree` entries. Deleting every page is refused with a new code (one `codes.json` entry and its `docs/errors/` page)
- Reports: `Applied` and the façade's `Edited` gain `references` (each retargeted or removed reference), `page_order` (the page IDs in order, when the list changed) and set `still_in_earlier_revisions` for `DeletePages`
- Each op reports the fields whose widgets it deleted, rotated or moved on the page, for [E2r](../done/E2r-planner-page-checks.md)'s checks: `Effect::widgets` as it applies, `Op::page_changes()` before. `RotatePage` reports its page too (`WidgetChange::Reshown`), which turns the approval fixture's rotation in `crates/laminapdf-save/tests/planner.rs` (`auto_saves_a_signed_file_incrementally_and_keeps_the_signed_bytes`) into a `Conflict`

**Out of scope** (and which todo covers it)

- Page-tree editing, the tree writers, `PageLabels::remap` and the reference index → [E2j](../done/E2j-page-tree-and-tree-writers.md)
- The planner's field-level checks → [E2r](../done/E2r-planner-page-checks.md)
- Duplicating pages (tags and widgets) → [E2k](../ready/E2k-duplicate-pages.md)
- Pages from other documents, replace, extract → [E2c](../ready/E2c-cross-document-pages.md)
- Merge and split → [E2d](../ready/E2d-merge-and-split.md)
- The viewer across page-list changes → [E2l](../ready/E2l-viewer-across-page-edits.md); the Organise UI → [E2h](../ready/E2h-page-organiser-ui.md)
- The CLI's page commands (`laminapdf pages delete`, `extract`, `reorder`, `insert`, `merge`, `split`) → [T6a](../ready/T6a-cli-page-commands.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.01` insert blank pages; `C.03` delete pages (duplicate in E2k); `C.04` reorder pages; `C.08` rotate pages 90/180/270; `C.13` reverse page order
- **Crates and packages:** `laminapdf-ops`, the façade and bindings; `laminapdf-model`'s editor and writers come from E2j, the planner's checks from E2r
- **`Op` variants:** `InsertBlankPages`, `DeletePages`, `MovePages`, `RotatePages`, `ReversePages` (class `PageStructure`)
- **`SaveConstraint`:** class `PageStructure`; the verdicts per constraint are the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint), including the recommended rule, stricter than ISO 32000-2 §12.8 and ADR 0007 §2, that deleting or rotating a page that holds a signed signature's widget is `Restriction::SignedWidget { field }`
- **Web vs desktop:** identical (core); UI in E2h
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.01` → `round-trip` (core, pdf); `C.04` → `round-trip` (core, pdf); `C.08` → `round-trip` (pdf; core is already proved by `RotatePage`); `C.13` → `round-trip` (core, pdf). `C.03` flips in E2k, once duplicate exists too

## Tasks

- [x] Implement each op over E2j's page-tree editor, with undo records; `MovePages` keeps `PageId`s and object numbers
- [x] Reference cleanup per the [E2 plan's decision](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions), through E2j's `PageReferences` and name-tree writer, reported in `Applied` and `Edited`
- [x] `PageLabels::remap` called by every op
- [x] Each op reports its touched widgets to E2r's checks
- [x] The refusal code for deleting every page (`codes.json`, `docs/errors/`, `cargo xtask gen codes`)
- [x] Bindings (`doc.pages.insertBlank()`, `.delete()`, `.move()`, `.rotate()`, `.reverse()`) and `Edited`'s new fields
- [x] Examples in Rust and TS

## Acceptance criteria

- [x] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [x] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [x] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [x] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [x] Page count, order and rotation after each op match pypdf and pdf.js on 20 corpus files (incl. deep/unbalanced page trees and inherited attributes)
- [x] On an approval-signed fixture (the plan's recommended rule, through E2r), `DeletePages` and `RotatePages` on the page holding the signed widget give a `Restriction::SignedWidget` violation and `constraint.kind() == Conflict` (save refused without the override); the same ops on another page are `AppendOnly` and the signature still validates after the incremental save
- [x] Page labels survive every op: on a fixture with roman + arabic + prefixed ranges, each surviving page's label is unchanged after delete/move/reverse (read back in pdf.js and PDFium)
- [x] On the FieldMDP-lock fixture, deleting or rotating the page that holds the locked field's widget is a `Restriction::FieldMdp` violation; moving, reversing and inserting are not
- [x] Deleting a page that an outline item targets follows the [E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions) decision and the report lists it
- [x] Determinism: saving the same state twice gives identical bytes on native and wasm32 (ADR 0007 §6)

## Post-merge checks

None.

## Tests

- Unit/property tests (random sequences of page ops then undo all → empty overlay)
- Corpus differential vs pypdf (page order facts) and pdf.js (render hashes per page)
- Fixtures: E2j's `fixtures/pages/` (`inherited.pdf`, `unbalanced.pdf`, `references.pdf`, `labels.pdf`, `forms.pdf`), the builder's tagged goldens, and `fixtures/signed/`

## Docs to update

- API docs: pages
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Shared page dicts (same object referenced twice in `/Kids`) are invalid but occur; E2j's editor copies them on first change and warns

## As built

- **Ops** (`crates/laminapdf-ops/src/pages/`): `InsertBlankPages`, `DeletePages` (`ReferencePolicy::{Retarget, Remove}`), `MovePages`, `RotatePages` (`RotateMode::{Set, Turn}`) and `ReversePages`, over E2j's editor, with frozen samples in `fixtures/ops/v1/`. Each op first copies a page dictionary listed twice, since the page reader takes it for a cycle; E2j's editor handled such a page, `Document::pages` doesn't. `ReversePages` moves only the reversed pages, each one after its predecessor in the new order. `RotatePage` now reports its widgets as reshown (`Op::page_changes`, `Effect::widgets`), so rotating the approval fixture's page is a conflict. The save tests that rotated signed fixtures now expect it, or edit the title instead, and the round-trip suite asks the planner which rotations reshow a signed widget.
- **Cleanup** (`cleanup.rs`, `structure.rs`): under `retarget`, outline items, links and `GoTo` actions on surviving pages, the `/Names /Dests` tree (written again whole) and the PDF 1.1 `/Dests`, and `/OpenAction` point to the nearest surviving page with `/Fit`. A destination that goes through a named destination is left alone, and the name is retargeted instead. Under `remove`, an outline item loses `/Dest` and `/A`, a link leaves its page's `/Annots`, and a named destination and `/OpenAction` are removed. Structure elements whose content was all on deleted pages leave their parents' `/K`, kids of surviving elements on deleted pages are dropped, and the deleted pages' `/ParentTree` keys (their `/StructParents` and their annotations' `/StructParent`) are removed. The report is `Applied::references` and `page_order`, which become the façade's `Edited::references` (`ReferenceEdit`, `ReferenceEditKind`) and `page_order`, and WASM's `pageOrder` and `references`. `laminapdf_model::set_catalog_entry` became public under `edit`.
- **`PageSize`** moved from `laminapdf-build` to `laminapdf-model`, so the op and the builder share one type. The builder re-exports it, and the façade exports it once.
- **Bindings:** **deviation:** `doc.pages` already returns the page list in `@laminapdf/sdk`, so the ops are `Document` and `Transaction` methods (`insertBlankPages`, `deletePages`, `movePages`, `rotatePages`, `reversePages`), like `setMetadata`, and the worker client serves them. The WASM modules build with laminapdf-save's `page-checks` (ADR 0007 §11), and ADR 0016's budgets rise with an amendment for the owner: base 547,000, extract 866,000, text 1,544,000, about 71–78 KB more each, more than the 15–40 KB estimated when the owner approved the raise.
- **Examples:** `organise_pages.rs` and `organise-pages.ts`, byte-identical in every runtime to `fixtures/golden/examples/organise_pages.pdf` (from `fixtures/pages/references.pdf`), on the docs recipe page Organise pages.
- **Tests:** `crates/laminapdf-ops/tests/pages.rs` (each op on `fixtures/pages/`: labels kept, retarget and remove, structure cleanup, widgets), `pages_props.rs` (random sequences, undo all and redo all), `pages_wire.rs`. `crates/laminapdf-save/tests/page_ops.rs` gives each op's verdict on the signed fixtures: DocMDP P1–P3, nothing on an unsigned file, `SignedWidget` for delete or rotate on the approval-signed page and an incremental save keeping the signed bytes on another page, `FieldMdp` only for delete or rotate on the locked page. The round-trip suite saves every fixture with each page op, and pdf.js and qpdf (11.9 locally; CI pins 12.4.2) read them back. `packages/sdk/test/pages.test.ts` has 14 tests. The capability matrix's C.01, C.04, C.08 and C.13 are round-trip for core. The `pdf` column needs `xtask capabilities` to scan the SDK tests, whose titles carry `C.01 (pdf, round-trip)` and the rest as `metadata.test.ts` does.
- **Not done here:** the pypdf comparison over 20 corpus files. The round-trip suite's pdf.js and PDFium checks cover page count and rotation on the fixtures; PDFium needs pypdfium2 5.14.0, which the full lane installs.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
