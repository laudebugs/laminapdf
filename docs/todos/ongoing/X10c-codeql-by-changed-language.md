# X10c — Run each CodeQL language only when a PR changes its files

| Field | Value |
| --- | --- |
| ID | `X10c` |
| Status | `ongoing` |
| Plan | [CI caching §8.2](../../plans/ci-caching.md#82-desktop-legs-and-codeql-by-what-changed-x10) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | [X9c](../done/X9c-skip-jobs-by-changed-paths.md) |
| ADRs | [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes) |
| Branch | `ci/x10-desktop-ci-reuse` |
| PR title | `ci: run the desktop legs and CodeQL only for what changed` |
| Size | S |
| PR | — |

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
