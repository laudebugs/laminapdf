# Q2q — Split the COS reader module

| Field | Value |
| --- | --- |
| ID | `Q2q` |
| Status | `ongoing` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](../done/Q2a-maintainability-checks-rust.md), [Q2e](../done/Q2e-error-policy-consolidation.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2q-split-cos-reader` |
| PR title | `refactor(cos): split reader/mod.rs into page-tree and object-loading modules` |
| Size | M |
| PR | — |

## Goal

`laminapdf-cos/src/reader/mod.rs` is 1,533 lines (1,080 of code) and mixes opening and repair, object loading and the page-tree walk. This todo splits it along those seams with no API change and takes it off the maintainability baseline.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `laminapdf-cos/src/reader/mod.rs` (opening at :532, loading at :980–1210, the page tree at :60–490 and :1242–1450); [Q2e](../done/Q2e-error-policy-consolidation.md) has already moved the error policies out.

## Scope

**In scope**

- `reader/pages.rs` (the page tree) and `reader/load.rs` (dereferencing, loading, object streams, stream decoding); `mod.rs` keeps opening and repair; the baseline entry removed.

**Out of scope** (and which todo covers it)

- The content interpreter ([Q2w](../ready/Q2w-split-content-interpreter.md)); other large modules ([Q2r](../ready/Q2r-split-surface-modules.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Move the page-tree code in one commit with no edits; run the tests.
- [ ] Move the loading code in a second commit; run the tests.
- [ ] Tidy imports and visibility in a third commit.
- [ ] Run the differential and a short fuzz run of `open` and `page_tree`; remove the baseline entry.

## Acceptance criteria

- [ ] No behaviour change: tests, the differential and the reference files are unchanged.
- [ ] Every reader file is under 800 lines of non-test code.

## Post-merge checks

None.

## Tests

Existing suites, `cargo xtask differential`, a short fuzz run.

## Docs to update

Module docs, plan §9.

## Risks and open questions

A large move is hard to review: keep moves and edits in separate commits.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
