# E4g — Cross-viewer appearance parity suite and E4 round close

| Field | Value |
| --- | --- |
| ID | `E4g` |
| Status | `ready` |
| Plan | [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) |
| Round / milestone | Editor phase 3 (E4–E5) — M6 Annotator |
| Depends on | [E4f](./E4f-annotation-tools-ui.md), [X3a](./X3a-nightly-lane.md) |
| Blocks | None |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `test/e4g-appearance-parity-suite` |
| PR title | `test(annots): cross-viewer appearance parity and round close` |
| Size | M (200–600) |
| PR | — |

## Goal

Plan to-do "Round-trip tests: appearance identical in pdf.js and PDFium" as a standing suite: every subtype × property matrix rendered in pdf.js, PDFium and hayro, compared against goldens, run nightly; plus annotations on corpus files edited and re-saved; then verify all E4 rows.

## Context

- [ADR 0018](../../decisions/0018-test-strategy.md): differential tier
- [04 §2.4 Why appearance streams matter](../../research/pdf/04-interactive-security.md#24-why-appearance-streams-matter): lamina always writes `/AP` for what it creates ([full-editor §5](../../plans/full-editor.md#5-requirements-that-hold-for-every-round) req. 3)

## Scope

**In scope**

- Matrix generator (subtype × colour × opacity × border × rotation of page)
- Three-renderer diff harness with thresholds
- Corpus edit test (move every annotation by 1 pt and save)
- Capability verification

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Operations shipped** ([06 §3](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms) IDs): `D.16` generate appearance streams (cross-cutting)
- **Crates and packages:** tests and `tools/xtask` lane config
- **`SaveConstraint`:** incremental saves on Free fixtures
- **Web vs desktop:** n/a (core suite)
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** `D.16` → `round-trip` (core, pdf)

## Tasks

- [ ] Harness
- [ ] Thresholds per renderer
- [ ] Nightly lane wiring
- [ ] Capability check

## Acceptance criteria

- [ ] Suite green when run locally and via `workflow_dispatch` on the PR branch; failures produce diff images as artefacts
- [ ] All E4 rows at target status

## Post-merge checks

- [ ] The suite is green in the first scheduled nightly run on `main` (link it in a follow-up PR or the done todo)

## Tests

The suite itself.

## Docs to update

- `docs/guidelines` testing note if thresholds are new
- [full-editor §E4](../../plans/full-editor.md#e4-markup-and-drawing-annotations) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`
- `docs/capabilities/manifest.toml` statuses (regenerated `docs/capabilities/README.md` via `cargo xtask capabilities`)

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
