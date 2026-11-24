# E8n — Flat forms: the Text, Check and Date tools

| Field | Value |
| --- | --- |
| ID | `E8n` |
| Status | `ongoing` |
| Plan | [E8 plan §10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E4j](../done/E4j-text-box-tools.md), [E8e](../done/E8e-field-widgets-ui.md) |
| ADRs | Follows [0006](../../decisions/0006-op-log-and-undo.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e8n-flat-form-tools` |
| PR title | `feat(ui): fill flat forms with Text, Check and Date` |
| Size | S (< 200) |
| PR | — |

## Goal

People fill a form that has no fields, the scanned or printed kind they get most often. Fill & Sign's Text, Check and Date tools place a borderless text box, a tick or today's date where they click, as presets of the text box tool.

## Context

- **Design (Paper):** [P01 Fill & Sign mode](../../design/paper/forms.md#p01-fill--sign-mode)'s toolbar (`form.add-text`, `form.add-check`, `form.add-date`) and its "No fields" state. Build to match it; record any deviation in the area document.
- [E8 plan §10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions), question 10
- [E4j](../done/E4j-text-box-tools.md): the text box tool these tools preset

## Scope

**In scope**

- The three tools in Fill & Sign's toolbar and the palette, each an `AddAnnot` with a FreeText preset (no border, the date in the reader's locale format, the tick as a glyph from the standard 14 fonts)
- The "No fields" empty state's pointer to them

**Out of scope** (and which todo covers it)

- Detecting fields on a flat form → [E9d](../ready/E9d-automatic-field-detection.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): none (annotations, not fields)
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`Op` variants:** none new; E4's `AddAnnot`
- **`SaveConstraint`:** as an annotation: allowed under P3 and approval signatures, a violation under P1 and P2
- **Web vs desktop:** this todo is the web side; E8m checks the desktop
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] The three presets and their commands
- [ ] Playwright specs

## Acceptance criteria

- [ ] Playwright: on a page without fields, place a text, a tick and a date, save, and reopen in pdf.js with all three drawn
- [ ] Each placement is one undo step; axe passes

## Post-merge checks

None.

## Tests

- Playwright in `apps/web/e2e/`

## Docs to update

- User help: filling a form without fields
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
