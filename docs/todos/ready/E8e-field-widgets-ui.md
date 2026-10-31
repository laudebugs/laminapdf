# E8e — Field widgets as DOM overlays, tab order, required/read-only, push buttons and submit

| Field | Value |
| --- | --- |
| ID | `E8e` |
| Status | `ready` |
| Plan | [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E1e](./E1e-save-and-constraint-banner.md), [E8d](./E8d-field-appearances.md) |
| Blocks | [E8f](./E8f-declarative-behaviour.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e8e-field-widgets-ui` |
| PR title | `feat(viewer): fillable form widgets as DOM overlays` |
| Size | L (600–1,500) |
| PR | — |

## Goal

`formFill` mode: form widgets as accessible DOM overlays in `@laminapdf/viewer` (inputs, checkboxes, radios, selects), tab order following `/Tabs`, required/read-only UI, push buttons executing non-JS actions (GoTo, URI with confirm, ResetForm, Hide), and SubmitForm off by default with a per-submit confirmation that says what leaves the device.

## Context

- **Design (Paper):** [P01 Fill & Sign mode](../../design/paper/forms.md#p01-fill--sign-mode), [P04 Phone: fill a form field by field](../../design/paper/forms.md#p04-phone-fill-a-form-field-by-field). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data)
- [full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 6 (privacy)
- ISO 32000-2 §12.7.6 (form actions)

## Scope

**In scope**

- Overlay layer synced with zoom/rotation
- Coalesced typing transactions
- Tab order (`/Tabs /R /C /S`, struct order)
- Action executor for non-JS actions
- Submit confirmation dialog (URL, fields, format)

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01` text fields (single/multi-line); `E.02` checkboxes and radio buttons; `E.03` dropdowns and list boxes; `E.04` push buttons; `E.06` required and read-only; `E.09` tab order; `E.16` submit to external endpoints
- **Crates and packages:** `@laminapdf/viewer`, `@laminapdf/ui`
- **`Op` variants:** `SetFieldValue` via UI
- **`SaveConstraint`:** `Free` → incremental. Signed: class `FormFill` is allowed under DocMDP P2 and P3 and under approval signatures, except for fields locked by FieldMDP (`/Lock`) or by a signature's field-lock dictionary → blocked with the locking signature named
- **Web vs desktop:** identical; submit uses `fetch` on web and the HTTP plugin on desktop (allow-listed scope)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `E.01` → `write` (web, desktop) (intermediate: existing text fields can be filled in the UI; it flips to `round-trip` once, in [E9c](E9c-form-design-mode.md), when they can also be created); `E.02` → `round-trip` (web, desktop); `E.03` → `round-trip` (web, desktop); `E.04` → `round-trip` (web, desktop); `E.06` → `round-trip` (web, desktop); `E.09` → `read` (web, desktop) (intermediate: the existing tab order is followed; it flips to `round-trip` once, in [E9c](E9c-form-design-mode.md), which edits it); `E.16` → `write` (web, desktop)

## Tasks

- [ ] Overlay components
- [ ] Tab order
- [ ] Actions
- [ ] Submit dialog
- [ ] e2e

## Acceptance criteria

- [ ] e2e: fill every field type, tab through, save, reopen in pdf.js
- [ ] Screen reader announces labels (`/TU`) and required state
- [ ] No network request without confirmation (Playwright asserts)

## Post-merge checks

None.

## Tests

- Playwright and WebdriverIO
- Network assertions

## Docs to update

- User help: filling forms
- [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
