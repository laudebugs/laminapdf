# E9f — Signature fields, with locks and seed values

| Field | Value |
| --- | --- |
| ID | `E9f` |
| Status | `ready` |
| Plan | [E9 plan §2.2](../../plans/editor/e9-form-authoring.md#22-signature-fields) ([full-editor §E9](../../plans/full-editor.md#e9-form-authoring)) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E9e](./E9e-field-configuration-ops.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e9f-signature-fields` |
| PR title | `feat(forms): create signature fields` |
| Size | M (200–400 of code) |
| PR | — |

## Goal

Unsigned signature fields that E10 signs: visible or invisible, with an optional `/Lock` (the fields locked once it is signed) and `/SV` seed values. People prepare a form for signing before anyone signs it.

## Context

- **Design (Paper):** [P05 Form design mode](../../design/paper/forms.md#p05-form-design-mode). Build to match it; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [E9 plan §2.2](../../plans/editor/e9-form-authoring.md#22-signature-fields)
- [04 §8.1](../../research/pdf/04-interactive-security.md#81-objects-involved): signature objects
- ISO 32000-2 §12.7.5.5 (signature fields, `/Lock`, seed values) and §12.7.3 (`/SigFlags`)

## Scope

**In scope**

- `FieldSpec::Signature` in `CreateField`; `lock` and `seed` in `UpdateFieldDef`
- `/SigFlags` bit 1 on the first signature field
- The empty signature box appearance
- `TerminalField::lock` and `TerminalField::seed` read back

**Out of scope** (and which todo covers it)

- Signing, FieldMDP transforms and certification → [E10f](E10f-certification-and-locking.md)
- The interface → [E9c](E9c-form-design-mode.md)

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `E.15`
- **Crates and packages:** `laminapdf-model`, `laminapdf-ops`, `laminapdf-build`, `laminapdf`, `@laminapdf/sdk`
- **`Op` variants:** `CreateField` and `UpdateFieldDef` gain signature fields; `CreateField` with a signature is class `Signature`, `UpdateFieldDef` stays `Document` ([E9 plan §2.2](../../plans/editor/e9-form-authoring.md#22-signature-fields))
- **`SaveConstraint`:** as [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)
- **Web vs desktop:** identical
- **Server features:** none
- **Capability-matrix rows to flip:** `E.15` → `round-trip` (core, pdf)

## Tasks

- [ ] Model types and read-back
- [ ] Ops and appearance
- [ ] `signature-field-locked.pdf` in `fixtures/signed/make.py`
- [ ] pyHanko check

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is on `laminapdf` and `@laminapdf/sdk` in this PR, with rustdoc or TSDoc, a runnable example, tsify types and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Property tests: for every new op, apply then undo leaves the overlay empty and the next save is byte-identical (ADR 0006 §3); redo re-applies the same bytes
- [ ] Every new op reports its class (`Signature` for a new signature field, else `Document`) and `requires_rewrite` false; a test per op runs `planner.would_violate(&op)` on the signed fixtures (approval, DocMDP P1, P2, P3, `locked.pdf`, `form-locked-include.pdf`, the UR3 files) and asserts [E9 plan §5](../../plans/editor/e9-form-authoring.md#5-saving-and-the-save-constraint)'s verdict, because a misclassified op is a security bug
- [ ] Round-trip: open, op, save, reopen in Lamina PDF, pdf.js and PDFium keeps the change and everything else (pages, outline, annotations, fields, structure facts in `expected.json`); `qpdf --check` is clean
- [ ] pyHanko's difference analysis accepts a new visible and a new invisible signature field on `certified-p2.pdf`, or the visible one becomes class `Document` and the E9 plan §10 records why
- [ ] pyHanko signs a created field (visible and invisible) and validates it; it reads the `/Lock` and `/SV` written

## Post-merge checks

None.

## Tests

- Unit tests for `/Lock` actions, `/SV` entries and `/SigFlags`
- pyHanko round-trip in `fixtures/signed/check_saved.py`

## Docs to update

- [E9 plan §7](../../plans/editor/e9-form-authoring.md#7-todos), [full-editor §E9](../../plans/full-editor.md#e9-form-authoring) and its §10 to-dos
- `docs/capabilities/README.md` via `cargo xtask capabilities`

## Risks and open questions

- E10's signing flow must follow `/SV`; note it in E10j if E10b's plan doesn't

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
