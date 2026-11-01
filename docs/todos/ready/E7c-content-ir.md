# E7c — Content IR with object identity and encoder

| Field | Value |
| --- | --- |
| ID | `E7c` |
| Status | `ready` |
| Plan | [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) |
| Round / milestone | Editor phase 4 (E6–E7) — M7 Content editor |
| Depends on | [E7b](./E7b-editing-existing-content-feature-plan.md) |
| Blocks | [E7d](./E7d-select-transform-existing.md), [E7e](./E7e-text-runs-and-inline-edit.md), [E9d](./E9d-automatic-field-detection.md), [E11d](./E11d-sanitiser.md), [E12a](./E12a-redaction-feature-plan.md), [E12c](./E12c-apply-content-removal.md), [E16d](./E16d-colour.md) |
| ADRs | Implements ADR 0022; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e7c-content-ir` |
| PR title | `feat(content): editable content IR with object identity` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Implement the ADR 0022 IR in `laminapdf-content`: parse a page's content (and Form XObjects) into objects with stable identity, bounding boxes and graphics state, hit-testing, and an encoder that re-emits unmodified spans byte-identically and modified ones canonically.

## Context

- ADR 0022 (E7a)
- [library B9](../../plans/pdf-library.md#b9-text-extraction) interpreter
- [02 §4 The graphics state](../../research/pdf/02-imaging-model.md#4-the-graphics-state)

## Scope

**In scope**

- IR types (`ContentObject` with `ObjId`, kind, bbox, gstate snapshot, source span)
- Parser from B9 interpreter events
- Encoder with span preservation
- `hit_test(page, point|rect) -> Vec<ObjId>`
- Limits and fuzzing

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `laminapdf-content`
- **`Op` variants:** none yet (read + encode)
- **`SaveConstraint`:** n/a
- **Web vs desktop:** identical (core)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Types
- [ ] Parser
- [ ] Encoder
- [ ] Hit-testing
- [ ] Fuzz target

## Acceptance criteria

- [ ] Parse → encode of every corpus page renders identically (PDFium diff zero) and is byte-identical for ≥ the rate set in ADR 0022
- [ ] Fuzz 10 min without panic
- [ ] Parse time within the ADR's budget

## Post-merge checks

None.

## Tests

- Corpus round-trip
- Fuzz
- Bench

## Docs to update

- rustdoc
- [full-editor §E7](../../plans/full-editor.md#e7-editing-existing-content) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
