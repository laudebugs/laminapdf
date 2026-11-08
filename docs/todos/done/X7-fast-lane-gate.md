# X7 — A required check for the fast lane

| Field | Value |
| --- | --- |
| ID | `X7` |
| Status | `done` |
| Plan | [architecture §7 cross-cutting](../../plans/architecture.md#7-to-dos) |
| Round / milestone | Cross-cutting |
| Depends on | [X4](./X4-full-lane-gate.md) |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (CI lanes) |
| Branch | `ci/x7-fast-lane-gate` |
| PR title | `ci: a required check for the fast lane (X7)` |
| Size | S |
| PR | [#165](https://github.com/laudebugs/lamina/pull/165) |

## Goal

A PR that fails a fast-lane job can't merge. Today `gate` is the only required check, and it lives in `full.yml`, so a failure in `ci.yml` (the Rust, WASM, TypeScript, MSRV, spike and `pdf-runtimes` jobs) doesn't block a merge. B8b (#143) merged that way on 2026-11-01. Its fast-lane Rust job failed on a rustdoc lint, `gate` was green, and `main` went red until #149. #145, #147 and #151 also merged with the same red job.

## Context

- `.github/workflows/ci.yml` (its jobs skip on docs-only PRs through `changes`), `.github/workflows/full.yml` (`gate` and the comment at its top), [X4](../done/X4-full-lane-gate.md) (how `gate` was built).
- Branch protection on `main` requires only `gate` (`gh api repos/laudebugs/lamina/branches/main/protection/required_status_checks`).
- The merge scripts that the agents use now wait for every check run on the head commit, but that is a habit, not a rule. An `--admin` merge skips required checks anyway.

## Scope

**In scope**

- A `fast-gate` job in `ci.yml` that needs every other fast-lane job and passes only when each one passed or was skipped as not needed, on the same pattern as `gate`: a `changes` job that didn't succeed, a `code` output other than `true` or `false`, or any result other than `success` (code PR) or `skipped` (docs-only PR) fails it.
- Make `fast-gate` a required check beside `gate`. This is an owner step: the repository settings, or the PATCH in PR #165's description. The PATCH sets `checks` with GitHub Actions' `app_id` 15368, as the live protection does for `gate`.
- A comment at the top of `ci.yml`, like `full.yml`'s, saying that a new job joins `fast-gate`'s `needs`.
- A unit test in `tools/xtask` (beside the release-lane test that reads `nightly.yml`) that fails when a `ci.yml` job is missing from `fast-gate`'s `needs`. The same check for `full.yml`'s `gate` if it's cheap.

**Out of scope**

- Changing what either lane runs.
- Windows ([X6](../done/X6-windows-ci.md)).

## Tasks

- [x] Claim: this work PR's first commit moved the todo to `ongoing/`. On 2026-11-01, after this todo was written, the owner replaced claim PRs on `main` with claims in the work PR.
- [x] Add `fast-gate` and the header comment.
- [x] Add the `needs` test.
- [x] Run one code PR through it (this PR's CI). The docs-only run and a real red run moved to the post-merge checks. Instead of a throwaway red commit, which costs a full CI run, the gate's script was run locally against `needs` results for each case: all green, docs-only skips, a failed, cancelled or skipped job on a code PR, and a failed `changes` job.
- [x] Ask the owner to add `fast-gate` to the required checks (the command is in PR #165's description).

## Acceptance criteria

- [ ] With `fast-gate` required, a PR with a red fast-lane job shows "Merging is blocked".
- [ ] A docs-only PR still merges with the fast lane's jobs skipped.

## Post-merge checks

- [x] The owner has made `fast-gate` required (branch protection lists `gate` and `fast-gate`, 2026-11-01), and the next code PR shows it among the required checks. PR #173 dropped the notes saying it wasn't required yet from CLAUDE.md and git-workflow §5. The workflow comments in `ci.yml` and `full.yml` are written as true after this step.
- [ ] The next docs-only PR shows `fast-gate` green with every fast-lane job skipped.
- [ ] The first PR with a red fast-lane job shows `fast-gate` red: this checks GitHub's own `needs` results, which the local run only imitated.

## Tests

The `needs` test in `tools/xtask/src/workflows.rs`; the gate's script run locally against each case (see Tasks); the post-merge checks.

## Docs to update

ADR 0018's implementation note, CLAUDE.md (the CI paragraph: "Its `gate` job is the required check"), docs/guidelines/git-workflow.md §5 if it names the required check, the architecture plan's to-do list.

## Risks and open questions

- `pdf-runtimes` is a matrix: `fast-gate` must see the matrix's combined result, which `needs` gives.
- A job that a later PR adds to `ci.yml` but leaves out of `needs` doesn't block merges; the `needs` test catches that.

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
