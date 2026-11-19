# E9b — Create and delete form fields

| Field | Value |
| --- | --- |
| ID | `E9b` |
| Status | `ready` |
| Plan | [E9 plan §2.1](../../plans/editor/e9-form-authoring.md#21-field-ops) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8d](../done/E8d-field-appearances.md), [E9k](../ongoing/E9k-authoring-save-checks.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e9b-field-creation-ops` |
| PR title | `feat(ops): create and delete form fields` |
| Size | L (400–800 of code; tests and goldens beyond) |
| PR | — |

## Goal

`CreateField` and `DeleteField` for text fields, check boxes, radio groups, combo boxes, list boxes and push buttons, each with its widgets, `/DA`, `/DR` and appearances, under a unique fully qualified name. E9k's planner checks land first, so these ops are held to field locks from their first commit.

## Context

- **Design (Paper):** [P05 Form design mode](../../design/paper/forms.md#p05-form-design-mode). Build to match it; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E9 plan §2.1, §2.4 and §5](../../plans/editor/e9-form-authoring.md#21-field-ops)
- [04 §3.1](../../research/pdf/04-interactive-security.md#31-acroform-structure-127-interactive-form-dictionary-1273-17-1272) and [04 §3.2](../../research/pdf/04-interactive-security.md#32-field-types-and-flags): the AcroForm structure and field flags
- ISO 32000-2 §12.7.3 (the interactive form dictionary), §12.7.4 (fields, names, `/DA`), §12.7.5 (field types), §14.8.4 and ISO 14289-1 §7.18.4 (the `Form` element)
- [ADR 0007](../../decisions/0007-save-planner.md) §13 (from E9k): the checks these ops report to through `Op::authored_fields()`
- ISO 19005-2 §6.6.1: ResetForm is a forbidden action type on PDF/A files; SubmitForm isn't

## Scope

**In scope**

- `CreateField` (every `FieldSpec` but `Signature`) and `DeleteField` in `laminapdf-ops`' `forms/`; `Applied::created_fields`
- `FieldSpec`, `WidgetSpec`, `TextStyle`, `ButtonAction` and the name rules in `laminapdf-model`'s `forms/`; codes `field-name-taken` and `field-name-invalid`
- Push button captions in `laminapdf-build`'s `annot/appearance/field.rs`
- `Form` elements for new fields in tagged files (`forms/tags.rs`, over the shared parent-tree code); `field-label-missing` in the save's PDF/UA-1 check
- `Op::authored_fields()` and `Op::form_right()` for the two ops, as E9k defines them
- Code `pdfa-field-actions` with its `docs/errors` page: a push button's ResetForm refused on a PDF/A file
- pyHanko's difference analysis of `CreateField` on `approval.pdf`, recorded in the E9 plan §10, question 2

**Out of scope** (and which todo covers it)

- Changing, renaming and moving fields, widgets and options → [E9e](E9e-field-configuration-ops.md)
- Signature fields → [E9f](E9f-signature-fields.md)
- Behaviours, calculation and tab order → [E9g](E9g-behaviour-and-order-ops.md)
- The interface → [E9c](E9c-form-design-mode.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01`, `E.02`, `E.03`, `E.04` (created; configured in E9e), `L.06` (`/TU` and `Form` elements on new fields)
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf-build`, `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** `CreateField`, `DeleteField` (class `Document`)
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** none; tests are tagged `E.01`–`E.04` and `L.06` at `write` (core, pdf), and E9e flips E.01–E.04

## Tasks

- [ ] Model types and name rules
- [ ] `CreateField` with `/AcroForm` creation, merged and separate widgets, on-state names and `/Opt`
- [ ] Push button captions
- [ ] `DeleteField`
- [ ] Tagging and `field-label-missing`
- [ ] `Op::authored_fields()`, `Op::form_right()` and `pdfa-field-actions`
- [ ] Façade, SDK, example (a sign-up form on an existing page), frozen samples
- [ ] pyHanko check

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is on `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc or TSDoc, a runnable example, tsify types and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports class `Document` and `requires_rewrite` false; a test per op runs `planner.would_violate(&op)` on the signed fixtures (approval, DocMDP P1, P2, P3, `locked.pdf`, `form-locked-include.pdf`, the UR3 files) and asserts [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)'s verdict, because a misclassified op is a security bug
- [ ] Round-trip: open, op, save, reopen in Lamina PDF, pdf.js and PDFium keeps the change and everything else (pages, outline, annotations, fields, structure facts in `expected.json`); `qpdf --check` is clean
- [ ] pdf.js and PDFium fill every created field kind through their form APIs, and the filled value reopens in Lamina PDF
- [ ] Renaming or deleting is not possible yet, but a `CreateField` whose name a lock covers (`/All`, `/Exclude`, `/Include`) is `Restriction::FieldMdp` on `locked.pdf` and `form-locked-include.pdf`
- [ ] veraPDF reports nothing new on `tagged-form.pdf` after `CreateField`
- [ ] A manual Acrobat Reader fill of each field kind, recorded in the PR

## Post-merge checks

None.

## Tests

- Unit tests per kind, names and refusals
- Byte goldens in `fixtures/golden/forms/authoring/`
- `ops_json` fuzz target gains the ops
- E8o's differential over the authored files

## Docs to update

- API docs: creating forms
- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- If the PR passes ~1,500 changed lines, move tagging into its own todo before review

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
