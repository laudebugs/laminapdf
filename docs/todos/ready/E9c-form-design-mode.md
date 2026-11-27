# E9c — Form design mode: field tools, move and resize, the inspector

| Field | Value |
| --- | --- |
| ID | `E9c` |
| Status | `ready` |
| Plan | [E9 plan §4](../../plans/editor/e9-form-authoring.md#4-crate-and-package-changes) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8e](../done/E8e-field-widgets-ui.md), [E9e](./E9e-field-configuration-ops.md), [E9f](./E9f-signature-fields.md) |
| ADRs | Follows [0007](../../decisions/0007-save-planner.md), [0010](../../decisions/0010-platform-adapter.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e9c-form-design-mode` |
| PR title | `feat(ui): form design mode` |
| Size | L (400–800 of code; e2e specs beyond) |
| PR | — |

## Goal

P05 in the web reader: the `formDesign` mode, its field tools (text, check box, radio group, dropdown, list box, button, signature, date), select, move and resize with snapping, the Fields tab's design view, and the inspector for name, accessible label, options, flags, position, duplicate, delete and a signature field's lock. Each change is one op and one undo step.

## Context

- **Design (Paper):** [P05 Form design mode](../../design/paper/forms.md#p05-form-design-mode). Build to match it; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E9 plan §4 and §5](../../plans/editor/e9-form-authoring.md#4-crate-and-package-changes)
- E8e's form layer and Fields tab, and E4's `annotation-gesture.ts`: the patterns to reuse

## Scope

**In scope**

- The `formDesign` mode and toolbar; design style in the form layer
- Field tools, a drag as one step on release, keyboard moves and the Position inputs
- Snapping to a grid and to other fields' edges, shared with [E6e](../ongoing/E6e-content-tools-ui.md) (whichever lands first writes it)
- The inspector (all but Format, Validation and Calculation) and signature field settings
- The conflict dialog on signed files

**Out of scope** (and which todo covers it)

- Format, Validation, Calculation, the Calculation order dialog, tab order and labels → [E9h](E9h-behaviour-and-tab-order-ui.md)
- Detection → [E9i](E9i-field-detection-review-ui.md)
- The desktop → [E9j](E9j-desktop-form-design.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01`, `E.02`, `E.03`, `E.04`, `E.15`
- **Crates and packages:** `@laminapdf/viewer`, `@laminapdf/ui`, `apps/web`
- **`Op` variants:** E9b's, E9e's and E9f's from the interface
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** `E.01`, `E.02`, `E.03`, `E.04`, `E.15` → `round-trip` (web)

## Tasks

- [ ] Mode, toolbar and design style
- [ ] Tools, gestures and snapping
- [ ] Inspector
- [ ] Signed-file states
- [ ] Playwright specs tagged with capability IDs

## Acceptance criteria

- [ ] e2e: design a form with every field type, save, fill it in pdf.js
- [ ] On `certified-p2.pdf` the first field change opens the conflict dialog; on `form-locked-include.pdf` a locked field can't be changed without it
- [ ] axe reports no serious violation on each screen
- [ ] Form design loads in the editor's lazy chunk; `cargo xtask web size` stays within budget

## Post-merge checks

None.

## Tests

- Vitest for the mode's state and the inspector
- Playwright in `apps/web/e2e/`

## Docs to update

- User help: designing forms
- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
