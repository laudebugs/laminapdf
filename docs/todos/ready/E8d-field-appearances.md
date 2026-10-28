# E8d — Value appearance generation for every field type

| Field | Value |
| --- | --- |
| ID | `E8d` |
| Status | `ready` |
| Plan | [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E4e](./E4e-freetext-callouts.md), [E8c](./E8c-acroform-field-model.md) |
| Blocks | [E8e](./E8e-field-widgets-ui.md), [E8g](./E8g-form-data-and-flatten.md), [E9b](./E9b-field-creation-ops.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e8d-field-appearances` |
| PR title | `feat(build): appearance streams for form field values` |
| Size | L (600–1,500) |
| PR | — |

## Goal

Generate `/AP` for field values: text fields (DA parse, auto-size, multiline, comb, quadding, rotation, password masking), check boxes and radio buttons (on-state names, `/AS`), combo and list boxes (selected rows, top index), so filled forms look the same in every viewer.

## Context

- [04 §3.5 Filling forms: generating appearances](../../research/pdf/04-interactive-security.md#35-filling-forms-generating-appearances-the-hard-part)
- [04 §2.4](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter)

## Scope

**In scope**

- Appearance builders reusing E4e text layout and DA handling
- Font fallback with `/DR` updates and subset embedding
- Comb and auto-size algorithms
- `/NeedAppearances` policy from E8b

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.01` text fields (single/multi-line); `E.02` checkboxes and radio buttons; `E.03` dropdowns and list boxes; `E.12` populate fields programmatically
- **Crates and packages:** `lamina-build`, `lamina-text`
- **`Op` variants:** appearance regeneration inside `SetFieldValue`
- **`SaveConstraint`:** `Free` → incremental. Signed: class `FormFill` is allowed under DocMDP P2 and P3 and under approval signatures, except for fields locked by FieldMDP (`/Lock`) or by a signature's field-lock dictionary → blocked with the locking signature named
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `E.01` → `round-trip` (core, pdf); `E.02` → `round-trip` (core, pdf); `E.03` → `round-trip` (core, pdf); `E.12` → `round-trip` (core, pdf)

## Tasks

- [ ] Builders
- [ ] Auto-size and comb
- [ ] Fallback fonts
- [ ] Goldens vs Acrobat

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Filled values render the same in pdf.js, PDFium and Acrobat reference renders (diff thresholds)
- [ ] Non-Latin values render and extract correctly

## Tests

- Goldens
- Render diffs
- Acrobat-filled reference fixtures

## Docs to update

- API docs: filling forms
- [full-editor §E8](../../plans/full-editor.md#e8-form-filling-and-data) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
