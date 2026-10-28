# E14e — Optional server batch via Workflows

| Field | Value |
| --- | --- |
| ID | `E14e` |
| Status | `ready` |
| Plan | [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E13e](./E13e-server-jobs-infrastructure.md), [E14d](./E14d-batch-runner.md) |
| Blocks | [E16g](./E16g-server-batch-optimise.md) |
| ADRs | Implements ADR 0013; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e14e-server-batch` |
| PR title | `feat(jobs): opt-in server batch jobs` |
| Size | M (200–600) |
| PR | — |

## Goal

Run saved op sequences on the server for many files (Worker running `lamina-wasm` for small files, Containers for large), opt-in, with retention rules, if the E14a answer to full-editor §9 Q3 keeps a hosted batch.

## Context

- ADR 0013
- [full-editor: server features](../../plans/full-editor.md#optional-cloudflare-server-features)

## Scope

**In scope**

- Job type `apply_ops` with size routing
- Upload/download of file sets
- Opt-in UI on web

**Out of scope** (and which todo covers it)

- None

**Editor facts**

- **Crates and packages:** `apps/jobs`
- **`SaveConstraint`:** no new save behaviour beyond the round's: read-only views; batch outputs follow each op's rules
- **Web vs desktop:** web offers it; desktop batches locally
- **Server features:** Workflows, Queues, Worker with `lamina-wasm`, Containers
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] Job type
- [ ] Routing
- [ ] UI

## Acceptance criteria

- [ ] Preview e2e: batch of 20 files processed and expired per retention

## Tests

- Worker tests

## Docs to update

- Privacy page
- [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Depends on the §9 Q3 answer; if batch stays local only, close this todo as won't-do in the E14a PR

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
