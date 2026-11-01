# Q1l — Docs checks fail instead of warn

| Field | Value |
| --- | --- |
| ID | `Q1l` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1f](./Q1f-examples-readable.md), [Q1i](./Q1i-guides-plain-english.md), [Q1j](./Q1j-recipes-and-troubleshooting.md), [Q1k](./Q1k-internal-ids-out-of-reader-text.md) |
| Blocks | None |
| ADRs | None |
| Branch | `ci/q1l-docs-checks-strict` |
| PR title | `ci(xtask): docs-prose and docs-snippets fail on any report` |
| Size | S |
| PR | — |

## Goal

Once the rewrites have cleared every report, make the docs checks bind: `docs-prose` and `docs-snippets` run with `--strict` in `check docs`, the examples' Biome override uses `error`, and `clippy-examples` denies. From here on a PR that adds a page or example breaking the rules fails CI.

## Context

- [Quality plan §4](../../plans/quality.md#4-standards); [Q1b](./Q1b-docs-prose-check.md), [Q1c](./Q1c-examples-check.md).

## Scope

**In scope**

- Flip the levels; remove any temporary allowances; update the guidelines' "How it's checked" sections.

**Out of scope** (and which todo covers it)

- New rules (a later todo, if review shows a gap).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Flip, run `cargo xtask check`, fix stragglers.

## Acceptance criteria

- [ ] `cargo xtask check` passes with every docs check strict.
- [ ] A deliberately bad page or example in a scratch branch fails CI (shown in the PR description).

## Post-merge checks

None.

## Tests

The checks' own tests.

## Docs to update

The docs-writing and examples guidelines, plan §9 (round Q1 done).

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
