# E7b — Editing existing content: feature plan

| Field | Value |
| --- | --- |
| ID | `E7b` |
| Status | `done` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E6a](./E6a-adding-content-feature-plan.md), [E7a](./E7a-adr-0022-content-editing.md) |
| ADRs | Follows ADR 0005, 0006, 0007 |
| Branch | `docs/e7b-editing-existing-content-feature-plan` |
| PR title | `docs(editor): E7 feature plan (editing existing content)` |
| Size | S (< 200 lines) |
| PR | [#564](https://github.com/laudebugs/laminapdf/pull/564) |

## Goal

The short feature-plan PR that opens round E7 ([full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content)): selecting and editing existing text, images and vector objects using the ADR 0022 IR, font handling, search and replace, stacking, clipping, struct-tree maintenance and an edit corpus. It fixes the op list, test plan, fixtures and PR stack before any code, as every editor round requires ([full-editor §7](../../plans/full-editor.md#7-phases-and-rounds): "each starts with a short feature-plan PR").

## Context

- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content): the round as planned (ships, crates, save, web vs desktop)
- [06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) and [06 §7](../../research/pdf/06-feature-inventory.md#7-difficulty-map): mechanism and difficulty of each operation in scope
- [full-editor §3](../../plans/full-editor.md#3-tracking-the-capability-matrix) and [§5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round): capability-matrix statuses and the requirements every round meets
- [ADR 0006](../../decisions/0006-op-log-and-undo.md) §1 and [ADR 0007](../../decisions/0007-save-planner.md) §2: how each op is classified and how the planner gates it
- [ADR 0018](../../decisions/0018-test-strategy.md): test tiers the plan must assign (golden, corpus, differential, fuzz)

## Scope

**In scope**

- The feature plan `docs/plans/editor/e7-editing-existing-content.md`
- Capability-manifest rows for the round's operations
- Adjustments to the E7 implementation todos (split, merge, dependency fixes)

**Out of scope** (and which todo covers it)

- Any code: each implementation todo of the round

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `A.02` edit existing text; `A.03` delete or replace text; `A.04` change font family/size/colour/style; `A.05` bold, italic, underline, super/subscript; `A.06` alignment and line spacing; `A.07` move/resize/rotate text blocks; `A.08` search and replace throughout the document; `B.01` insert, delete, replace images; `B.02` resize, rotate, crop, reposition; `B.07` edit vector paths, fills, strokes; `B.08` change stacking order; `B.11` clipping paths and masks; `B.12` transformations on graphical objects
- **Crates and packages:** (planned for the round) ADR 0022 and the `laminapdf-content` IR with identity and encoder; `laminapdf-text` re-layout and subsetting
- **`SaveConstraint`:** (round summary from the plan; the feature plan details it per op) incremental on `Free`; struct tree and MCIDs maintained; PDF/A/UA claims re-checked on save
- **Web vs desktop:** identical; desktop font substitution can use system fonts
- **Server features:** none
- **Capability-matrix rows to flip:** none flipped here; rows added with status `unsupported` and the round as target

## Tasks

- [x] Create `docs/plans/editor/e7-editing-existing-content.md` (Status: Draft) with: goal; the operations in scope by inventory ID (`A.02`, `A.03`, `A.04`, `A.05`, `A.06`, `A.07`, `A.08`, `B.01`, `B.02`, `B.07`, `B.08`, `B.11`, `B.12`) and the target status of each; for each op its `Op` variant name, payload sketch (by `LaminaPdfId`, no ambient inputs), `ModificationClass` and `requires_rewrite`
- [x] Describe model and crate changes (new typed views, new modules, public API in `laminapdf` and `@laminapdf/sdk`), and the UI tools/panels in `@laminapdf/ui`
- [x] Write the `SaveConstraint` table for the round: behaviour on `Free`, `AppendOnly` (DocMDP P1/P2/P3, FieldMDP), `RequiresRewrite` and `Conflict`, and the UI warnings
- [x] List web vs desktop differences through `Platform` capability flags, and any server feature (off by default, opt-in, what leaves the device)
- [x] List fixtures to add (with provenance and licence in `fixtures/README.md`), and the test plan per op: unit, golden, round-trip (lamina, pdf.js, PDFium, qpdf), differential, fuzz, Playwright and WebdriverIO e2e
- [x] Add the round's rows to `docs/capabilities/manifest.toml` with `target round = "E7"` and status `unsupported` until each implementation PR flips them
- [x] Confirm or adjust the round's implementation todos in `docs/todos/ready/` (E7c onwards): split or merge them so each stays one reviewable PR (≤ ~1,500 lines), and keep their `Depends on` explicit
- [x] Answer or assign an owner to each open question below; record decisions in the feature plan
- [x] Index the new plan in `docs/plans/README.md` and link it from [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content)

## Acceptance criteria

- [x] The plan lists every operation in scope by inventory ID with an `Op` name, class and target status
- [x] The `SaveConstraint` table covers Free, AppendOnly (P1/P2/P3), RequiresRewrite and Conflict
- [x] Every implementation todo of the round is linked from the plan and each has explicit dependencies
- [x] `docs/capabilities/manifest.toml` has a row per op with `target round = "E7"`; `cargo xtask capabilities` regenerates without error
- [x] Each open question below is answered or has a named owner and a deadline round

## Post-merge checks

None.

## Tests

Docs only: the link checker passes; `cargo xtask capabilities` (from `X2`) accepts the new manifest rows.

## Docs to update

- `docs/plans/README.md` index
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Answered: line-level only for E7, decided by the owner on 2026-11-13 ([E7 plan §10](../../plans/editor/e7-editing-existing-content.md#10-decisions-on-the-open-questions), question 1)
- Answered: the edit corpus and its thresholds ([E7 plan §10](../../plans/editor/e7-editing-existing-content.md#10-decisions-on-the-open-questions), question 3)
- The plan location `docs/plans/editor/` is a recommendation; if the plans README prescribes another place, use it and update this todo's links

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **The plan** is [docs/plans/editor/e7-editing-existing-content.md](../../plans/editor/e7-editing-existing-content.md). The owner answered its two open questions on 2026-11-13: line-level text editing for this round, and no "keep the claim" option.
- **Todos:** E7d and E7e handed their reader UI to two new todos, E7k (A05) and E7l (A04), so the core waits only on E7c. E7i gained `RemoveConformanceClaims`, since the claim properties are read-only to every other op.
- **Matrix:** B.01 and B.02 moved to round E7 at `round-trip`; A.06 targets `write`, since positioned glyphs carry no alignment to read back.
- **Size:** about 400 lines rather than the estimated < 200, mostly the two new todos.
