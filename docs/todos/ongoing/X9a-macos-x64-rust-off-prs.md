# X9a — Rust on macOS x64 only on pushes to main and in the nightly

| Field | Value |
| --- | --- |
| ID | `X9a` |
| Status | `ongoing` |
| Plan | [CI caching §8](../../plans/ci-caching.md#8-the-waits) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X8a](../done/X8a-rust-compile-cache-trial.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) (the lanes: what runs on PRs, on `main` and nightly) |
| Branch | `ci/x9a-macos-x64-rust-off-prs` |
| PR title | `ci: run Rust on macOS x64 on pushes to main and in the nightly, not on PRs` |
| Size | S |
| PR | [#514](https://github.com/laudebugs/laminapdf/pull/514) |

## Goal

Pull requests stop waiting for a macOS x64 runner. In the plan's baseline, the macOS x64 Rust leg of `ci.yml` waited 27 to 120 minutes for a runner on PRs, against about 13 minutes of compiling, and it was the last Rust leg to finish. This todo runs that leg only on pushes to `main` and adds the same check to the nightly. A break that shows only on macOS x64 then appears after the merge, not before it. The risk is small, because the macOS Arm64 leg still runs on every PR, and the owner accepted it on 2026-11-12.

## Context

- [CI caching plan §2](../../plans/ci-caching.md#2-what-we-measured-2026-11-12) and §2.1: the waits and the macOS x64 baseline.
- `.github/workflows/ci.yml`'s `rust` job: its matrix, and the `compile-cache` flag X8a set on the macOS x64 leg.
- `.github/workflows/nightly.yml`: it runs no native Rust check today, and must not use the compile cache (plan §4.3).
- `tools/xtask/src/workflows.rs`: the tests that hold the workflows to their rules.

## Scope

**In scope**

- `ci.yml`: the macOS x64 leg leaves the matrix on pull requests (a matrix `exclude` keyed on the event), and stays on pushes to `main` with `compile-cache: true`, so it keeps filling the compile cache.
- `nightly.yml`: a job that runs `cargo xtask check rust` on `macos-15-intel`, without the compile cache, and that the nightly's report covers.
- A test in `tools/xtask/src/workflows.rs` that the leg is excluded only on pull requests, and that the nightly has the job.
- ADR 0018 §2 and `ci.yml`'s header comment: which legs run where.

**Out of scope** (and which todo covers it)

- Moving X8b's PR measurements from macOS x64 to macOS Arm64 → [X8b](../ready/X8b-rust-compile-cache-everywhere.md) and its owner's small PR before this one (agreed 2026-11-12).
- Render goldens on Mac runners → [X9b](X9b-render-goldens-when-rendering-changes.md).
- Other macOS jobs (WebKit, the desktop builds) stay as they are.

## Tasks

- [ ] `ci.yml`: exclude the `macos-x64` leg when `github.event_name == 'pull_request'`; keep its flags.
- [ ] `nightly.yml`: add the macOS x64 Rust job (`cargo xtask check rust`, the pinned toolchain and Rust cache, no compile cache) and list it where the nightly gathers its results.
- [ ] `tools/xtask/src/workflows.rs`: test both.
- [ ] ADR 0018 §2 and the comments in both workflows.

## Acceptance criteria

- [ ] A PR run of `ci.yml` has no `Rust (macos-x64)` job, and `fast-gate` passes without it.
- [ ] The workflow tests fail if the leg is dropped from pushes to `main` or the nightly loses its job.

## Post-merge checks

- [ ] The first push to `main` after the merge runs `Rust (macos-x64)`.
- [ ] The next nightly runs the macOS x64 job and its report includes it.

## Tests

`tools/xtask/src/workflows.rs`, as above.

## Docs to update

ADR 0018 §2; the CI caching plan's §8 (tick X9a); `ci.yml`'s and `nightly.yml`'s comments.

## Risks and open questions

- A macOS x64-only break lands on `main`. It shows on the next push to `main`, or that night; the fix is a normal PR.
- Merge only after the PR that moves X8b's trial measurements to macOS Arm64 has merged.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
