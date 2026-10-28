# E16g — Optional server batch optimisation

| Field | Value |
| --- | --- |
| ID | `E16g` |
| Status | `ready` |
| Plan | [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) |
| Round / milestone | Editor phase 6 (E11–E17) — M9 Advanced |
| Depends on | [E14e](./E14e-server-batch.md), [E16b](./E16b-structural-optimiser.md), [E16e](./E16e-preflight-and-conversion.md) |
| Blocks | None |
| ADRs | Implements ADR 0013; Follows [0005](../../decisions/0005-document-model.md), [0006](../../decisions/0006-op-log-and-undo.md), [0007](../../decisions/0007-save-planner.md) |
| Branch | `feat/e16g-server-batch-optimise` |
| PR title | `feat(jobs): optimise as a server batch job` |
| Size | S (< 200 lines) |
| PR | — |

## Goal

Register `optimise` and `convert_pdfa` (the PDF/A conversion from [E16e](E16e-preflight-and-conversion.md)) as job types in the opt-in server batch (E14e) so large print files can be optimised in Containers. Closed as won't-do together with E14e if there is no hosted batch (see Risks).

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

## Post-merge checks

None.

## Tests

- Worker tests

## Docs to update

- Privacy page
- [full-editor §E16](../../plans/full-editor.md#e16-optimisation-and-print-production) and its §10 to-dos: tick what this PR completes and point the round's todo link at `done/`

## Risks and open questions

- If E14a decides batch is local only, [E14e](E14e-server-batch.md) is closed as won't-do (moved to `done/` with Status `done` and a `Closed as won't-do` note) and this todo is closed the same way in that PR, with the note `Closed as won't-do: no hosted batch (E14e closed as won't-do); optimisation runs locally`. It is never left in `ready/` waiting for a dependency that will not be built
- **Won't-do close (how the board records it):** the board has no separate won't-do state. A todo closed as won't-do is moved `ready/` → `done/` with Status `done`, a first line in its Goal reading `Closed as won't-do: <reason and link to the decision>`, the PR field pointing at the PR that closed it, and no tasks ticked. Its dependants' "Depends on" are then satisfied, so they show as startable and are either done normally or closed the same way.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
