# E17c — Layers panel: show/hide and edit optional content groups

| Field | Value |
| --- | --- |
| ID | `E17c` |
| Status | `ready` |
| Plan | [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E17a](./E17a-advanced-interactions-feature-plan.md) |
| Blocks | [E17b](./E17b-action-editor.md), [E17f](./E17f-3d-detection.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e17c-layers` |
| PR title | `feat(ocg): layers panel and OCG editing` |
| Size | M (200–600) |
| PR | — |

## Goal

List OCGs and configurations, toggle visibility in the viewer (reader feature on both shells), rename, reorder, lock, set default state, and move lamina-added content into layers.

## Context

- ISO 32000-2 §8.11 (optional content)

## Scope

**In scope**

- OCG model and ops
- Viewer visibility (PDFium OC config)
- Panel

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `M.04` manage/show/hide layers; `I.08` optional content groups (layers)
- **Crates and packages:** `lamina-model`, `lamina-render`, `@lamina/ui`
- **`Op` variants:** `UpdateOcg`, `SetOcgDefaults` (class `Document`)
- **`SaveConstraint`:** `Free` → incremental. Signed: class `Document` is a DocMDP violation at every certification level (ADR 0007 §2: P1 allows no changes; P2 allows only form filling, page templates and signing; P3 adds annotations); ops noted as `Annotation` are allowed under P3 or when there is no certification. Violations are gated (override names the signatures). With approval signatures only (no certification): `AppendOnly`, an incremental update that keeps their `/ByteRange` intact; a violation only when it changes a field locked by FieldMDP/`/Lock`
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `M.04` → `round-trip` (core, pdf, web, desktop); `I.08` → `round-trip` (core, pdf, web, desktop)

## Tasks

- [ ] Model
- [ ] Viewer toggles
- [ ] Panel

## Acceptance criteria

- [ ] Library parity ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 7): every new `Op` is exposed in `lamina` and `@lamina/pdf` in this PR, with rustdoc/TSDoc, a runnable example, tsify-generated TS types, and serde samples frozen in `fixtures/ops/v1/` (unknown ops and fields rejected, ADR 0006 §1, §6)
- [ ] Every new op reports exactly one `ModificationClass` and `requires_rewrite` (ADR 0006 §1); a test per op runs `planner.would_violate(&op)` on the signed fixtures (no certification, DocMDP P1, P2, P3, FieldMDP lock) and asserts the verdict ([ADR 0007](../../decisions/0007-save-planner.md) §2), because a misclassified op is a security bug
- [ ] Round-trip: open → op → save → reopen in lamina, pdf.js and PDFium preserves the change and everything else (page count, outline, annotations, fields, struct tree facts in `expected.json`); `qpdf --check` is clean
- [ ] Toggled defaults honoured in pdf.js

## Post-merge checks

None.

## Tests

- e2e

## Docs to update

- User help
- [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
