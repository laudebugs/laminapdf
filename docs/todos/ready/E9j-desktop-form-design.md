# E9j — Desktop: form design, and the round's close

| Field | Value |
| --- | --- |
| ID | `E9j` |
| Status | `ready` |
| Plan | [E9 plan §6](../../plans/editor/e9-form-authoring.md#6-differences-between-the-shells) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8m](./E8m-desktop-forms.md), [E9h](./E9h-behaviour-and-tab-order-ui.md), [E9i](./E9i-field-detection-review-ui.md) |
| ADRs | Follows [0007](../../decisions/0007-save-planner.md), [0010](../../decisions/0010-platform-adapter.md) |
| Branch | `feat/e9j-desktop-form-design` |
| PR title | `feat(desktop): form design` |
| Size | M (200–400 of code; e2e specs beyond) |
| PR | — |

## Goal

Form design in the desktop app: the round's ops through the engine, fonts on the computer for field appearances, and WebdriverIO suites on macOS, Windows and Linux. It closes the round: every E9 row reaches its target on every surface, and the manual Acrobat checks are recorded.

## Context

- [E9 plan §6 and §7](../../plans/editor/e9-form-authoring.md#6-differences-between-the-shells)
- E8m's desktop suites: the pattern to follow

## Scope

**In scope**

- The engine runs every E9 op and `suggest_fields`
- `systemFonts` in the field appearance font picker
- WebdriverIO suites for P05, P06 and P07
- The round close: capability statuses checked, the plan's Status and to-dos

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01`–`E.04`, `E.07`–`E.09`, `E.15`, `L.06` (labels)
- **Crates and packages:** `laminapdf-desktop-engine`, `apps/desktop`
- **`Op` variants:** the round's, from the desktop
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** this todo is the desktop side
- **Server features:** none
- **Capability-matrix rows to flip:** `E.01`–`E.04`, `E.07`–`E.09`, `E.15` → `round-trip` (desktop); `L.06` tagged at `write` (desktop)

## Tasks

- [ ] Engine and fonts
- [ ] WebdriverIO suites
- [ ] Round close

## Acceptance criteria

- [ ] WebdriverIO: design a form with every field type, save, reopen, fill, on macOS, Windows and Linux
- [ ] `cargo xtask capabilities` shows every E9 row at its target on every surface

## Post-merge checks

None.

## Tests

- WebdriverIO in `apps/desktop`

## Docs to update

- User help: designing forms (desktop notes)
- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`
- The E9 plan's **Status**

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
