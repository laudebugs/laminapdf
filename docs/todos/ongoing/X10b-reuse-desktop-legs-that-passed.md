# X10b — Skip a desktop leg that passed on the same inputs

| Field | Value |
| --- | --- |
| ID | `X10b` |
| Status | `ongoing` |
| Plan | [CI caching §8.2](../../plans/ci-caching.md#82-desktop-legs-and-codeql-by-what-changed-x10) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X10a](./X10a-desktop-inputs-from-the-graph.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x10-desktop-ci-reuse` |
| PR title | `ci: run the desktop legs and CodeQL only for what changed` |
| Size | M |
| PR | — |

## Goal

A pull request runs the desktop build and e2e legs again whenever the full lane reruns: after a description edit, a rerun of an unrelated failure, or a merge from `main` that touched nothing the app reads. Each leg takes 4 to 11 minutes, and macOS legs wait 19 minutes or more for a runner. This todo names the desktop's inputs by a key and leaves out each leg that already passed on the same key.

## Context

- X10a's inputs.
- The GitHub Actions cache: a PR's run restores entries saved by the same PR or by `main`, and the REST API lists entries by key prefix and ref.

## Scope

**In scope**

- `cargo xtask desktop-key`: a digest of `git ls-tree -r HEAD` over the inputs, the same on every OS; `changes` prints it as `desktop-key`.
- Each passing leg saves an empty entry, `desktop-passed-<key>-<build|e2e>-<os>`.
- The `changes` job lists the entries for the PR's ref and `main`, and gives each job the legs without one; `gate` holds each job to its own flag.

**Out of scope** (and which todo covers it)

- macOS desktop legs off pull requests: the owner kept them (2026-11-12).

## Tasks

- [x] `desktop::key`, `print_key` and the `desktop-key` task.
- [x] `full.yml`: the lookup step, the legs as outputs, the matrices, the save steps, the gate.
- [x] Workflow tests: the saved names match the lookup; the gate's flags.

## Acceptance criteria

- [x] A rerun of a PR whose desktop legs passed runs none of them, and `gate` passes.

## Post-merge checks

- A PR's full lane, rerun after its desktop legs passed, skips them.

## Tests

`tools/xtask/src/workflows.rs`.

## Docs to update

The plan's §8.2; ADR 0018 §2; CLAUDE.md; `full.yml`'s header.
