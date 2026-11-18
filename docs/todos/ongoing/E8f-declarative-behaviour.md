# E8f — Native formatting, validation and calculation (ADR 0021)

| Field | Value |
| --- | --- |
| ID | `E8f` |
| Status | `ongoing` |
| Plan | [E8 plan §2.3](../../plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8a](../done/E8a-adr-0021-javascript-policy.md), [E8d](../done/E8d-field-appearances.md) |
| ADRs | Implements [0021](../../decisions/0021-pdf-javascript-policy.md); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (a new limit), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e8f-declarative-behaviour` |
| PR title | `feat(forms): native formatting, validation and calculation` |
| Size | L (400–800 of code; tests and fixtures beyond) |
| PR | — |

## Goal

Forms behave in the library as they do in Acrobat, without running JavaScript. The recogniser reads each `/AA` script as tokens and matches it against ADR 0021 §2's helper calls; native code formats, filters, validates and calculates in `/CO` order. `SetFieldHidden` does what a Hide action does to a field. Every script that isn't a helper call is listed, and the corpus run measures how many are.

## Context

- [ADR 0021](../../decisions/0021-pdf-javascript-policy.md) §2, §3 and its security notes
- [E8 plan §2.3 and §5](../../plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit), decision 3 in [§10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions)
- [04 §3.3](../../research/pdf/04-interactive-security.md#33-calculation-order-and-scripts), [04 §4](../../research/pdf/04-interactive-security.md#4-javascript-in-pdf)
- ISO 32000-2 §12.6.3 (trigger events), §12.6.4 (Hide), §12.7.3 (`/CO`)

## Scope

**In scope**

- `forms::behaviour` in `laminapdf-model`: the tokeniser, the helper-call list, `FieldBehaviour` (`NumberFormat`, `PercentFormat`, `DateFormat`, `TimeFormat`, `SpecialFormat`, `RangeValidate`, `SimpleCalculate`)
- `Limits::max_script_length` through ADR 0003's process, its default measured on the corpus
- Native formatting (to E8d's builders), keystroke filters, validation, calculation in `/CO` order inside the user's transaction; locked calculated fields skipped and listed
- `SetFieldHidden { field, hidden }` (class `Annotation`: allowed under P3, a violation under P1 and P2, UR3 `/Annots [/Modify]` through `Op::annot_right`); pyHanko's difference analysis on `form-certified-p2.pdf` recorded, since promotion to `FormFill` needs it and an Acrobat check both to accept the hide ([E8 plan §5](../../plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint))
- `form_scripts()`: every script with location, size and state (`handled`, `not handled`, `xfa`)
- The inventory run over the external corpus and the test forms (ADR 0021 §3's evidence), reported in the nightly summary
- An `af_script` fuzz target
- `fixtures/forms/scripts.pdf`; one real form with helper calls (public domain, provenance in `fixtures/README.md`)
- Façade `form_scripts`, `field_behaviour`, `validate`, `calculate`; SDK `formScripts` and the rest

**Out of scope** (and which todo covers it)

- The reader's formatted input, messages and script notice → [E8j](../ready/E8j-validation-and-calculation-ui.md)
- Hide actions fired by buttons → [E8i](../ready/E8i-form-actions-and-submit.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.05` date and formatted input; `E.07` input validation; `E.08` automatic calculations; `E.10` conditional behaviour
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf-cos`, the façade and bindings
- **`Op` variants:** `SetFieldHidden` (class `Annotation`); calculations add `SetFieldValue`s to the user's transaction
- **`SaveConstraint`:** as E8c for calculations; a calculated field a lock covers is skipped, never a conflict. `SetFieldHidden` as an annotation change: allowed under P3 and approval signatures, a violation under P1 and P2
- **Web vs desktop:** identical (core)
- **Server features:** none
- **Capability-matrix rows to flip:** `E.05` → `round-trip` (core, pdf); `E.07` → `round-trip` (core, pdf); `E.08` → `round-trip` (core, pdf); `E.10` → `round-trip` (core, pdf)

## Tasks

- [ ] The recogniser and its limit
- [ ] Native implementations per helper
- [ ] Calculation order and the transaction
- [ ] `SetFieldHidden` and its classification test
- [ ] `form_scripts` and the corpus run
- [ ] The fuzz target, added to `laminapdf-model`'s regression guard list

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7)
- [ ] No code path evaluates script text; a test feeds every non-matching shape of ADR 0021 §2 and gets `not handled`
- [ ] Recognised formats give the displayed values of pdf.js on the real form (Acrobat's, if the owner adds reference files later, are optional extras)
- [ ] Undo of a change reverts its totals; `would_violate` gives the plan's verdicts for `SetFieldHidden` and for calculated fields
- [ ] The inventory run reports the share of handled scripts per corpus source

## Post-merge checks

- [ ] The nightly's first inventory report is linked from ADR 0021 §3

## Tests

- Unit tests per helper and format string; the form corpus with expected values; `af_script` fuzzing

## Docs to update

- API docs: form behaviour; user help: forms that use scripts
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Acrobat's helpers are undocumented one by one; pdf.js's implementation (Apache-2.0) is the second reading (ADR 0021 §2)

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
