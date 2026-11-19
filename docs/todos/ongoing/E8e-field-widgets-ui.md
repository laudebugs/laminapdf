# E8e — Fill & Sign: the form layer, tab order, required and read-only

| Field | Value |
| --- | --- |
| ID | `E8e` |
| Status | `ongoing` |
| Plan | [E8 plan §4–§5](../../plans/editor/e8-form-filling.md#4-crate-and-package-changes) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E1e](../done/E1e-save-and-constraint-banner.md), [E4f](../done/E4f-annotation-tools-ui.md), [E8d](../done/E8d-field-appearances.md) |
| ADRs | Follows [0007](../../decisions/0007-save-planner.md), [0010](../../decisions/0010-platform-adapter.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e8e-field-widgets-ui` |
| PR title | `feat(viewer): fill forms in Fill & Sign mode` |
| Size | L (400–800 of code; e2e specs beyond) |
| PR | — |

## Goal

People fill a form in the web reader. Fill & Sign mode (`formFill`) puts a native control over each widget, Tab follows the form's order across pages, required fields are marked and counted, read-only and locked fields refuse typing, and a certified file asks before its first change. Each field visit is one undo step.

## Context

- **Design (Paper):** [P01 Fill & Sign mode](../../design/paper/forms.md#p01-fill--sign-mode). Build to match it; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E8 plan §4 and §5](../../plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint)
- E4f's Comment mode and E4i's annotation layer: the mode, toolbar and layer patterns to follow

## Scope

**In scope**

- `@laminapdf/viewer`'s form layer (`layers/form-layer.ts`): native inputs, text areas, check boxes, radio groups, list boxes and dropdowns over each widget, synced with zoom and rotation; typing coalesced into one `SetFieldValue` per field visit
- The `formFill` mode, its toolbar (Previous, Next, Highlight fields, Form data ▾ as a placeholder for E8k), the Fields rail tab, the required count
- Tab order from `tab_order`, across pages, leaving the form without a keyboard trap
- Read-only fields, locked fields ("Locked by … signature"), the P1 banner and conflict dialog
- The status bar's form note, with `/NeedAppearances`' "Some answers may show only in Fill & Sign"

**Out of scope** (and which todo covers it)

- Push buttons, actions and submit → [E8i](../ready/E8i-form-actions-and-submit.md)
- Flat forms' Text, Check and Date tools → [E8n](../ready/E8n-flat-form-tools.md)
- Formatting, validation and calculated fields → [E8j](../ready/E8j-validation-and-calculation-ui.md)
- The Form data menu → [E8k](../ready/E8k-form-data-ui.md)
- The phone → [E8l](../ready/E8l-phone-form-filling.md); the desktop → [E8m](../ready/E8m-desktop-forms.md)
- The XFA notice → [E8h](../ready/E8h-xfa-detection-and-notice.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01`, `E.02`, `E.03`, `E.06`, `E.09`
- **Crates and packages:** `@laminapdf/viewer`, `@laminapdf/ui`, `apps/web`
- **`Op` variants:** `SetFieldValue` from the interface
- **`SaveConstraint`:** as [E8 plan §5](../../plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `E.01`, `E.02`, `E.03` → `write` (web); `E.06` → `round-trip` (web); `E.09` → `read` (web)

## Tasks

- [ ] The form layer and its worker calls
- [ ] The mode, toolbar, Fields tab and status note
- [ ] Tab order and focus rules
- [ ] Locked, read-only and certified states
- [ ] Playwright specs tagged with capability IDs

## Acceptance criteria

- [ ] e2e: fill every field type, tab through both pages, save, and reopen in pdf.js with the values
- [ ] A screen reader announces each field's `/TU` and required state (axe and an ARIA snapshot)
- [ ] On `form-locked-include.pdf` a locked field is read-only and names its signature; on `certified-p1.pdf` the banner shows on entering the mode
- [ ] Every Fill & Sign screen loads in the editor's lazy chunk; `cargo xtask web size` stays within budget

## Post-merge checks

None.

## Tests

- Vitest for the mode's state and focus order; Playwright in `apps/web/e2e/`

## Docs to update

- User help: filling forms
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- P01's web region-cycling key (Ctrl F6) is untested; test it in Chrome and Firefox on Windows and Linux and record the result in the area document
- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
