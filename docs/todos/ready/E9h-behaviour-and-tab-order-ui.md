# E9h — Format, validation and calculation settings; tab order and accessible labels

| Field | Value |
| --- | --- |
| ID | `E9h` |
| Status | `ready` |
| Plan | [E9 plan §2.3](../../plans/editor/e9-form-authoring.md#23-behaviour-calculation-order-and-tab-order) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E9c](./E9c-form-design-mode.md), [E9g](./E9g-behaviour-and-order-ops.md) |
| ADRs | Follows [0007](../../decisions/0007-save-planner.md), [0021](../../decisions/0021-pdf-javascript-policy.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e9h-behaviour-and-tab-order-ui` |
| PR title | `feat(ui): field behaviours, tab order and labels in form design` |
| Size | L (400–800 of code; e2e specs beyond) |
| PR | — |

## Goal

The rest of form design in the web reader: P05's Format, Validation and Calculation sections and its Calculation order dialog, and P06's tab-order view with badges, the ordered list, the order select, and the accessible-label tools (the unlabelled-field count, Use nearby text, the screen-reader preview).

## Context

- **Design (Paper):** [P05 Form design mode](../../design/paper/forms.md#p05-form-design-mode), [P06 Tab order and accessible labels](../../design/paper/forms.md#p06-tab-order-and-accessible-labels). Build to match them; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E9 plan §2.3 and §2.4](../../plans/editor/e9-form-authoring.md#23-behaviour-calculation-order-and-tab-order)

## Scope

**In scope**

- The inspector's Format, Validation and Calculation sections, disabled on PDF/A files
- The Calculation order dialog
- Tab order mode: badges, the list, the order select, clicking badges in sequence
- Labels: the count, Next field without a label, Use nearby text, the preview, the tagging checkbox

**Out of scope** (and which todo covers it)

- Detection → [E9i](E9i-field-detection-review-ui.md)
- The desktop → [E9j](E9j-desktop-form-design.md)
- Tagging fields Lamina PDF didn't create → [E15c](E15c-tagging-tools.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.07`, `E.08`, `E.09`, `L.06` (labels)
- **Crates and packages:** `@laminapdf/viewer`, `@laminapdf/ui`, `apps/web`
- **`Op` variants:** `SetFieldBehaviour`, `SetCalculationOrder`, `SetTabOrder`, `UpdateFieldDef` from the interface
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `E.07`, `E.08`, `E.09` → `round-trip` (web); `L.06` is tagged at `write` (web) and flips in E15c

## Tasks

- [ ] Behaviour sections
- [ ] Calculation order dialog
- [ ] Tab order mode
- [ ] Label tools
- [ ] Playwright specs tagged with capability IDs

## Acceptance criteria

- [ ] e2e: give a field a currency format and a total, reorder tabs by clicking badges, label every field, save, and check the order and totals in pdf.js
- [ ] A screen reader announces each new label (ARIA snapshot); axe reports no serious violation

## Post-merge checks

None.

## Tests

- Vitest for the list and the dialogs
- Playwright in `apps/web/e2e/`

## Docs to update

- User help: tab order and labels
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
