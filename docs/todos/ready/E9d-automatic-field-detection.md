# E9d — Suggest fields on flat forms

| Field | Value |
| --- | --- |
| ID | `E9d` |
| Status | `ready` |
| Plan | [E9 plan §2.5](../../plans/editor/e9-form-authoring.md#25-automatic-field-detection) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E7c](../done/E7c-content-ir.md), [E8g](./E8g-form-data-and-flatten.md), [E9a](../done/E9a-form-authoring-feature-plan.md) |
| ADRs | Follows [0030](../../decisions/0030-extraction-output-and-model-plugins.md) |
| Branch | `feat/e9d-automatic-field-detection` |
| PR title | `feat(extract): suggest fields on flat forms` |
| Size | M (200–400 of code; the labelled set beyond) |
| PR | — |

## Goal

`suggest_fields(page)` in `laminapdf-extract` suggests fields on a flat form from ruling lines, boxes, underscores and label text, with rules only (E9 plan §10, question 1). The labelled set measures it, so the round knows how good the suggestions are before any screen shows them.

## Context

- [E9 plan §2.5 and §10](../../plans/editor/e9-form-authoring.md#25-automatic-field-detection)
- [03 §8.4 Table extraction](../../research/pdf/03-text-and-fonts.md#84-table-extraction): ruling-line techniques
- [ADR 0030](../../decisions/0030-extraction-output-and-model-plugins.md) §6: rules in the core, models as plug-ins
- `laminapdf-extract`'s `tables/edges.rs`: the ruling lines to reuse

## Scope

**In scope**

- `suggest_fields` and `FieldSuggestion` (kind, box, label, reason) in a new `forms` module
- `fixtures/forms/flat/` from `tools/fixtures/flat_forms.py`, with provenance
- Precision and recall, reported by a test

**Out of scope** (and which todo covers it)

- The review interface → [E9i](E9i-field-detection-review-ui.md)
- A model → an ADR 0030 amendment, only if the rules fall short (E9 plan §10, question 1)

**Editor facts**

- **Operations shipped:** none (assists E.01–E.03 authoring)
- **Crates and packages:** `laminapdf-extract`, `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** none; suggestions become `CreateField` when accepted
- **`SaveConstraint`:** none: it changes nothing
- **Web vs desktop:** identical
- **Server features:** none: detection runs on the device
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Detector
- [ ] Labelled set
- [ ] Metrics test
- [ ] Façade and SDK (`suggest_fields`, `suggestFields`)

## Acceptance criteria

- [ ] Precision ≥ 0.85 and recall ≥ 0.70 on `fixtures/forms/flat/`, a suggestion counting as right when its kind matches and its box overlaps the original field's with an intersection over union ≥ 0.5
- [ ] Areas existing widgets cover get no suggestion
- [ ] The extract module stays within its ADR 0016 §6 budget

## Post-merge checks

None.

## Tests

- Unit tests on synthetic pages
- The metrics test over the labelled set
- A `field_suggest` fuzz target

## Docs to update

- API docs: suggesting fields
- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos

## Risks and open questions

- If the rules measure below the targets, record the numbers and the failing cases in the E9 plan §10 before deciding on a model

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
