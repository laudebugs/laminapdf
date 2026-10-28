# E10c — Visual signatures: image, drawn and typed (non-cryptographic)

| Field | Value |
| --- | --- |
| ID | `E10c` |
| Status | `ready` |
| Plan | [full-editor §E10](../../plans/full-editor.md#e10-signatures) |
| Round / milestone | Editor phase 5 (E8–E10) — M8 Forms and signing |
| Depends on | [E5c](./E5c-stamps-and-attachments.md), [E10b](./E10b-signatures-feature-plan.md) |
| Blocks | [E10j](./E10j-signature-ui.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e10c-visual-signatures` |
| PR title | `feat(sign): visual signatures clearly labelled as non-cryptographic` |
| Size | M (200–600) |
| PR | — |

## Goal

Place a handwritten-signature image, a drawn (ink) or typed (font) e-signature as a stamp/ink annotation or content, clearly labelled in the UI as not a digital signature, with a signature library stored on the device.

## Context

- [06 §3 group F](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms)

## Scope

**In scope**

- Signature capture (draw, type, image)
- Placement as Stamp annotation (default) or content
- Library storage per shell
- Labelling copy

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `F.01` handwritten signature image; `F.02` draw/type e-signature
- **Crates and packages:** `lamina-build`, `@lamina/ui`
- **`Op` variants:** `AddAnnot` spec `Stamp`/`Ink` (class `Annotation`)
- **`SaveConstraint`:** annotation rules (see E4/E5); on signed files gated by DocMDP
- **Web vs desktop:** desktop app-data, web IndexedDB
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `F.01` → `round-trip` (core, pdf, web, desktop); `F.02` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Capture
- [ ] Placement
- [ ] Library
- [ ] Copy review

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] UI never calls these "digital signatures" (copy test)

## Tests

- e2e

## Docs to update

- User help: visual vs digital signatures
- [full-editor §E10](../../plans/full-editor.md#e10-signatures) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
