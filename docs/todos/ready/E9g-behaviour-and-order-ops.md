# E9g — Write format, validation and calculation helpers; calculation and tab order

| Field | Value |
| --- | --- |
| ID | `E9g` |
| Status | `ready` |
| Plan | [E9 plan §2.3](../../plans/editor/e9-form-authoring.md#23-behaviour-calculation-order-and-tab-order) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8f](../done/E8f-declarative-behaviour.md), [E9b](./E9b-field-creation-ops.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md), [0021](../../decisions/0021-pdf-javascript-policy.md) |
| Branch | `feat/e9g-behaviour-and-order-ops` |
| PR title | `feat(forms): write field behaviours, calculation and tab order` |
| Size | L (400–800 of code) |
| PR | — |

## Goal

`SetFieldBehaviour` writes Acrobat's format, validation and calculation helpers as JavaScript actions built from typed values (ADR 0021 §4), so a field formats and totals the same in Lamina PDF and Acrobat. `SetCalculationOrder` sets `/CO`, and `SetTabOrder` sets a page's tab order.

## Context

- [E9 plan §2.3](../../plans/editor/e9-form-authoring.md#23-behaviour-calculation-order-and-tab-order)
- [ADR 0021](../../decisions/0021-pdf-javascript-policy.md) §2 and §4: the helper list and writing helpers as data
- [04 §3.3](../../research/pdf/04-interactive-security.md#33-calculation-order-and-scripts)
- ISO 32000-2 §12.6.3 (`/AA` triggers), §12.6.4 (JavaScript actions), §12.7.3 (`/CO`), §12.5.1 (`/Tabs`); ISO 14289-1 §7.18.3; ISO 19005-2 §6.6

## Scope

**In scope**

- The helper-call writer beside E8f's recogniser, and its write-then-recognise property test
- `SetFieldBehaviour`, `SetCalculationOrder`, `SetTabOrder`
- Codes `tab-order-needs-structure` and `tab-order-invalid`; `pdfa-field-actions` (from E9b) for helpers on PDF/A files

**Out of scope** (and which todo covers it)

- The interface → [E9h](E9h-behaviour-and-tab-order-ui.md)
- Custom tab order in files that claim PDF/UA-1 → E15b (E9 plan §10, question 6)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.07`, `E.08`, `E.09`
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** `SetFieldBehaviour`, `SetCalculationOrder`, `SetTabOrder` (class `Document`)
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `E.07`, `E.08` → `round-trip` (core, pdf) with authoring tests added to E8f's; `E.09` → `round-trip` (core, pdf)

## Tasks

- [ ] Writer and property test
- [ ] `SetFieldBehaviour`
- [ ] `SetCalculationOrder`
- [ ] `SetTabOrder`
- [ ] `behaviours.pdf` fixture
- [ ] Façade, SDK, example (a total field), frozen samples

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is on `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc or TSDoc, a runnable example, tsify types and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports class `Document` and `requires_rewrite` false; a test per op runs `planner.would_violate(&op)` on the signed fixtures (approval, DocMDP P1, P2, P3, `locked.pdf`, `form-locked-include.pdf`, the UR3 files) and asserts [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)'s verdict, because a misclassified op is a security bug
- [ ] Round-trip: open, op, save, reopen in Lamina PDF, pdf.js and PDFium keeps the change and everything else (pages, outline, annotations, fields, structure facts in `expected.json`); `qpdf --check` is clean
- [ ] Every behaviour written is recognised back unchanged (property test and the `af_write` fuzz target)
- [ ] pdf.js formats, validates and calculates the written fields as Lamina PDF does on `behaviours.pdf`

## Post-merge checks

None.

## Tests

- Unit tests per helper and parameter, `/CO` permutations, each `/Tabs` value and refusal
- `af_write` fuzz target
- Displayed values vs pdf.js

## Docs to update

- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Acrobat's exact spelling of each helper call is the reference; record the source of each in the writer's tests

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
