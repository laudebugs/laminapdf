# E17f — 3D detection with poster fallback; round close

| Field | Value |
| --- | --- |
| ID | `E17f` |
| Status | `ready` |
| Plan | [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E17b](./E17b-action-editor.md), [E17c](./E17c-layers.md), [E17d](./E17d-media.md), [E17e](./E17e-measurement.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e17f-3d-detection` |
| PR title | `feat(3d): detect 3D annotations and show posters` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

Detect 3D annotations (U3D/PRC), show the poster appearance and an "unsupported interactive 3D" notice; verify E17 rows.

## Context

- **Design (Paper):** [X12 Measurement and media](../../design/paper/convert-compare-produce.md#x12-measurement-and-media). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions)

## Scope

**In scope**

- Detection and report
- Capability check

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `M.06` embed 3D models
- **Crates and packages:** `laminapdf-model`, `@laminapdf/ui`
- **`SaveConstraint`:** no new save behaviour beyond the round's: incremental on `Free`
- **Web vs desktop:** identical
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `M.06` → `read` (core, pdf, web, desktop)

## Tasks

- [ ] Detection
- [ ] Notice
- [ ] Capability check

## Acceptance criteria

- [ ] 3D fixtures show posters and the notice

## Post-merge checks

None.

## Tests

- Fixtures

## Docs to update

- User help
- [full-editor §E17](../../plans/full-editor.md#e17-advanced-interactions) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
