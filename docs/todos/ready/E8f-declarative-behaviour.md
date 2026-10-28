# E8f — Formatting, validation, calculation and conditional behaviour (declarative and per ADR 0021)

| Field | Value |
| --- | --- |
| ID | `E8f` |
| Status | `ready` |
| Plan | [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8a](./E8a-adr-0021-javascript-policy.md), [E8e](./E8e-field-widgets-ui.md) |
| Blocks | [E9c](./E9c-form-design-mode.md) |
| ADRs | Implements ADR 0021; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e8f-declarative-behaviour` |
| PR title | `feat(forms): native formatting, validation and calculation` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Make interactive forms behave without arbitrary JavaScript: Hide actions and `/CO` calculation order first; recognised Acrobat AF* patterns (number, percent, date, special formats; simple calculations: sum/avg/product/min/max) implemented natively; everything else as ADR 0021 decides (sandbox or report).

## Context

- ADR 0021 (E8a)
- [04 §3.3](../../research/pdf/04-interactive-security.md#33-calculation-order-and-scripts)
- [04 §4](../../research/pdf/04-interactive-security.md#4-javascript-in-pdf)

## Scope

**In scope**

- Pattern recogniser for AF* calls in `/AA` scripts
- Native formatter/validator/calculator
- `/CO` ordering and recalculation
- Report listing scripts not handled

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.05` date/formatted input; `E.07` input validation; `E.08` automatic calculations; `E.10` conditional behaviour
- **Crates and packages:** `lamina-model` (forms), new `forms::behaviour` module
- **`Op` variants:** calculations run as `System`-origin transactions? No: as part of the user's `SetFieldValue` transaction (decide in E8b)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `FormFill` is allowed under DocMDP P2 and P3 and under approval signatures, except for fields locked by FieldMDP (`/Lock`) or by a signature's field-lock dictionary → blocked with the locking signature named
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `E.05` → `round-trip` (core, pdf, web, desktop); `E.07` → `round-trip` (core, pdf), `read` (web, desktop); `E.08` → `round-trip` (core, pdf), `read` (web, desktop) (web and desktop are intermediate: existing validation and calculations run when filling; they flip to `round-trip` once, in [E9c](E9c-form-design-mode.md), which adds authoring them in the UI); `E.10` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Recogniser
- [ ] Native implementations
- [ ] Calculation order
- [ ] Report
- [ ] Tests on real forms (IRS, government forms with AF* scripts)

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug (here: calculation/format side effects inside a `SetFieldValue` transaction stay `FormFill`; a calculated field that is FieldMDP-locked is a violation)
- [ ] Recognised AF* formats produce the same displayed values as Acrobat on reference forms
- [ ] Unhandled scripts are listed in the UI; nothing executes outside ADR 0021

## Post-merge checks

None.

## Tests

- Form corpus with expected values
- Unit tests per AF function

## Docs to update

- User help: interactive forms limits
- [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
