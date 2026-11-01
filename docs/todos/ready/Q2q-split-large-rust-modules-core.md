# Q2q — Split the largest core modules: the COS reader and the content interpreter

| Field | Value |
| --- | --- |
| ID | `Q2q` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2e](./Q2e-error-policy-consolidation.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2q-split-reader-interp` |
| PR title | `refactor(cos,content): split the reader module and the interpreter's execute` |
| Size | M |
| PR | — |

## Goal

`lamina-cos/src/reader/mod.rs` is 1,533 lines mixing opening and repair, object loading and the page-tree walk; `lamina-content/src/interp.rs`'s `execute` is 318 lines behind an `#[expect(too_many_lines)]`. Split both along their seams with no API change, and take them off the maintainability baseline.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina-cos/src/reader/mod.rs` (open :532, loading :980–1210, pages :60–490 and :1242–1450), `lamina-content/src/interp.rs:383`.

## Scope

**In scope**

- `reader/{pages.rs, load.rs}` (policies already moved by [Q2e](./Q2e-error-policy-consolidation.md)); `execute` dispatching per operator group (path, text, state, colour, marked content, XObject) to methods, the match a table of one-liners; remove the `#[expect]`; shrink the baseline.

**Out of scope** (and which todo covers it)

- Other large modules ([Q2r](./Q2r-split-large-rust-modules-surface.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Pure moves first, then the dispatch split; run every test, the differential and the fuzz smoke.

## Acceptance criteria

- [ ] No behaviour change (all tests, differential and goldens unchanged).
- [ ] Both files under the 800-line budget; the `#[expect]` is gone.

## Post-merge checks

None.

## Tests

Existing suites; `cargo xtask differential`; a short fuzz run of `open`, `page_tree` and `content`.

## Docs to update

Crate module docs, plan §9.

## Risks and open questions

Large diffs are hard to review: split moves and logic changes into separate commits.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
