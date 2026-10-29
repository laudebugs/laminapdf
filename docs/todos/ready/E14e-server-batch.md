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

Run saved op sequences on the server for many files (Worker running `lamina-wasm` for small files, Containers for large), opt-in, with retention rules, if the E14a answer to full-editor §9 Q3 keeps a hosted batch. If it does not, this todo is closed as won't-do (see Risks) rather than left in `ready/`.

## Context

- **Design (Paper):** [X05 Batch runner](../../design/paper/convert-compare-produce.md#x05-batch-runner). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
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

## Post-merge checks

None.

## Tests

- Worker tests

## Docs to update

- Privacy page
- [full-editor §E14](../../plans/full-editor.md#e14-compare-revisions-and-batch) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- Depends on the §9 Q3 answer; if batch stays local only, the E14a PR closes this todo as won't-do (below), and [E16g](E16g-server-batch-optimise.md), its only dependant, is closed as won't-do in the same PR
- **Won't-do close (how the board records it):** the board has no separate won't-do state. A todo closed as won't-do is moved `ready/` → `done/` with Status `done`, a first line in its Goal reading `Closed as won't-do: <reason and link to the decision>`, the PR field pointing at the PR that closed it, and no tasks ticked. Its dependants' "Depends on" are then satisfied, so they show as startable and are either done normally or closed the same way.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
