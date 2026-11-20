# E9i — Review suggested fields on flat forms

| Field | Value |
| --- | --- |
| ID | `E9i` |
| Status | `ready` |
| Plan | [E9 plan §2.5](../../plans/editor/e9-form-authoring.md#25-automatic-field-detection) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E9c](./E9c-form-design-mode.md), [E9d](./E9d-automatic-field-detection.md) |
| ADRs | Follows [0024](../../decisions/0024-app-shell-ux.md), [0030](../../decisions/0030-extraction-output-and-model-plugins.md) |
| Branch | `feat/e9i-field-detection-review-ui` |
| PR title | `feat(ui): review suggested fields` |
| Size | M (200–400 of code; e2e specs beyond) |
| PR | — |

## Goal

P07 in the web reader: Detect fields runs `suggestFields`, the Fields tab lists the suggestions, and the user accepts, edits, rejects or groups each. Accepting makes a `CreateField`; nothing is added until the user acts.

## Context

- **Design (Paper):** [P07 Automatic field detection review](../../design/paper/forms.md#p07-automatic-field-detection-review). Build to match it; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E9 plan §2.5](../../plans/editor/e9-form-authoring.md#25-automatic-field-detection)

## Scope

**In scope**

- Detect fields, the notice, the suggestions view and inspector
- Accept, Accept all on this page (one undo step), Reject and restore, Group as one radio group
- Leaving with pending suggestions

**Out of scope** (and which todo covers it)

- The desktop → [E9j](E9j-desktop-form-design.md)

**Editor facts**

- **Operations shipped:** none (assists E.01–E.03 authoring)
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`, `apps/web`
- **`Op` variants:** `CreateField` on accept
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** this todo is the web side
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Suggestions view and inspector
- [ ] Accept, reject and group
- [ ] Playwright specs

## Acceptance criteria

- [ ] No suggestion is applied without user action (e2e: open, detect, leave, and the file is unchanged)
- [ ] e2e: accept every suggestion on a flat form from the labelled set, save, and fill the fields in pdf.js
- [ ] axe reports no serious violation

## Post-merge checks

None.

## Tests

- Vitest for the review state
- Playwright in `apps/web/e2e/`

## Docs to update

- User help: turning a flat form into a fillable one
- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
