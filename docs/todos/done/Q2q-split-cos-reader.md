# Q2q — Split the COS reader module

| Field | Value |
| --- | --- |
| ID | `Q2q` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](./Q2a-maintainability-checks-rust.md), [Q2e](./Q2e-error-policy-consolidation.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2q-split-cos-reader` |
| PR title | `refactor(cos): split reader/mod.rs into page-tree and object-loading modules` |
| Size | M |
| PR | [#211](https://github.com/laudebugs/laminapdf/pull/211) |

## Goal

`laminapdf-cos/src/reader/mod.rs` is 1,533 lines (1,080 of code) and mixes opening and repair, object loading and the page-tree walk. This todo splits it along those seams with no API change and takes it off the maintainability baseline.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `laminapdf-cos/src/reader/mod.rs` (opening at :532, loading at :980–1210, the page tree at :60–490 and :1242–1450); [Q2e](../done/Q2e-error-policy-consolidation.md) has already moved the error policies out.

## Scope

**In scope**

- `reader/pages.rs` (the page tree) and `reader/load.rs` (dereferencing, loading, object streams, stream decoding); `mod.rs` keeps opening and repair; the baseline entry removed.

**Out of scope** (and which todo covers it)

- The content interpreter ([Q2w](./Q2w-split-content-interpreter.md)); other large modules ([Q2r](../done/Q2r-split-surface-modules.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Move the page-tree code in one commit with no edits; run the tests.
- [x] Move the loading code in a second commit; run the tests.
- [x] Tidy imports and visibility in a third commit.
- [x] Run the differential and a short fuzz run of `open` and `page_tree` (the full lane's 30 s per target on this PR); remove the baseline entry.

## Acceptance criteria

- [x] No behaviour change: tests, the differential and the reference files are unchanged.
- [x] Every reader file is under 800 lines of non-test code.

## Post-merge checks

None.

## Tests

Existing suites, `cargo xtask differential`, a short fuzz run.

## Docs to update

Module docs, plan §9.

## Risks and open questions

A large move is hard to review: keep moves and edits in separate commits.

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated), per CLAUDE.md's current rule
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
