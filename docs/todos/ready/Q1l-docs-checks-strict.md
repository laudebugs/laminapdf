# Q1l — The docs checks become strict everywhere

| Field | Value |
| --- | --- |
| ID | `Q1l` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1f](./Q1f-examples-readable.md), [Q1i](./Q1i-guides-create.md), [Q1j](./Q1j-recipes-and-troubleshooting.md), [Q1k](../done/Q1k-internal-ids-out-of-reader-text.md), [Q1m](./Q1m-guides-read-and-edit.md) |
| Blocks | None |
| ADRs | None |
| Branch | `ci/q1l-docs-checks-strict` |
| PR title | `ci(xtask): docs-prose and the examples checks are strict on every file` |
| Size | S |
| PR | — |

## Goal

Once the rewrites have cleared every report, this todo makes the docs checks strict on every file, not only changed ones: `docs-prose` and `docs-snippets` run with `--all-strict` in `check docs`, the examples' Biome override reports errors, and `clippy-examples` denies.

## Context

- [Quality plan §4](../../plans/quality.md#4-standards); [Q1b](../done/Q1b-docs-prose-check.md), [Q1c](../done/Q1c-examples-checks.md).

## Scope

**In scope**

- Turn on `--all-strict` and the error levels; remove temporary allowances; update the guidelines' "How it's checked" sections.

**Out of scope** (and which todo covers it)

- New rules: a later todo if review finds a gap.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Turn on the strict modes; run `cargo xtask check`; fix any stragglers.
- [ ] Show in the PR description that a bad page and a bad example each fail CI (on a scratch branch).

## Acceptance criteria

- [ ] `cargo xtask check` passes with every docs check strict everywhere.

## Post-merge checks

None.

## Tests

The checks' own tests.

## Docs to update

The docs-writing and examples guidelines, plan §9 (Q1 done).

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
