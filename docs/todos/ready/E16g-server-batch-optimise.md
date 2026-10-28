# E16g — Optional server batch optimisation

| Field | Value |
| --- | --- |
| ID | `E16g` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E14e](./E14e-server-batch.md), [E16b](./E16b-structural-optimiser.md) |
| Blocks | None |
| ADRs | Implements ADR 0013; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e16g-server-batch-optimise` |
| PR title | `feat(jobs): optimise as a server batch job` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

Register `optimise` (and `convert_pdfa`) as job types in the opt-in server batch (E14e) so large print files can be optimised in Containers.

## Context

- ADR 0013
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) server

## Scope

**In scope**

- Job types
- UI entry

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `apps/jobs`
- **`SaveConstraint`:** no new save behaviour beyond the round's: `RequiresRewrite` for optimisation; signed files warned
- **Web vs desktop:** web offers it; desktop runs locally
- **Server features:** Containers via Workflows (opt-in)
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Job types
- [ ] Tests

## Acceptance criteria

- [ ] Preview e2e: optimise a 200 MB fixture via the server

## Tests

- Worker tests

## Docs to update

- Privacy page
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Skip if E14a decides batch is local only

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
