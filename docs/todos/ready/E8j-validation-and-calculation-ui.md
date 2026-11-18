# E8j — Formatted input, validation messages and calculated fields in the reader

| Field | Value |
| --- | --- |
| ID | `E8j` |
| Status | `ready` |
| Plan | [E8 plan §2.3](../../plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8e](./E8e-field-widgets-ui.md), [E8f](./E8f-declarative-behaviour.md) |
| ADRs | Follows [0021](../../decisions/0021-pdf-javascript-policy.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e8j-validation-ui` |
| PR title | `feat(ui): formatted input, validation and calculated fields` |
| Size | M (200–400) |
| PR | — |

## Goal

The reader shows a form's own rules while people fill it: dates and amounts format on leaving a field, an invalid entry gets a message and is never saved, totals recalculate with the change, and a notice says which scripts Lamina PDF handled and which fields it couldn't.

## Context

- **Design (Paper):** [P02 Validation, formatting and calculated fields](../../design/paper/forms.md#p02-validation-formatting-and-calculated-fields). Build to match it; record any deviation in the area document.
- [E8 plan §2.3](../../plans/editor/e8-form-filling.md#23-behaviour-actions-and-submit), decisions 3 and 5 in [§10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions)

## Scope

**In scope**

- Keystroke filters, formatting on blur, `inputmode` per format
- Validation messages (`aria-invalid`, `role="alert"` the first time), the last valid value kept, Escape restoring it; the save and close prompt for invalid entries, which then drops them
- Calculated fields (Σ, caption, `<output>`), recalculated in the change's undo step
- The script notice and its Details panel; "Next field that needs attention"
- Hidden fields leave the page and the Fields tab

**Out of scope** (and which todo covers it)

- The phone's form bar → [E8l](E8l-phone-form-filling.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.05`, `E.07`, `E.08`
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`, `apps/web`
- **`Op` variants:** none new
- **`SaveConstraint`:** as E8f
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `E.05` → `round-trip` (web); `E.07` → `read` (web); `E.08` → `read` (web), their E8 targets; `E.07` and `E.08` reach `round-trip` in E9: [E9b](E9b-field-creation-ops.md) for core and pdf, where E8f already proves it, and [E9c](E9c-form-design-mode.md) for web and desktop

## Tasks

- [ ] Filters and formatting in the form layer
- [ ] Messages and the invalid-entry rules
- [ ] Calculated fields
- [ ] The script notice
- [ ] Playwright specs tagged with capability IDs

## Acceptance criteria

- [ ] Playwright on `scripts.pdf`: a date formats, an invalid date shows its message and saves the last valid value, a total follows its inputs and one undo restores both
- [ ] The notice lists every `not handled` script with its field; no script runs (ADR 0021)
- [ ] axe passes on each state

## Post-merge checks

None.

## Tests

- Vitest for the field states; Playwright in `apps/web/e2e/`

## Docs to update

- User help: forms with formats and totals; interactive forms' limits
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
