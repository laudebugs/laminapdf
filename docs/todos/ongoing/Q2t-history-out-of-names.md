# Q2t — Names and comments without project history

| Field | Value |
| --- | --- |
| ID | `Q2t` |
| Status | `ongoing` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](../done/Q2a-maintainability-checks-rust.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2t-history-out-of-names` |
| PR title | `refactor: name tests for behaviour; no todo IDs in src comments` |
| Size | S |
| PR | [#223](https://github.com/laudebugs/laminapdf/pull/223) |

## Goal

Test files are named after reviews and todos (`laminapdf-text/tests/review_b4b.rs`, `review_fixes.rs`, `laminapdf-build/tests/common/b5b.rs` and others), and 77 todo IDs sit in `src/` comments, where they explain nothing. This todo renames and folds the tests by behaviour, rewrites the comments to say why, and adds the ban to `check maintainability`.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("Names and comments"); code-organization §6 (test names describe behaviour); `grep -rnE '\(B[0-9]+[a-z]?\)'` and `review_` across `crates/` and `packages/`.

## Scope

**In scope**

- Test files renamed or folded into behaviour files; every todo-ID comment rewritten to keep its reason and drop its ID; a regex ban on todo IDs in `crates/*/src` and `packages/*/src`.

**Out of scope** (and which todo covers it)

- Anything outside `crates/` and `packages/`; commit messages and plans keep their IDs.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] List the files and comments with the grep.
- [ ] Rename or fold each test file; run its crate's tests.
- [ ] Rewrite each comment.
- [ ] Add the ban to `check maintainability`.

## Acceptance criteria

- [ ] No todo ID in `crates/*/src` or `packages/*/src`; no test file named after a review or todo; the ban passes.

## Post-merge checks

None.

## Tests

Existing suites.

## Docs to update

Plan §9.

## Risks and open questions

Some IDs carry context: move the context into the comment's words.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
