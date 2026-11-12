# X10a — Work out the desktop jobs' inputs from the dependency graph

| Field | Value |
| --- | --- |
| ID | `X10a` |
| Status | `ongoing` |
| Plan | [CI caching §8.2](../../plans/ci-caching.md#82-desktop-legs-and-codeql-by-what-changed-x10) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X9c](../done/X9c-skip-jobs-by-changed-paths.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x10-desktop-ci-reuse` |
| PR title | `ci: run the desktop legs and CodeQL only for what changed` |
| Size | M |
| PR | — |

## Goal

The full lane's `desktop` flag counts every path under `crates/` and `packages/`, so a change to the CLI or the layout model builds and tests the desktop app on three systems. This todo works out the crates and packages the app builds from `cargo metadata` and the `package.json` files, so the flag covers only those.

## Context

- `tools/xtask/src/changes.rs` and its `desktop` flag (todo C4d's pattern, X9c's table in the plan's §8.1).
- `full.yml`'s `desktop` and `desktop-e2e` jobs: what they compile (the app, its engine, the WebAssembly the frontend's packages are built from), the fixtures the scenarios open, and the xtask tasks they run.

## Scope

**In scope**

- The crates reached through path dependencies from `laminapdf-desktop`, `laminapdf-desktop-engine` and the WebAssembly crates of `wasm::MODULES`.
- The packages reached through `workspace:` dependencies from `apps/desktop`.
- A fixed list for what no graph names: manifests, lockfiles, toolchains, shared configs, the fixtures the scenarios open, the xtask modules the jobs run, the workflow and its actions.

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
