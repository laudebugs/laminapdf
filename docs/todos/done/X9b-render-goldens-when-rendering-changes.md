# X9b — Render goldens only when a PR changes rendering

| Field | Value |
| --- | --- |
| ID | `X9b` |
| Status | `done` |
| Plan | [CI caching §8](../../plans/ci-caching.md#8-the-waits) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | None |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes); [ADR 0004](../../decisions/0004-render-backend.md) (the render goldens) |
| Branch | `ci/x9b-render-goldens-when-rendering-changes` |
| PR title | `ci: run the render goldens only when a PR changes what they draw` |
| Size | S |
| PR | [#514](https://github.com/laudebugs/laminapdf/pull/514) |

## Goal

`full.yml`'s `render-golden` job needs a macOS runner, so on most PRs it waits for one to check drawing the PR didn't touch. This todo runs it on a PR only when the PR changes what the goldens draw, and on every push to `main` as now.

## Context

- [CI caching plan §2](../../plans/ci-caching.md#2-what-we-measured-2026-11-12): waiting for a macOS runner costs 10–59 minutes per job.
- `full.yml`'s `render-golden` job and `cargo xtask render-golden` (`tools/xtask/src/render.rs`, `tools/render/`).
- `tools/xtask/src/changes.rs`: `touches_desktop` and `touches_web` are the pattern for a new flag.

## Scope

**In scope**

- `cargo xtask changes` prints and outputs `render`: true when a path can change what the goldens draw. That covers the render crates, the core crates they read through, `tools/render/`, `fixtures/corpus/`, `fixtures/golden/render*/`, the lockfiles and toolchain, and the job's workflow and actions. The PR decides the list from what the goldens build.
- `full.yml`: `render-golden` runs when `render` is true or on a push to `main`; `gate` treats it as skipped otherwise.
- Tests in `changes.rs` for the flag, as for the others.

**Out of scope** (and which todo covers it)

- Other jobs skipped by path → [X9c](X9c-skip-jobs-by-changed-paths.md).

## Tasks

- [x] `touches_render` and its input list in `changes.rs`, with the output.
- [x] `full.yml`: the job's `if`, and `gate`.
- [x] Tests, and the comments in `full.yml`.

## Acceptance criteria

- [x] A PR that changes only `apps/web/` or `docs/` skips `render-golden` and `gate` passes.
- [x] A PR that changes `crates/laminapdf-render/` or `fixtures/golden/render/` runs it.

## Post-merge checks

None.

## Tests

`tools/xtask/src/changes.rs`: which paths count as rendering, and which don't.

## Docs to update

The CI caching plan's §8 (tick X9b); ADR 0018 §2 if it lists the full lane's jobs.

## Risks and open questions

- A path missing from the list lets a drawing change through until the next push to `main`. Keep the list wide: any crate the render path reads counts.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
