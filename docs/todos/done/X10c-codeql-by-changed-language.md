# X10c — Run each CodeQL language only when a PR changes its files

| Field | Value |
| --- | --- |
| ID | `X10c` |
| Status | `done` |
| Plan | [CI caching §8.2](../../plans/ci-caching.md#82-desktop-legs-and-codeql-by-what-changed-x10) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X9c](./X9c-skip-jobs-by-changed-paths.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x10-desktop-ci-reuse` |
| PR title | `ci: run the desktop legs and CodeQL only for what changed` |
| Size | S |
| PR | [#522](https://github.com/laudebugs/laminapdf/pull/522) |

## Goal

CodeQL analyses four languages on every pull request, a docs-only one included. The owner asked that each run only when the PR changes files in its language (2026-11-12).

## Context

- `.github/workflows/codeql.yml`; CodeQL isn't a required check, so a skipped language leaves nothing waiting.

## Scope

**In scope**

- `cargo xtask changes`' `codeql` list: `actions` for workflows and actions, `javascript-typescript` for scripts and web pages, `python` for Python, `rust` for Rust files and the Cargo manifests; all four when `codeql.yml` changes; none for docs.
- `codeql.yml`: a `changes` job, and the matrix from its list. Pushes to `main` and the weekly schedule analyse all four.

**Out of scope** (and which todo covers it)

- None.

## Tasks

- [x] `codeql_languages` with tests.
- [x] `codeql.yml`'s `changes` job and matrix; a test that its full list is `CODEQL_LANGUAGES`.

## Acceptance criteria

- [x] A docs-only PR runs no CodeQL analysis; a TypeScript-only PR runs only `javascript-typescript`.

## Post-merge checks

- A docs-only PR shows no `Analyze` job.

## Tests

`tools/xtask/src/changes.rs`, `tools/xtask/src/workflows.rs`.

## Docs to update

The plan's §8.2; CLAUDE.md's `cargo xtask changes` entry.

## Risks and open questions

- A script under `docs/` is analysed only on its merge to `main`.
- Code scanning may warn on a PR that a skipped language's analysis is missing; harmless while no ruleset requires code scanning.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, added this file to `ongoing/` in its first commit with `[skip ci]` (the owner asked for this work directly, so it never sat in `ready/`), **PR** field filled in, board regenerated
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
