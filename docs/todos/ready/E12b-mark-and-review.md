# E12b — Mark and review: Redact annotations and preview

| Field | Value |
| --- | --- |
| ID | `E12b` |
| Status | `ready` |
| Plan | [full-editor §E12](../../plans/full-editor.md#e12-redaction) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E4b](./E4b-annotation-model-and-overlay.md), [E12a](./E12a-redaction-feature-plan.md) |
| Blocks | [E12c](./E12c-apply-content-removal.md), [E12e](./E12e-sensitive-search.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e12b-mark-and-review` |
| PR title | `feat(redact): mark redactions and preview them` |
| Size | M (200–600) |
| PR | — |

## Goal

`redact` mode: mark areas or text selections as Redact annotations (`/QuadPoints`, `/IC`, `/OverlayText`, `/RO`), review list with accept/reject, and a preview that renders the page as it will look after applying.

## Context

- [04 §10.2 What true redaction must do](../../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do)

## Scope

**In scope**

- Redact annotation spec and appearance
- Mode and review panel
- Preview render

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `G.06` permanent redaction
- **Crates and packages:** `lamina-model`, `lamina-build`, `@lamina/ui`
- **`Op` variants:** `MarkRedaction` (class `Annotation`)
- **`SaveConstraint`:** marks alone are annotations (incremental); the dialog warns that marks do not remove anything
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `G.06` → `write` (core, pdf, web, desktop)

## Tasks

- [ ] Spec
- [ ] Mode
- [ ] Panel
- [ ] Preview

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Marks round-trip and show in Acrobat as redaction marks (manual)

## Tests

- Round-trip
- e2e

## Docs to update

- User help: redaction
- [full-editor §E12](../../plans/full-editor.md#e12-redaction) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
