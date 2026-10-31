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

`formDesign` mode in `@laminapdf/ui`: draw fields, move/resize with snapping, property inspector (name, tooltip `/TU`, flags, options, format, validation, calculation via the native declarative settings from E8f), calculation-order and tab-order editors, and accessible labels.

## Context

- **Design (Paper):** [P05 Form design mode](../../design/paper/forms.md#p05-form-design-mode), [P06 Tab order and accessible labels](../../design/paper/forms.md#p06-tab-order-and-accessible-labels). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
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
- **Crates and packages:** `@laminapdf/ui`, `@laminapdf/viewer`
- **`Op` variants:** E9b ops; `SetTabOrder`, `SetCalculationOrder` (class `Document`)
- **`SaveConstraint`:** `Free` → incremental. Signed: creating/changing field definitions is not form filling (class `Document`), so it is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → saved only after the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; changing or removing a field locked by FieldMDP/`/Lock`, or a signature field, is a violation
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `E.07` → `round-trip` (web, desktop); `E.08` → `round-trip` (web, desktop); `E.09` → `round-trip` (web, desktop); `E.01` → `round-trip` (web, desktop); `E.15` → `round-trip` (web, desktop). `L.06` accessible form labels is not flipped here even though the design panel edits `/TU`; it flips once, in [E15c](E15c-tagging-tools.md), which completes it with form tagging

## Tasks

- [ ] Tools
- [ ] Inspector
- [ ] Editors
- [ ] e2e

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@laminapdf/sdk` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] e2e: design a form with each field type, save, fill it in pdf.js
- [ ] axe zero serious violations

## Post-merge checks

None.

## Tests

- Playwright, WebdriverIO

## Docs to update

- User help: designing forms
- [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
