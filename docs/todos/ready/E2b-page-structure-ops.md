# E2b — Page-structure ops: insert blank, delete, reorder, rotate, reverse

| Field | Value |
| --- | --- |
| ID | `E2b` |
| Status | `ready` |
| Plan | [E2 page organisation §2.1](../../plans/editor/e2-page-organisation.md#21-ops-and-functions) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B8a](../done/B8a-structure-tree.md), [B8b](../done/B8b-conformance-profiles-and-verapdf.md), [E1c](../done/E1c-metadata-ops.md), [E2j](./E2j-page-tree-and-tree-writers.md), [E2r](./E2r-planner-page-checks.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2b-page-structure-ops` |
| PR title | `feat(ops): page structure ops with reference cleanup` |
| Size | L (400–800): five ops over E2j's page-tree editor, the reference cleanup and one new code, with bindings; the planner's checks are [E2r](E2r-planner-page-checks.md)'s and duplicating is [E2k](E2k-duplicate-pages.md)'s |
| PR | — |

## Goal

Implement the single-document page ops as view-level mapping of `PageId`s (no re-parse of content) over [E2j](E2j-page-tree-and-tree-writers.md)'s page-tree editor: insert blank pages, delete, move (reorder), rotate (batch, generalising B3's `RotatePage`) and reverse, with cleanup of references to deleted pages (outline items, link destinations, named destinations, struct-tree `/Pg`, `/OpenAction`). Each op reports the widgets it touches, which [E2r](E2r-planner-page-checks.md)'s checks judge. Duplicating pages is [E2k](E2k-duplicate-pages.md).

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
- Each op reports the fields whose widgets it deleted, rotated or moved on the page, for [E2r](E2r-planner-page-checks.md)'s checks

**Out of scope** (and which todo covers it)

- Page-tree editing, the tree writers, `PageLabels::remap` and the reference index → [E2j](E2j-page-tree-and-tree-writers.md)
- The planner's field-level checks → [E2r](E2r-planner-page-checks.md)
- Duplicating pages (tags and widgets) → [E2k](E2k-duplicate-pages.md)
- Pages from other documents, replace, extract → [E2c](E2c-cross-document-pages.md)
- Merge and split → [E2d](E2d-merge-and-split.md)
- The viewer across page-list changes → [E2l](E2l-viewer-across-page-edits.md); the Organise UI → [E2h](E2h-page-organiser-ui.md)
- The CLI's page commands (`laminapdf pages delete`, `extract`, `reorder`, `insert`, `merge`, `split`) → [T6a](./T6a-cli-page-commands.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.01` insert blank pages; `C.03` delete pages (duplicate in E2k); `C.04` reorder pages; `C.08` rotate pages 90/180/270; `C.13` reverse page order
- **Crates and packages:** `laminapdf-ops`, the façade and bindings; `laminapdf-model`'s editor and writers come from E2j, the planner's checks from E2r
- **`Op` variants:** `InsertBlankPages`, `DeletePages`, `MovePages`, `RotatePages`, `ReversePages` (class `PageStructure`)
- **`SaveConstraint`:** class `PageStructure`; the verdicts per constraint are the [E2 plan §6 table](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint), including the recommended rule, stricter than ISO 32000-2 §12.8 and ADR 0007 §2, that deleting or rotating a page that holds a signed signature's widget is `Restriction::SignedWidget { field }`
- **Web vs desktop:** identical (core); UI in E2h
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.01` → `round-trip` (core, pdf); `C.04` → `round-trip` (core, pdf); `C.08` → `round-trip` (pdf; core is already proved by `RotatePage`); `C.13` → `round-trip` (core, pdf). `C.03` flips in E2k, once duplicate exists too

## Tasks

- [ ] Implement each op over E2j's page-tree editor, with undo records; `MovePages` keeps `PageId`s and object numbers
- [ ] Reference cleanup per the [E2 plan's decision](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions), through E2j's `PageReferences` and name-tree writer, reported in `Applied` and `Edited`
- [ ] `PageLabels::remap` called by every op
- [ ] Each op reports its touched widgets to E2r's checks
- [ ] The refusal code for deleting every page (`codes.json`, `docs/errors/`, `cargo xtask gen codes`)
- [ ] Bindings (`doc.pages.insertBlank()`, `.delete()`, `.move()`, `.rotate()`, `.reverse()`) and `Edited`'s new fields
- [ ] Examples in Rust and TS

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Page count, order and rotation after each op match pypdf and pdf.js on 20 corpus files (incl. deep/unbalanced page trees and inherited attributes)
- [ ] On an approval-signed fixture (the plan's recommended rule, through E2r), `DeletePages` and `RotatePages` on the page holding the signed widget give a `Restriction::SignedWidget` violation and `constraint.kind() == Conflict` (save refused without the override); the same ops on another page are `AppendOnly` and the signature still validates after the incremental save
- [ ] Page labels survive every op: on a fixture with roman + arabic + prefixed ranges, each surviving page's label is unchanged after delete/move/reverse (read back in pdf.js and PDFium)
- [ ] On the FieldMDP-lock fixture, deleting or rotating the page that holds the locked field's widget is a `Restriction::FieldMdp` violation; moving, reversing and inserting are not
- [ ] Deleting a page that an outline item targets follows the [E2 plan §11](../../plans/editor/e2-page-organisation.md#11-decisions-on-the-open-questions) decision and the report lists it
- [ ] Determinism: saving the same state twice gives identical bytes on native and wasm32 (ADR 0007 §6)

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

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
