# E9b — Field creation and configuration ops, including signature fields

| Field | Value |
| --- | --- |
| ID | `E9b` |
| Status | `ready` |
| Plan | [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E8d](./E8d-field-appearances.md), [E9a](./E9a-form-authoring-feature-plan.md) |
| Blocks | [E9c](./E9c-form-design-mode.md), [E10d](./E10d-pades-signing-core.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e9b-field-creation-ops` |
| PR title | `feat(ops): create and configure form fields` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Ops to create, move, resize, rename, delete and configure fields of every type (text, check box, radio group, combo, list, push button, signature) with widgets, default appearances, `/DR`, options and flags; unique fully qualified names.

## Context

- [04 §3.1](../../research/pdf/04-interactive-security.md#31-acroform-structure-127-interactive-form-dictionary-1273-17-1272)
- [04 §3.2](../../research/pdf/04-interactive-security.md#32-field-types-and-flags)
- [04 §8.1 signature objects](../../research/pdf/04-interactive-security.md#81-objects-involved)

## Scope

**In scope**

- `CreateField`, `UpdateFieldDef`, `MoveWidget`, `DeleteField`, `RenameField` ops
- Signature fields with optional `/Lock` and `/SV` seed values
- Name-clash validation

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01` text fields (single/multi-line); `E.02` checkboxes and radio buttons; `E.03` dropdowns and list boxes; `E.04` push buttons; `E.15` signature fields
- **Crates and packages:** `lamina-model`, `lamina-build`, `lamina-ops`
- **`Op` variants:** the ops above (class `Document`)
- **`SaveConstraint`:** `Free` → incremental. Signed: creating/changing field definitions is not form filling (class `Document`), so it is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations) → saved only after the user confirms invalidating the named signatures (ADR 0007 §3 override). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; changing or removing a field locked by FieldMDP/`/Lock`, or a signature field, is a violation
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `E.01` → `round-trip` (core, pdf); `E.02` → `round-trip` (core, pdf); `E.03` → `round-trip` (core, pdf); `E.04` → `round-trip` (core, pdf); `E.15` → `round-trip` (core, pdf)

## Tasks

- [ ] Ops
- [ ] Widget builders
- [ ] Seed values
- [ ] Tests

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical to the previous one (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Fields created by lamina are fillable in pdf.js, PDFium and Acrobat Reader (manual check recorded)

## Post-merge checks

None.

## Tests

- Round-trip
- Cross-viewer fill tests

## Docs to update

- API docs: creating forms
- [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
