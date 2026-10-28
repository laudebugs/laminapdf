# E2b — Page-structure ops: insert blank, delete, duplicate, reorder, rotate, reverse

| Field | Value |
| --- | --- |
| ID | `E2b` |
| Status | `ready` |
| Plan | [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1c](./E1c-metadata-ops.md), [E2a](./E2a-page-organisation-feature-plan.md) |
| Blocks | [E2c](./E2c-cross-document-pages.md), [E2f](./E2f-page-geometry.md), [E2g](./E2g-page-labels.md), [E3b](./E3b-bookmarks-and-destinations.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2b-page-structure-ops` |
| PR title | `feat(ops): page structure ops with reference cleanup` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Implement the single-document page ops as view-level mapping of `PageId`s (no re-parse of content): insert blank pages, delete, duplicate, move (reorder), rotate (batch, generalising B3's `RotatePage`) and reverse, with cleanup of references to deleted pages (outline items, link destinations, named destinations, struct-tree `/Pg`, `/OpenAction`).

## Context

- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation)
- [ADR 0005](../../decisions/0005-document-model.md) §5: `MovePages` keeps every `PageId`; new pages get new IDs
- [06 §3 group C](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms): mechanisms (`/Kids`, `/Count`, inherited attributes)
- [01 §10 Document logical structure](../../research/pdf/01-file-structure.md#10-document-logical-structure-77): page tree and inheritance
- ISO 32000-2 §7.7.3 (page tree, inheritable attributes), §12.3.2 (destinations)

## Scope

**In scope**

- Ops: `InsertBlankPages { at, count, size: PageSize, rotate }`, `DeletePages { pages }`, `DuplicatePages { pages, at }`, `MovePages { pages, to }`, `RotatePages { pages, rotate }`, `ReversePages { range }`
- Page-tree editing that materialises inherited attributes (`/Resources`, `/MediaBox`, `/CropBox`, `/Rotate`) before moving a page out of its parent
- Rebalancing: keep the tree shape for small edits; flatten-and-rebuild only when depth or fan-out passes a threshold
- Reference cleanup on delete according to the E2a decision, with an `OpReport` listing affected outline items/links/destinations
- Duplicate copies the page dict and annotations (new `/NM`, new IDs), sharing content streams and resources

**Out of scope** (and which todo covers it)

- Pages from other documents, replace, extract → [E2c](E2c-cross-document-pages.md)
- Merge and split → [E2d](E2d-merge-and-split.md)
- Thumbnail UI → [E2h](E2h-page-organiser-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `C.01` insert blank pages; `C.03` delete and duplicate pages; `C.04` reorder pages; `C.08` rotate pages 90/180/270; `C.13` reverse page order
- **Crates and packages:** `lamina-model` (page tree mutation helpers), `lamina-ops`, bindings
- **`Op` variants:** `InsertBlankPages`, `DeletePages`, `DuplicatePages`, `MovePages`, `RotatePages`, `ReversePages` (class `PageStructure`)
- **`SaveConstraint`:** `Free` → incremental. Signed: page insertion/deletion/reordering is class `PageStructure`, a violation under DocMDP P1/P2/P3 and of approval signatures' coverage; blocked by default and allowed only by the explicit invalidation override (warning names signatures)
- **Web vs desktop:** identical (core); UI in E2h
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `C.01` → `round-trip` (core, pdf); `C.03` → `round-trip` (core, pdf); `C.04` → `round-trip` (core, pdf); `C.08` → `round-trip` (core, pdf); `C.13` → `round-trip` (core, pdf)

## Tasks

- [ ] Implement page-tree mutation in `lamina-model` with inherited-attribute materialisation and `/Count` maintenance
- [ ] Implement each op with undo records; `MovePages` must keep `PageId`s and object numbers
- [ ] Reference index: find outline items, link annotations, named destinations, struct elements and open action pointing at a page
- [ ] Cleanup policy per E2a, reported in `Applied`
- [ ] Bindings (`doc.pages.insertBlank()`, `.delete()`, `.duplicate()`, `.move()`, `.rotate()`, `.reverse()`)
- [ ] Examples in Rust and TS

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Page count, order and rotation after each op match pypdf and pdf.js on 20 corpus files (incl. deep/unbalanced page trees and inherited attributes)
- [ ] Deleting a page that an outline item targets follows the E2a policy and the report lists it
- [ ] Determinism: saving the same state twice gives identical bytes on native and wasm32 (ADR 0007 §6)

## Tests

- Unit/property tests (random sequences of page ops then undo all → empty overlay)
- Corpus differential vs pypdf (page order facts) and pdf.js (render hashes per page)
- Fixtures: inherited `/Resources` and `/MediaBox`, 3-level page tree, page referenced by outline + named dest + link

## Docs to update

- API docs: pages
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- Shared page dicts (same object referenced twice in `/Kids`) are invalid but occur; duplicate them on first mutation and warn

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
