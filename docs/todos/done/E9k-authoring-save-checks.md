# E9k — The save planner's checks of authoring ops

| Field | Value |
| --- | --- |
| ID | `E9k` |
| Status | `done` |
| Plan | [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E9a](./E9a-form-authoring-feature-plan.md) |
| ADRs | Amends [0007](../../decisions/0007-save-planner.md) (§13); follows [0006](../../decisions/0006-op-log-and-undo.md) |
| Branch | `feat/e9k-authoring-save-checks` |
| PR title | `feat(save): check authoring ops against field locks and usage rights` |
| Size | M (200–400 of code; tests beyond) |
| PR | [#754](https://github.com/laudebugs/laminapdf/pull/754) |

## Goal

The save planner judges authoring ops field by field before any of them ships. Today a class `Document` op is judged by class alone, and a field lock allows it, so an op that renamed or deleted a locked field under an approval signature would save. This PR adds `Op::authored_fields()` and `Op::form_right()`, holds each authored field to the locks that cover any name it has or gets and to signed signature fields, reads UR3's `/Form` `/Add` and `/Delete` rights, and records it all as ADR 0007 §13.

## Context

- [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint): the rules and the save-constraint table
- [ADR 0007](../../decisions/0007-save-planner.md) §11 and §12: the field-level checks of page ops and form ops this PR extends
- ISO 32000-2 §12.8.2.2 (DocMDP), §12.8.2.3 and Table 260 (UR3 `/Form` rights), §12.8.2.4 (FieldMDP), §12.7.5.5 (`/Lock`)
- `laminapdf-save`'s `detect.rs` and `planner.rs`; `laminapdf-ops`' `test_support::Declared`, the stand-in E2r used before page ops shipped

## Scope

**In scope**

- `Op::authored_fields()` (the field, when it exists, and every fully qualified name the op gives or takes away) and `Op::form_right()` (`Add`, `Delete`), returning nothing for every op that exists today. A `CreateField` of class `Signature` still reports `Add`: under UR3 it needs `/Form [/Add]` besides the class rule, since Table 260's `/Signature [/Modify]` covers only signing an existing field
- `PendingChanges` keeping the authored fields until the overlay is empty or rebased
- The planner's checks: `Restriction::FieldMdp` for each covering lock (the plan's recommendation: `/All` and `/Exclude` also cover a field created after signing), `Restriction::SignedWidget` for a signed signature field, the UR3 `/Form` rights; `would_violate(&op)` giving the same verdict
- `usage-rights-form-add.pdf` and `usage-rights-form-delete.pdf` in `fixtures/signed/make.py`
- ADR 0007 §13

**Out of scope** (and which todo covers it)

- The ops themselves → [E9b](../ready/E9b-field-creation-ops.md), [E9e](../ready/E9e-field-configuration-ops.md), [E9f](../ready/E9f-signature-fields.md), [E9g](../ready/E9g-behaviour-and-order-ops.md)

**Editor facts**

- **Operations shipped:** none (the checks every authoring op relies on)
- **Crates and packages:** `laminapdf-ops`, `laminapdf-save`, and the WASM record of `Violation`
- **`Op` variants:** none; `Declared` stands in for the authoring ops in tests
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [x] `Op::authored_fields()`, `Op::form_right()` and `Declared`'s fields for them
- [x] `PendingChanges::authored_fields()`
- [x] Planner and `would_violate` checks
- [x] UR3 `/Form` `/Add` and `/Delete` read in `detect.rs`
- [x] The two UR3 fixtures
- [x] ADR 0007 §13

## Acceptance criteria

- [x] For a `Declared` create, rename (old and new names), change and delete, `would_violate` and the save-time plan agree on every signed fixture: approval, DocMDP P1, P2, P3, `locked.pdf`, `form-locked-include.pdf`, a signed signature field, both UR3 files
- [x] A rename into and out of an `/Include` lock's `/Fields` is `Restriction::FieldMdp`; a create under `/All` or `/Exclude` is too
- [x] A `Declared` signature-field create (class `Signature`) is a UR3 violation on a file whose rights lack `/Form [/Add]`
- [x] `SetCalculationOrder`- and `SetTabOrder`-like changes that name no field keep the class rule
- [ ] The WASM modules stay within their ADR 0016 §6 budgets

## Post-merge checks

None.

## Tests

- Unit tests in `laminapdf-save` over the signed fixtures, per rule

## Docs to update

- [ADR 0007](../../decisions/0007-save-planner.md) §13
- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos

## Risks and open questions

- These checks are security-sensitive: review lock coverage of new names under `/Exclude` locks closely

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
