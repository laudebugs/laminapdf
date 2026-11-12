# X9c — Skip more jobs by the paths a PR changes

| Field | Value |
| --- | --- |
| ID | `X9c` |
| Status | `ready` |
| Plan | [CI caching §8](../../plans/ci-caching.md#8-the-waits) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X9b](./X9b-render-goldens-when-rendering-changes.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x9c-skip-jobs-by-changed-paths` |
| PR title | `ci: skip the jobs a PR's changed paths can't affect` |
| Size | M |
| PR | — |

## Goal

Today `cargo xtask changes` sorts a PR into docs-only or code, plus three narrower flags (`gallery`, `web`, `desktop`), so a PR that changes one TypeScript package still runs every Rust leg, and a Rust-only PR runs the browser jobs. This todo adds flags for the other large groups of jobs and skips each group when no changed path can affect it, on PRs only.

## Context

- `tools/xtask/src/changes.rs` and its tests; X9b's `render` flag.
- `ci.yml`'s and `full.yml`'s jobs, their `if`s, and `fast-gate` and `gate`, which must treat a skipped job as passing.

## Scope

**In scope**

- A survey of the jobs in both lanes: what each reads, and the paths that can change its result. The PR records the table in the plan's §8.
- New flags for the groups the survey finds worth it, at least `rust` (the native Rust legs) and `ts` (the TypeScript and browser jobs), each with tests.
- The jobs' `if`s on PRs; pushes to `main` run everything, as now.

**Out of scope** (and which todo covers it)

- The render goldens → [X9b](X9b-render-goldens-when-rendering-changes.md).
- Caching task results (Turborepo, Nx): ADR 0015 left that until TypeScript time shows on the critical path.

## Tasks

- [ ] The survey table.
- [ ] The flags in `changes.rs`, with their input lists and tests.
- [ ] The `if`s, and the gates.

## Acceptance criteria

- [ ] A PR that changes only `apps/web/` runs no native Rust leg; a PR that changes only a Rust crate's source still runs the jobs that build the WebAssembly from it.
- [ ] Each flag has tests for paths in and out of it.

## Post-merge checks

None.

## Tests

`tools/xtask/src/changes.rs`, and the workflow tests in `workflows.rs` if they check the gates' `needs`.

## Docs to update

The CI caching plan's §8 (the survey; tick X9c); `ci.yml`'s header comment, which says how a new job is gated.

## Risks and open questions

- A list too narrow lets a break through until `main`. When unsure, a path counts for the group.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
