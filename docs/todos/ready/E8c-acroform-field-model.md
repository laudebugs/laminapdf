# E8c — AcroForm field model, value ops, flags and XFA detection

| Field | Value |
| --- | --- |
| ID | `E8c` |
| Status | `ready` |
| Plan | [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8b](./E8b-form-filling-feature-plan.md) |
| Blocks | [E8d](./E8d-field-appearances.md), [E12d](./E12d-scrub-and-rewrite.md), [E18b](./E18b-collaboration-feature-plan.md), [E18c](./E18c-layer-document-schema.md), [E18e](./E18e-client-sync-and-presence.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e8c-acroform-field-model` |
| PR title | `feat(model): AcroForm field tree and form-fill ops` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Typed AcroForm views (field tree, fully qualified names, inheritance of `/FT`, `/Ff`, `/V`, `/DA`, `/Q`), `FieldId`s, `SetFieldValue` for every field type with FormFill classification and lock checks, required/read-only flags honoured, and XFA detection with a user-facing report (static XFA falls back to the AcroForm).

## Context

- [04 §3.1 AcroForm structure](../../research/pdf/04-interactive-security.md#31-acroform-structure-127-interactive-form-dictionary-1273-17-1272)
- [04 §3.2 Field types and flags](../../research/pdf/04-interactive-security.md#32-field-types-and-flags)
- [04 §3.4 XFA](../../research/pdf/04-interactive-security.md#34-xfa)
- [ADR 0007](../../decisions/0007-save-planner.md) §2 FieldMDP/Lock

## Scope

**In scope**

- Views: `Form`, `Field` (text, button: check/radio/push, choice: combo/list, signature), widgets, kids
- Ops: `SetFieldValue { field, value: FieldValue }`, `ResetFields { fields }` (to `/DV`)
- Lock detection (FieldMDP, `/Lock` dict) in the planner's violations
- XFA detection (`/XFA` in AcroForm, dynamic vs static) and report

**Out of scope** (and which todo covers it)

- Appearance generation → [E8d](E8d-field-appearances.md)
- Widgets UI → [E8e](E8e-field-widgets-ui.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.06` required and read-only; `E.17` XFA forms (cross-cutting)
- **Crates and packages:** `lamina-model` (forms), `lamina-ops`, `lamina-save` (lock checks)
- **`Op` variants:** `SetFieldValue`, `ResetFields` (class `FormFill`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `FormFill` is allowed under DocMDP P2 and P3 and under approval signatures, except for fields locked by FieldMDP (`/Lock`) or by a signature's field-lock dictionary → blocked with the locking signature named
- **Web vs desktop:** identical (core)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `E.06` → `round-trip` (core, pdf); `E.17` → `unsupported` (core, pdf)

## Tasks

- [ ] Views with inheritance and tolerant reading
- [ ] Ops
- [ ] Lock checks
- [ ] XFA detector and report codes

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Field names/types/values match pypdf and pdf.js on the form corpus
- [ ] Locked fields refuse `SetFieldValue` with the signature named

## Post-merge checks

None.

## Tests

- Form corpus differential
- Signed-form fixtures (FieldMDP)

## Docs to update

- API docs: forms (read and fill)
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
