# E2r — The save planner's field-level checks for page ops

| Field | Value |
| --- | --- |
| ID | `E2r` |
| Status | `ready` |
| Plan | [E2 page organisation §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E2j](../ongoing/E2j-page-tree-and-tree-writers.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e2r-planner-page-checks` |
| PR title | `feat(save): field-level checks for page ops` |
| Size | M (200–400): an index of the pages that hold restricted widgets, two restrictions and the per-field FieldMDP check in `laminapdf-save`, with tests on the signed fixtures; the ops that report their touched widgets come in [E2b](E2b-page-structure-ops.md) |
| PR | — |

## Goal

Let the save planner judge page ops field by field before any page op ships, so deleting or turning the page of a signed or locked field is never saved as a plain incremental update. Split out of [E2b](E2b-page-structure-ops.md) by the [E2 plan](../../plans/editor/e2-page-organisation.md#8-todos), which E2b now waits on.

## Context

- [E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint): the table and the recommended rule for signed widgets and FieldMDP locks
- Today a FieldMDP lock is broken only by `FormFill` (`Rule::allows` in `crates/laminapdf-save/src/detect.rs`), because op classes don't name fields (ADR 0007 §9)
- ISO 32000-2 §12.8.2.2–§12.8.2.4 (DocMDP, UR3, FieldMDP)

## Scope

**In scope**

- `PendingChanges` gains, per transaction, the fields whose widgets an op deleted, rotated or moved on the page, which ops report through a new `Applied` field
- The planner's index of the pages that hold widgets of signed `/FT /Sig` fields and of the fields each lock covers (`/Action` with `/Fields`)
- `Restriction::SignedWidget { field }` and a per-field `Restriction::FieldMdp`, applied by the [E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) rules, and the same verdict from `would_violate(&op)` before an op applies

**Out of scope** (and which todo covers it)

- The page ops, which report their touched widgets → [E2b](E2b-page-structure-ops.md), [E2c](E2c-cross-document-pages.md), [E2f](E2f-page-geometry.md), [E2s](E2s-resize-and-orientation.md)
- The suite of every op on every signed fixture → [E2i](E2i-signed-gating-and-close.md)

**Editor facts**

- **Operations shipped:** none
- **Crates and packages:** `laminapdf-save`, `laminapdf-ops` (`PendingChanges`, `Applied`)
- **`Op` variants:** none
- **`SaveConstraint`:** the [E2 plan §6](../../plans/editor/e2-page-organisation.md#6-saving-and-the-save-constraint) rules for signed widgets and FieldMDP locks; they are a recommendation stricter than ISO 32000-2 §12.8 and ADR 0007 §2, and this PR records them in ADR 0007
- **Web vs desktop:** identical (core)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] `PendingChanges` and `Applied` record touched widgets by field
- [ ] The planner's widget index and the two restrictions
- [ ] `would_violate` for ops that declare touched widgets
- [ ] An ADR 0007 note recording the per-field rule

## Acceptance criteria

- [ ] With a test op that declares a signed widget deleted, the approval-signed fixture's constraint is `Conflict` with `Restriction::SignedWidget`, and the save is refused without the override
- [ ] With a test op that declares a locked field's widget rotated, the FieldMDP fixture reports `Restriction::FieldMdp`; one that declares it moved on the page reports nothing
- [ ] `would_violate` gives the same verdict as the plan, for each case
- [ ] Existing planner tests are unchanged

## Post-merge checks

None.

## Tests

- Unit tests in `laminapdf-save` over `fixtures/signed/`

## Docs to update

- [ADR 0007](../../decisions/0007-save-planner.md): a dated note for the per-field rule
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- The fixtures for a visible signature widget and a FieldMDP lock come from E2i's `fixtures/signed/make.py`; if E2i hasn't added them, this PR adds the two it needs

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
