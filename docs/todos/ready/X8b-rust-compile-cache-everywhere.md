# X8b — sccache on R2: spread it to every Rust job, or remove it

| Field | Value |
| --- | --- |
| ID | `X8b` |
| Status | `ready` |
| Plan | [CI caching §5](../../plans/ci-caching.md#5-rollout) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X8a](../done/X8a-rust-compile-cache-trial.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x8b-rust-compile-cache-everywhere` |
| PR title | `ci: compile Rust through the compile cache in the fast and full lanes` |
| Size | S |
| PR | — |

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

- [ ] Record the trial's hit rates and step times in the plan's §2: on macOS Arm64, five PR runs in each Rust-cache state (a state with fewer than five doesn't count toward the rule).
- [ ] Apply the rule: the patch for every Rust job, or the removal.
- [ ] If kept: check install-action has sccache for `aarch64-pc-windows-msvc` before that leg.
- [ ] If kept: one leg without the Rust cache for five pushes to `main`; keep the faster setup unless its R2 reads exceed §4.4's estimate.

## Acceptance criteria

- [ ] The plan's §2 holds the trial's numbers and §5 the decision.
- [ ] If kept, every Rust job of the fast and full lanes uses the action, and the workflow checks still pass.

## Post-merge checks

- [ ] A week of R2 request counts and storage, against §4.4's estimate.

## Tests

The workflow checks from X8a; if the action is removed, they go with it.

## Docs to update

The plan (status, §2, §5, to-dos), ADR 0018's amendment, `CLAUDE.md`.

## Risks and open questions

- Deployment entries on `main` (plan §6): if the list becomes noise, move the writes to one job.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
