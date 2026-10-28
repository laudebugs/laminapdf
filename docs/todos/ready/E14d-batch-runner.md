# E14d — Batch runner over saved op sequences (local)

| Field | Value |
| --- | --- |
| ID | `E14d` |
| Status | `ready` |
| Plan | [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E6d](./E6d-watermarks-headers-footers.md), [E14a](./E14a-compare-revisions-batch-feature-plan.md) |
| Blocks | [E14e](./E14e-server-batch.md) |
| ADRs | Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e14d-batch-runner` |
| PR title | `feat(batch): apply saved op sequences to many files` |
| Size | M (200–600) |
| PR | — |

## Goal

Record an op sequence (ADR 0006 JSON with `v` envelope), parameterise it, and apply it to many files: desktop over folders (multi-threaded engine processes), web over selected files (sequential in workers), CLI-friendly in the library; with a per-file report.

## Context

- [ADR 0006](../../decisions/0006-op-log-and-undo.md) consequences (batch processing)
- [06 §9](../../research/pdf/06-feature-inventory.md#9-suggested-phasing) phase 6

## Scope

**In scope**

- Op-sequence recorder and editor
- Runner with per-file transactions and reports
- Library API and example

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `lamina-ops`, `apps/desktop`, `@lamina/ui`
- **`SaveConstraint`:** no new save behaviour beyond the round's: read-only views; batch outputs follow each op's rules
- **Web vs desktop:** desktop: folders, parallel; web: selected files, zip output
- **Server features:** none (this PR adds no server feature; all processing stays on the device)
- **Capability-matrix rows to flip:** no inventory row (06 §9 batch); documented in the matrix notes

## Tasks

- [ ] Recorder
- [ ] Runner
- [ ] UI
- [ ] Examples

## Acceptance criteria

- [ ] Applying Bates numbering to 100 files produces correct outputs and a report
- [ ] A failing file doesn't stop the batch

## Post-merge checks

None.

## Tests

- Integration and e2e

## Docs to update

- User help: batch
- [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- None known beyond the round's risks in the plan

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
