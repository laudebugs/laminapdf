# E9e — Change, rename and move form fields, widgets and options

| Field | Value |
| --- | --- |
| ID | `E9e` |
| Status | `ready` |
| Plan | [E9 plan §2.1](../../plans/editor/e9-form-authoring.md#21-field-ops) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E9b](./E9b-field-creation-ops.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e9e-field-configuration-ops` |
| PR title | `feat(ops): change, rename and move form fields` |
| Size | L (400–800 of code) |
| PR | — |

## Goal

The ops that make a created field configurable: `UpdateFieldDef` (label, flags, appearance, options, maximum length, default value, caption and action), `RenameField`, `MoveWidget`, and `AddWidget` and `RemoveWidget` for radio options and fields shown on several pages. With them, text fields, check boxes, radio groups, choices and push buttons are authored end to end in the library.

## Context

- **Design (Paper):** [P05 Form design mode](../../design/paper/forms.md#p05-form-design-mode). Build to match it; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E9 plan §2.1 and §5](../../plans/editor/e9-form-authoring.md#21-field-ops)
- ISO 32000-2 §12.7.4.2 (field names), §12.7.5.2.4 (`/Opt` on buttons), §12.7.5.4 (choice options)

## Scope

**In scope**

- `UpdateFieldDef` (every entry but `lock` and `seed`), `RenameField` (moving the field between parents), `MoveWidget`, `AddWidget`, `RemoveWidget`
- Appearances redrawn for every change that shows
- Each op in `Op::authored_fields()`, old and new names included

**Out of scope** (and which todo covers it)

- `lock` and `seed` → [E9f](E9f-signature-fields.md)
- The interface → [E9c](E9c-form-design-mode.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01`, `E.02`, `E.03`, `E.04`
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf-build`, `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** `UpdateFieldDef`, `RenameField`, `MoveWidget`, `AddWidget`, `RemoveWidget` (class `Document`)
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `E.01`, `E.02`, `E.03`, `E.04` → `round-trip` (core, pdf)

## Tasks

- [ ] `UpdateFieldDef`
- [ ] `RenameField`
- [ ] `MoveWidget`, `AddWidget`, `RemoveWidget`
- [ ] Planner verdicts per op
- [ ] Façade, SDK, frozen samples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is on `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc or TSDoc, a runnable example, tsify types and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports class `Document` and `requires_rewrite` false; a test per op runs `planner.would_violate(&op)` on the signed fixtures (approval, DocMDP P1, P2, P3, `locked.pdf`, `form-locked-include.pdf`, the UR3 files) and asserts [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)'s verdict, because a misclassified op is a security bug
- [ ] Round-trip: open, op, save, reopen in Lamina PDF, pdf.js and PDFium keeps the change and everything else (pages, outline, annotations, fields, structure facts in `expected.json`); `qpdf --check` is clean
- [ ] Renaming a field into or out of a lock's `/Fields` is `Restriction::FieldMdp`; moving a signed signature's widget is `Restriction::SignedWidget`

## Post-merge checks

None.

## Tests

- Unit tests per entry set and removed; renames across parents; radio options added and removed
- Goldens on `nested-names.pdf`
- `ops_json`

## Docs to update

- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
