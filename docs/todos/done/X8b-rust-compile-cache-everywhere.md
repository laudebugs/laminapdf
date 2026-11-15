# X8b — sccache on R2: spread it to every Rust job, or remove it

| Field | Value |
| --- | --- |
| ID | `X8b` |
| Status | `done` |
| Plan | [CI caching §5](../../plans/ci-caching.md#5-rollout) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X8a](./X8a-rust-compile-cache-trial.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x8b-rust-compile-cache-everywhere` |
| PR title | `ci: compile Rust through the compile cache in the fast and full lanes` |
| Size | S |
| PR | [#653](https://github.com/laudebugs/laminapdf/pull/653) |

## Goal

X8a's trial shows how much sccache saves on the Linux x64, macOS x64 and macOS Arm64 Rust legs. This todo acts on those numbers. If the compile minutes of `cargo xtask check rust` on macOS Arm64 PR runs fell by a third or more in at least one Rust-cache state, compared within that state, and grew in neither (plan §5), every Rust job in `ci.yml` and `full.yml` uses the compile cache. Otherwise the action, the pin and the secrets go, and the plan records why.

## Context

- [CI caching plan §5](../../plans/ci-caching.md#5-rollout): the rule this todo applies.
- X8a's post-merge checks: the trial's runs and their `sccache --show-stats`.

## Scope

**In scope**

- The trial's numbers, in the plan's §2.
- The action in every Rust job of the fast and full lanes (keep), or its removal (remove).
- One leg without the Rust cache, measured (keep only).

**Out of scope** (and which todo covers it)

- The publishing, deploying and nightly workflows, which never use the compile cache (plan §4.3).

## Tasks

- [x] Record the trial's hit rates and step times in the plan's §2: on macOS Arm64, five PR runs in each Rust-cache state (a state with fewer than five doesn't count toward the rule). Step times in §2.2; the hit rates went only to job summaries, which no API returns.
- [x] Apply the rule: the patch for every Rust job, or the removal. Removed (the owner chose the rule, 2026-11-15).
- [x] If kept: check install-action has sccache for `aarch64-pc-windows-msvc` before that leg. It does (0.19.0), though the cache wasn't kept.
- [x] If kept: one leg without the Rust cache for five pushes to `main`; keep the faster setup unless its R2 reads exceed §4.4's estimate. Not needed: removed.

## Acceptance criteria

- [x] The plan's §2 holds the trial's numbers and §5 the decision.
- [x] If kept, every Rust job of the fast and full lanes uses the action, and the workflow checks still pass. Removed instead; the workflow checks pass without the compile cache's.

## Post-merge checks

- [x] A week of R2 request counts and storage, against §4.4's estimate. Not needed: the bucket was deleted on 2026-11-15.

## Tests

The workflow checks from X8a; if the action is removed, they go with it.

## Docs to update

The plan (status, §2, §5, to-dos), ADR 0018's amendment, `CLAUDE.md`.

## Risks and open questions

- Deployment entries on `main` (plan §6): if the list becomes noise, move the writes to one job.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
