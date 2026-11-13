# E2j — Page-tree editing, tree writers, page-label remapping and the page reference index

| Field | Value |
| --- | --- |
| ID | `E2j` |
| Status | `done` |
| Plan | [E2 page organisation §4](../../plans/editor/e2-page-organisation.md#4-crate-and-package-changes) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [B8a](./B8a-structure-tree.md), [E2a](./E2a-page-organisation-feature-plan.md) |
| ADRs | Follows [0003](../../decisions/0003-reader-tolerance-and-limits.md), [0005](../../decisions/0005-document-model.md) |
| Branch | `feat/e2j-page-tree-and-tree-writers` |
| PR title | `feat(model): page-tree editor, name and number tree writers, page reference index` |
| Size | L (400–800): the page-tree editor, two tree writers, `PageLabels::remap` and the reference index in `laminapdf-model`, with fixtures; no op |
| PR | [#493](https://github.com/laudebugs/laminapdf/pull/493) |

## Goal

Give `laminapdf-model` the write side that every E2 page op needs, so the ops themselves stay small: a page-tree editor, writers for number trees and name trees, `PageLabels::remap`, and an index of everything that points at a page. Split out of [E2b](../done/E2b-page-structure-ops.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos) to keep each PR reviewable; [E2b](../done/E2b-page-structure-ops.md), [E2c](../ready/E2c-cross-document-pages.md), [E2d](../ready/E2d-merge-and-split.md), [E2f](../ready/E2f-page-geometry.md), [E2g](../ready/E2g-page-labels.md), [E3b](../ready/E3b-bookmarks-and-destinations.md) and [E3f](../ready/E3f-attachments-and-associated-files.md) build on it instead of writing their own.

## Context

- [E2 plan §2.2 and §4](../../plans/editor/e2-page-organisation.md#22-what-follows-a-page): what follows a page and where each piece lives
- [01 §10 Document logical structure](../../research/pdf/01-file-structure.md#10-document-logical-structure-77): the page tree and inheritance
- ISO 32000-2 §7.7.3.2 (page tree nodes, `/Count`), §7.7.3.4 (inheritance of page attributes), §7.9.6 (name trees), §7.9.7 (number trees), §12.3.2 (destinations), §12.4.2 (page labels), §14.7 (logical structure)
- `crates/laminapdf-model/src/{page.rs,locate.rs,tree.rs,labels.rs,outline.rs,link.rs,structure.rs}`: the read side it extends; `RotatePage`'s inherited-`/Rotate` handling in `crates/laminapdf-ops/src/rotate.rs`

## Scope

**In scope**

- **Page-tree editor** over `CosMut`: insert, remove and move kids by `PageId` and `PagePosition`; materialise inherited `/Resources`, `/MediaBox`, `/CropBox` and `/Rotate` on a page before it leaves its parent; keep every `/Count` on the path; keep the tree's shape for small edits and rebuild it balanced only past a depth or fan-out threshold; copy a page dictionary listed twice in `/Kids` on its first change, with a warning
- **Number-tree writer** (`/PageLabels`, `/ParentTree`) and **name-tree writer** (`/Dests`, `/EmbeddedFiles`, …): balanced, keys sorted (byte order for names), a bounded leaf size, `/Limits` kept right
- **`PageLabels::remap`**: rebuilds `/PageLabels` from page IDs after a change to the page list, so each surviving page keeps its label and new pages continue the range they land in
- **`PageReferences`**: for a set of pages, every outline item, link annotation, named destination (name tree and PDF 1.1 `/Dests` dictionary), `GoTo` action, `/OpenAction`, structure element (by `/Pg`) and field widget (by `/P`, else the page's `/Annots`) that points at them, as typed entries the ops can retarget or remove

**Out of scope** (and which todo covers it)

- The ops that use them → [E2b](../done/E2b-page-structure-ops.md), [E2k](../ready/E2k-duplicate-pages.md)
- Copying objects between documents → [E2c](../ready/E2c-cross-document-pages.md)

**Editor facts**

- **Operations shipped:** none (foundation for `C.01`–`C.16`, `I.10`)
- **Crates and packages:** `laminapdf-model`
- **`Op` variants:** none
- **`SaveConstraint`:** not applicable (no op)
- **Web vs desktop:** identical (core)
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [x] Page-tree editor with inherited-attribute materialisation, `/Count` maintenance, rebalancing past a threshold and the shared-dictionary copy
- [x] Number-tree and name-tree writers
- [x] `PageLabels::remap`, public for the ops
- [x] `PageReferences` over outlines, links, named destinations, actions, `/OpenAction`, structure elements and widgets
- [x] `fixtures/pages/` and its `make.py` (no dependencies), with provenance in `fixtures/README.md`: `inherited.pdf`, `unbalanced.pdf`, `references.pdf`, `labels.pdf`, `boxes.pdf`, `forms.pdf`, `layers.pdf`, `chapters.pdf` ([E2 plan §9](../../plans/editor/e2-page-organisation.md#9-fixtures))

## Acceptance criteria

- [x] Property tests: random insert, remove and move sequences on `inherited.pdf` and `unbalanced.pdf` give trees whose page order, `/Count`s and effective attributes match a flat model of the same edits; qpdf `--check` is clean on the saved results
- [x] Property tests for both tree writers: random inserts and deletes give valid trees (sorted keys, right `/Limits`, bounded leaves) that qpdf and pdf.js read back with the same entries
- [x] `PageLabels::remap` on `labels.pdf` keeps every surviving page's label after random moves, deletes and inserts (unit test against `PageLabels::label`)
- [x] `PageReferences` finds each of `references.pdf`'s references to page 3, and nothing else
- [x] No new size-lint `#[expect]`; every module under 800 lines (`cargo xtask check maintainability`)

## Post-merge checks

None.

## Tests

- Property and unit tests in `crates/laminapdf-model/tests/page_tree.rs` and `tests/trees.rs`
- qpdf and pdf.js read-back of the writers' trees

## Docs to update

- rustdoc for the editor, writers and index
- `CLAUDE.md`'s `laminapdf-model` line (now writes trees)
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and the [E2 plan §8](../../plans/editor/e2-page-organisation.md#8-todos): point this todo's link at `done/`

## Risks and open questions

- The rebalance threshold: pick it from the corpus's deepest and widest trees and note it in rustdoc

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **The editor** is `Document::edit_page_tree()` → `PageTreeEditor` (`src/page_tree/{mod,load,write}.rs`):
  - It edits an in-memory copy of the tree: `insert`, `insert_new`, `remove` and `move_pages`, by `PageId` and the new `PagePosition` (`Start`, `End`, `Before`, `After`).
  - `finish` writes everything after a journal mark and rolls back on error. Dropping the editor writes nothing, though `insert_new` has already used up its reserved object number.
  - `insert` takes only a `/Type /Page` dictionary, or one without `/Type` that has no `/Kids` and has a `/MediaBox` or `/Contents`. Errors are `PageTreeError` (no codes yet; the ops map them).
  - **Inherited attributes:** a moved page keeps the attributes it showed. A value its new ancestors would give it but it never had is overridden with a neutral one: `/Rotate 0`, empty `/Resources`, or a `/CropBox` equal to the media box.
  - **Rebalancing:** past `MAX_DEPTH` (32) or `MAX_KIDS` (1,024 kids on a touched node), the whole tree is rebuilt with `FAN_OUT` 32. The root keeps its object number. The thresholds come from reasoning in rustdoc, not measurement: the corpus is too small to measure them.
  - **A page listed twice** is copied by the first `finish`, without `/Annots` or `/StructParents`, so the two pages share no annotations or structure entry. It raises the existing `invalid-entry` warning on `Kids`; no new code was needed.
  - **A subtree past `max_page_tree_depth`** keeps its stored `/Count`.
- **`TreeWriter`** (`src/tree_writer.rs`) writes a fresh, balanced name tree or number tree. Leaves differ in size by at most one; there are 64 keys per leaf by default (`with_max_kids`). `/Limits` is on every node but the root.
- **`PageLabels::remap(before, after)` and `PageLabels::write`** (`src/labels.rs`):
  - A new page continues the label of the page before it.
  - New pages at the very start lead into the first range.
  - An old page that had no label gets the decimal number a viewer showed for it.
- **`PageReferences`** (`src/references.rs`): every outline item, annotation with a destination or `GoTo` action, named destination (name tree and `/Dests`), `/OpenAction` and structure element by `/Pg` that points at the pages. It also finds `MCR`/`OBJR` kids with their own `/Pg` (`StructContentRef`) and field widgets (by `/P`, else `/Annots`), classifying each kid of a field on its own.
  - Not followed: `GoTo` actions in `/Next` chains and `/AA`, and annotations written directly in `/Annots`.
- **Fixtures:** `fixtures/pages/make.py` writes all eight files byte for byte. The tests use `inherited`, `unbalanced`, `references` and `labels`; `boxes`, `forms`, `layers` and `chapters` wait for E2b–E2f.
- **Read-back:** qpdf reads back the page tree (`--check`) and a 300-range `/PageLabels` tree (`--json`). qpdf's JSON has no list of named destinations, so the name tree is checked by the model's own reader only. No pdf.js read-back: no Rust test runs Node.
- **Not tested:** the rollback in `finish` when a write fails partway, since a test can't make one fail.
- **Behind the `edit` feature:** the write side (the editor, `TreeWriter`, `PageLabels::remap`/`write`, `PageReferences`) is compiled only with `laminapdf-model`'s `edit` feature, on by default in the crate. The workspace dependency turns default features off, so the façade and the WASM modules compile none of it. Compiled at all, even uncalled, it grew the `extract` module by about 1.2 KB gzip, past its budget, because it changes how shared read code is inlined. The page ops (E2b onwards) turn `edit` on, and the WASM budget question returns with them.
- **Reviews:** `/code-review high` found nine problems, all fixed in 7f1ac77e with regression tests. A second `/code-review low` found none. `/writing-review` found two problems in the crate README, both fixed.
