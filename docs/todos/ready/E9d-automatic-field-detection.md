# E9d — Automatic field detection on flat forms (optional)

| Field | Value |
| --- | --- |
| ID | `E9d` |
| Status | `ready` |
| Plan | [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E7c](./E7c-content-ir.md), [E9c](./E9c-form-design-mode.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e9d-automatic-field-detection` |
| PR title | `feat(forms): detect fields on flat forms` |
| Size | M (200–600) |
| PR | — |

## Goal

Suggest fields on flat (non-interactive) forms from ruling lines, boxes, underscores and label text (heuristic per E9a decision); the user accepts/edits suggestions in design mode.

## Context

- [03 §8.4 Table extraction](../../research/pdf/03-text-and-fonts.md#84-table-extraction) (ruling-line detection techniques)
- [full-editor §E9](../../plans/full-editor.md#e9-form-authoring)

## Scope

**In scope**

- Detector over the E7c IR and B9 text
- Suggestion UI
- Precision/recall measurement on a labelled set

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `lamina-content`, `lamina-text`, `@lamina/ui`
- **`Op` variants:** suggestions become `CreateField` ops on accept
- **`SaveConstraint`:** `Free` → incremental. Signed: creating/changing field definitions is not form filling (class `Document`), so it is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → saved only after the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; changing or removing a field locked by FieldMDP/`/Lock`, or a signature field, is a violation
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none (assists E.01–E.03 authoring; no new inventory row)

## Tasks

- [ ] Detector
- [ ] UI
- [ ] Labelled set
- [ ] Metrics

## Acceptance criteria

- [ ] Precision ≥ target and recall ≥ target set in E9a on the labelled set
- [ ] No suggestion is applied without user action

## Post-merge checks

None.

## Tests

- Labelled fixture set

## Docs to update

- User help
- [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Open question from E9a: heuristic vs ML; ML must be licence- and size-checked

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
