# E9c — Form-design mode: inspector, validation/format/calculation settings, tab-order editor, accessible labels

| Field | Value |
| --- | --- |
| ID | `E9c` |
| Status | `ready` |
| Plan | [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8f](./E8f-declarative-behaviour.md), [E9b](./E9b-field-creation-ops.md) |
| Blocks | [E9d](./E9d-automatic-field-detection.md), [E17b](./E17b-action-editor.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e9c-form-design-mode` |
| PR title | `feat(ui): form design mode` |
| Size | L (600–1,500) |
| PR | — |

## Goal

`formDesign` mode in `@lamina/ui`: draw fields, move/resize with snapping, property inspector (name, tooltip `/TU`, flags, options, format, validation, calculation via the native declarative settings from E8f), calculation-order and tab-order editors, and accessible labels.

## Context

- [full-editor §E9](../../plans/full-editor.md#e9-form-authoring)

## Scope

**In scope**

- Design mode tools
- Inspector
- Calc-order and tab-order editors
- `/TU` labels and Form struct elements when tagged (with E15)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.07` input validation; `E.08` automatic calculations; `E.09` tab order; `L.06` accessible form labels
- **Crates and packages:** `@lamina/ui`, `@lamina/viewer`
- **`Op` variants:** E9b ops; `SetTabOrder`, `SetCalculationOrder` (class `Document`)
- **`SaveConstraint`:** `Free` → incremental. Signed: creating/changing field definitions is not form filling (class `Document`); blocked under any certification and invalidates approval signatures' coverage → only allowed after the user confirms removing/invalidating signatures (ADR 0007 §3 override)
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `E.07` → `round-trip` (web, desktop); `E.08` → `round-trip` (web, desktop); `E.09` → `round-trip` (web, desktop); `L.06` → `round-trip` (web, desktop); `E.01` → `round-trip` (web, desktop); `E.15` → `round-trip` (web, desktop)

## Tasks

- [ ] Tools
- [ ] Inspector
- [ ] Editors
- [ ] e2e

## Acceptance criteria

- [ ] e2e: design a form with each field type, save, fill it in pdf.js
- [ ] axe zero serious violations

## Tests

- Playwright, WebdriverIO

## Docs to update

- User help: designing forms
- [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
