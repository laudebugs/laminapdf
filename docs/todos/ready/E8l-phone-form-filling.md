# E8l — Phone: fill a form field by field

| Field | Value |
| --- | --- |
| ID | `E8l` |
| Status | `ready` |
| Plan | [E8 plan §6](../../plans/editor/e8-form-filling.md#6-differences-between-the-shells) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8e](../ongoing/E8e-field-widgets-ui.md), [E8j](./E8j-validation-and-calculation-ui.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md) (the phone shell) |
| Branch | `feat/e8l-phone-form-filling` |
| PR title | `feat(ui): fill forms on a phone, field by field` |
| Size | M (200–400) |
| PR | — |

## Goal

People fill a form on a phone without pinching. The page zooms to each field, a form bar above the keyboard carries Previous, Next, Next required and Done, choice fields open a sheet, and errors show in the bar.

## Context

- **Design (Paper):** [P04 Phone: fill a form field by field](../../design/paper/forms.md#p04-phone-fill-a-form-field-by-field). Build to match it; record any deviation in the area document.
- P03's phone notes: Form data in the More sheet, export through the share sheet

## Scope

**In scope**

- The form bar riding on the keyboard, the 16 px zoom, the choice sheet, the last field's Review
- Form data in the More sheet; export through `canShare` where it is true, else a download
- The P1 notice and Apply anyway… in More

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): none new; the web column's rows on a phone
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`Op` variants:** none new
- **`SaveConstraint`:** as E8e
- **Web vs desktop:** phone breakpoints of the web reader
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] The form bar and zoom
- [ ] The choice sheet
- [ ] Playwright specs at phone width with touch

## Acceptance criteria

- [ ] Playwright (iPhone and Pixel profiles): fill every field type with Next only, fix an invalid date from the bar, save
- [ ] Every target is at least 44 px; focus moves to the field, never the bar; axe passes

## Post-merge checks

None.

## Tests

- Playwright in `apps/web/e2e/` with device profiles

## Docs to update

- User help: filling forms on a phone
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos

## Risks and open questions

- A browser's emulated keyboard is not the real one; check the bar on a real iPhone and Android phone before merging, and say so in the PR

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
