# Q2t — Names and comments without project history

| Field | Value |
| --- | --- |
| ID | `Q2t` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2t-names-not-history` |
| PR title | `refactor: name tests for behaviour; no todo IDs in src comments` |
| Size | S |
| PR | — |

## Goal

Test files are named after reviews and todos (`lamina-text/tests/review_b4b.rs`, `review_fixes.rs`, `lamina-build/tests/common/b5b.rs`, others) and 77 todo IDs sit in `src/` comments, where they explain nothing. Rename and fold the tests by behaviour, rewrite the comments to say why, and add the ban to `check maintainability`.

## Context

- [Maintainability guideline](../../guidelines/maintainability.md) rule 8; code-organization.md §6 (test names describe behaviour); grep `\(B\d+[a-z]?\)` and `review_` across `crates/` and `packages/`.

## Scope

**In scope**

- Rename/fold test files; rewrite each todo-ID comment (keep the *why*, drop the ID); add the regex ban (src only; commit messages and plans keep IDs).

**Out of scope** (and which todo covers it)

- Anything outside `crates/` and `packages/`.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Grep, rewrite, rename, run tests.

## Acceptance criteria

- [ ] No todo ID in `crates/*/src` or `packages/*/src`; no test file named after a review or todo; the ban runs in `check maintainability` (or a grep step if [Q2a](./Q2a-maintainability-checks-rust.md) hasn't landed).

## Post-merge checks

None.

## Tests

Existing suites.

## Docs to update

Plan §9.

## Risks and open questions

Some IDs carry context: move that context into the comment's words.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
