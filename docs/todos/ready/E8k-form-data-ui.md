# E8k — The Form data menu: import, export, reset and flatten in the reader

| Field | Value |
| --- | --- |
| ID | `E8k` |
| Status | `ready` |
| Plan | [E8 plan §2.1 and §5](../../plans/editor/e8-form-filling.md#21-ops) ([full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8e](./E8e-field-widgets-ui.md), [E8g](./E8g-form-data-and-flatten.md) |
| ADRs | Follows [0007](../../decisions/0007-save-planner.md), [0010](../../decisions/0010-platform-adapter.md), [0024](../../decisions/0024-app-shell-ux.md) |
| Branch | `feat/e8k-form-data-ui` |
| PR title | `feat(ui): import, export, reset and flatten form data` |
| Size | M (200–400) |
| PR | — |

## Goal

People move answers in and out of a form from Fill & Sign's Form data menu. They import JSON, XFDF or FDF in one undo step, export the answers as a separate file, reset the form, and flatten it, with every signed-file and locked-field case P03 draws.

## Context

- **Design (Paper):** [P03 Form data](../../design/paper/forms.md#p03-form-data). Build to match it; record the deviation that import is a transaction of `SetFieldValue`, not an `ImportFormData` op ([E8 plan §10](../../plans/editor/e8-form-filling.md#10-decisions-on-the-open-questions), question 4).
- [E8 plan §5](../../plans/editor/e8-form-filling.md#5-saving-and-the-save-constraint)

## Scope

**In scope**

- The menu, its note and commands (`form.import-data`, `form.export-data`, `form.reset-fields`, `form.flatten-fields`)
- The import toast (filled, not in this form, locked), the no-match notice and the malformed-file error
- The export dialog (format, filled or all fields, file name) through `canSaveAsDialog`
- The reset alert dialog and the flatten dialog, both naming locked fields; flatten on a certified file as the conflict confirmation
- The Fields tab's summary

**Out of scope** (and which todo covers it)

- The phone's More sheet and share sheet → [E8l](E8l-phone-form-filling.md); the desktop's native dialogs → [E8m](E8m-desktop-forms.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.11`, `E.13`, `E.14`
- **Crates and packages:** `@laminapdf/ui`, `apps/web`
- **`Op` variants:** none new
- **`SaveConstraint`:** P03's table, from E8g's verdicts
- **Web vs desktop:** this todo is the web side
- **Server features:** none: data files stay on the device
- **Capability-matrix rows to flip:** `E.11`, `E.13`, `E.14` → `round-trip` (web)

## Tasks

- [ ] The menu and commands
- [ ] Import, export, reset and flatten dialogs
- [ ] Playwright specs tagged with capability IDs

## Acceptance criteria

- [ ] Playwright: export JSON, reset, import it back, flatten, save, and reopen in pdf.js with the values drawn and no fields left
- [ ] On `form-locked-include.pdf` reset and flatten leave the locked fields and say so; on `certified-p2.pdf` flatten needs "Flatten anyway"
- [ ] axe passes on each dialog

## Post-merge checks

None.

## Tests

- Vitest for the dialogs' state; Playwright in `apps/web/e2e/`

## Docs to update

- User help: form data
- [E8 plan §7](../../plans/editor/e8-form-filling.md#7-todos), [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- Gallery screenshots aren't refreshed by any workflow; say so in the PR if a shared component changes

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
