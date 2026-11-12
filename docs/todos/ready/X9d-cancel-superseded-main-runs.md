# X9d — A newer push to main cancels an older CI run on main

| Field | Value |
| --- | --- |
| ID | `X9d` |
| Status | `ready` |
| Plan | [CI caching §8](../../plans/ci-caching.md#8-the-waits) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | None |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x9d-cancel-superseded-main-runs` |
| PR title | `ci: let a newer push to main cancel the older run of the fast and full lanes` |
| Size | S |
| PR | — |

## Goal

`ci.yml` and `full.yml` queue pushes to `main` one at a time and never cancel one, so after a burst of merges each push waits for the runs before it: up to 43 minutes in the plan's survey. This todo lets a newer push to `main` cancel the older run, so `main` is checked at its newest commit as soon as runners allow.

## Context

- [CI caching plan §2](../../plans/ci-caching.md#2-what-we-measured-2026-11-12): the wait for `main`'s previous run.
- The `concurrency` blocks of `ci.yml` and `full.yml`, which cancel superseded runs on PRs only ("every push to main keeps its own result").
- What reads `main`'s runs: `release-train.yml` waits on the nightly, not on these lanes, and `cli-release-train.yml` on a tag push, so no release reads a push's own result. The rust and compile caches saved on `main` are saved again by the newer run.

## Scope

**In scope**

- `cancel-in-progress: true` for pushes to `main` in both lanes, and the comments that explain it.
- A test in `tools/xtask/src/workflows.rs` that both lanes cancel superseded runs.

**Out of scope** (and which todo covers it)

- The nightly, release and deploy workflows keep their own concurrency rules.

## Tasks

- [ ] Both `concurrency` blocks and their comments.
- [ ] The workflow test.
- [ ] ADR 0018 §2, if it says each push to `main` keeps its own result.

## Acceptance criteria

- [ ] Both lanes set `cancel-in-progress` to true for every event, and the workflow test checks it.

## Post-merge checks

- [ ] Two merges close together: the first push's runs are cancelled once the second starts, and the second runs to the end.

## Tests

`tools/xtask/src/workflows.rs`.

## Docs to update

The CI caching plan's §8 (tick X9d); ADR 0018 §2.

## Risks and open questions

- A commit on `main` whose runs were cancelled has no result of its own. A break it brought shows in the next push's run, and bisecting needs a manual rerun. The trade is the owner's (2026-11-12).

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
