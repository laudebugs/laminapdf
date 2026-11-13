# X10a — Work out the desktop jobs' inputs from the dependency graph

| Field | Value |
| --- | --- |
| ID | `X10a` |
| Status | `done` |
| Plan | [CI caching §8.2](../../plans/ci-caching.md#82-desktop-legs-and-codeql-by-what-changed-x10) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X9c](./X9c-skip-jobs-by-changed-paths.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x10-desktop-ci-reuse` |
| PR title | `ci: run the desktop legs and CodeQL only for what changed` |
| Size | L |
| PR | [#522](https://github.com/laudebugs/laminapdf/pull/522) |

## Goal

The full lane's `desktop` flag counts every path under `crates/` and `packages/`, so a change to the CLI or the layout model builds and tests the desktop app on three systems. This todo works out the crates and packages the app builds from `cargo metadata` and the `package.json` files, so the flag covers only those.

## Context

- `tools/xtask/src/changes.rs` and its `desktop` flag (todo C4d's pattern, X9c's table in the plan's §8.1).
- `full.yml`'s `desktop` and `desktop-e2e` jobs: what they compile (the app, its engine, the WebAssembly the frontend's packages are built from), the fixtures the scenarios open, and the xtask tasks they run.

## Scope

**In scope**

- The crates reached through path dependencies from `laminapdf-desktop`, `laminapdf-desktop-engine` and the WebAssembly crates of `wasm::MODULES`.
- The packages reached through `workspace:` dependencies from `apps/desktop`.
- A fixed list for what no graph names: manifests, lockfiles, toolchains, shared configs, the fixtures the scenarios open, `tools/xtask` (whose tasks the jobs run), the workflow and its actions.

**Out of scope** (and which todo covers it)

- Skipping a leg that already passed → [X10b](X10b-reuse-desktop-legs-that-passed.md).

## Tasks

- [x] `changes/desktop.rs`: `inputs`, `touches`, with tests on synthetic graphs and on the repository.
- [x] `changes.rs` uses it in place of `DESKTOP_INPUTS`.

## Acceptance criteria

- [x] A change only to `crates/laminapdf-cli/` or `packages/site-kit/` sets `desktop=false`; a change to a crate the engine or the WebAssembly reaches sets it.

## Post-merge checks

None.

## Tests

`tools/xtask/src/changes/desktop.rs`.

## Docs to update

The plan's §8.1 table and §8.2; CLAUDE.md's `cargo xtask changes` entry.

## Risks and open questions

- A path the fixed list misses skips the desktop jobs on a PR until the next push to `main`. When unsure, a path counts.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, added this file to `ongoing/` in its first commit with `[skip ci]` (the owner asked for this work directly, so it never sat in `ready/`), **PR** field filled in, board regenerated
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
