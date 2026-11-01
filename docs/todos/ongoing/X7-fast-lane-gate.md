# X7 — A required check for the fast lane

| Field | Value |
| --- | --- |
| ID | `X7` |
| Status | `ongoing` |
| Plan | [architecture §7 cross-cutting](../../plans/architecture.md#7-to-dos) |
| Round / milestone | Cross-cutting |
| Depends on | [X4](../done/X4-full-lane-gate.md) |
| Blocks | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) §2 (CI lanes) |
| Branch | `ci/x7-fast-lane-gate` |
| PR title | `ci: a required check for the fast lane (X7)` |
| Size | S |
| PR | — |

## Goal

A PR that fails a fast-lane job can't merge. Today `gate` is the only required check, and it lives in `full.yml`, so a failure in `ci.yml` (the Rust, WASM, TypeScript, MSRV, spike and `pdf-runtimes` jobs) doesn't block a merge. B8b (#143) merged that way on 2026-11-01. Its fast-lane Rust job failed on a rustdoc lint, `gate` was green, and main went red until #149. #145, #147 and #151 also merged with the same red job.

## Context

- `.github/workflows/ci.yml` (its jobs skip on docs-only PRs through `changes`), `.github/workflows/full.yml` (`gate` and the comment at its top), [X4](../done/X4-full-lane-gate.md) (how `gate` was built).
- Branch protection on `main` requires only `gate` (`gh api repos/laudebugs/lamina/branches/main/protection/required_status_checks`).
- The merge scripts that the agents use now wait for every check run on the head commit, but that is a habit, not a rule. An `--admin` merge skips required checks anyway.

## Scope

**In scope**

- A `fast-gate` job in `ci.yml` that needs every other fast-lane job and passes only when each one passed or was skipped as not needed, on the same pattern as `gate` (a skipped `changes` job, or a result other than `success`/`skipped`, fails it).
- Make `fast-gate` a required check beside `gate`. This is an owner step: the repository settings, or `gh api -X PATCH repos/laudebugs/lamina/branches/main/protection/required_status_checks -f 'contexts[]=gate' -f 'contexts[]=fast-gate'`.
- A comment at the top of `ci.yml`, like `full.yml`'s, saying that a new job joins `fast-gate`'s `needs`.
- A unit test in `tools/xtask` (beside the release-lane test that reads `nightly.yml`) that fails when a `ci.yml` job is missing from `fast-gate`'s `needs`. The same check for `full.yml`'s `gate` if it's cheap.

**Out of scope**

- Changing what either lane runs.
- Windows ([X6](../ready/X6-windows-ci.md)).

## Tasks

- [ ] Claim via a claim PR; then a fresh worktree off the updated `main`.
- [ ] Add `fast-gate` and the header comment.
- [ ] Add the `needs` test.
- [ ] Run one code PR and one docs-only PR through it: `fast-gate` is green on both, and it goes red when one fast-lane job is made to fail (a throwaway commit, reverted).
- [ ] Ask the owner to add `fast-gate` to the required checks.

## Acceptance criteria

- [ ] With `fast-gate` required, a PR with a red fast-lane job shows "Merging is blocked".
- [ ] A docs-only PR still merges with the fast lane's jobs skipped.

## Post-merge checks

- [ ] The owner has made `fast-gate` required, and the next code PR shows it among the required checks.

## Tests

The `needs` test in `tools/xtask`; the throwaway red commit above.

## Docs to update

ADR 0018's implementation note, CLAUDE.md (the CI paragraph: "Its `gate` job is the required check"), docs/guidelines/git-workflow.md §7 if it names the required check, the architecture plan's to-do list.

## Risks and open questions

- `pdf-runtimes` is a matrix: `fast-gate` must see the matrix's combined result, which `needs` gives.
- A job that a later PR adds to `ci.yml` but leaves out of `needs` doesn't block merges; the `needs` test catches that.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
