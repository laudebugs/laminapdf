# Q1l — The docs checks become strict everywhere

| Field | Value |
| --- | --- |
| ID | `Q1l` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1f](./Q1f-examples-readable.md), [Q1i](./Q1i-guides-create.md), [Q1j](./Q1j-recipes-and-troubleshooting.md), [Q1k](./Q1k-internal-ids-out-of-reader-text.md), [Q1m](./Q1m-guides-read-and-edit.md) |
| Blocks | None |
| ADRs | None |
| Branch | `claude/working-session-9g3nst` |
| PR title | `ci(xtask): docs-prose and the examples checks are strict on every file` |
| Size | S |
| PR | [#219](https://github.com/laudebugs/laminapdf/pull/219) |

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

- [x] Claim by opening the work PR (#219) off an up-to-date `main`. The cloud session that did the work had a plain clone and an assigned branch, so it worked on that branch instead of a worktree.
- [x] Turn on the strict modes; run `cargo xtask check`; fix any stragglers. Five error-code pages had long sentences or a long paragraph; `examples/differential/text-facts.rs`, a developer tool, is now skipped by the examples checks, as `cargo xtask examples` skips it.
- [x] Show in the PR description that a bad page and a bad example each fail CI (on a scratch branch). The session could push only its assigned branch, so the PR shows the same failures from the local `cargo xtask check docs` and `biome ci`: an untouched error page with its old 65-word sentence, and an example with a todo ID in its header and a non-null assertion.

## Acceptance criteria

- [x] `cargo xtask check` passes with every docs check strict everywhere.

## Post-merge checks

- [ ] A maintainer updates the checkout comment of the Rust job in `.github/workflows/ci.yml`: it still gives `clippy-examples` as a reason for `fetch-depth: 0`, and only `check maintainability` needs the merge base now. The PR's session couldn't change workflow files (its token lacks the `workflow` scope).

## Tests

The checks' own tests.

## Docs to update

The docs-writing and examples guidelines, plan §9 (Q1 done).

## Risks and open questions

None.

## Definition of done

- [x] Claimed: the work PR moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] Work PR opened; **PR** field filled in. It came from the session's assigned branch `claude/working-session-9g3nst` in a plain clone, not from a worktree or a `ci/…` branch (see Tasks)
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
