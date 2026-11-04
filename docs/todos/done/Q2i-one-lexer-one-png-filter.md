# Q2i — One PDF lexer and one PNG row filter

| Field | Value |
| --- | --- |
| ID | `Q2i` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](./Q2a-maintainability-checks-rust.md) |
| Blocks | [Q2r](../ready/Q2r-split-surface-modules.md) |
| ADRs | None |
| Branch | `refactor/q2i-one-lexer-one-png-filter` |
| PR title | `refactor(text,build): CMap parsing on laminapdf-syntax's lexer; PNG unfiltering from laminapdf-syntax` |
| Size | M |
| PR | [#212](https://github.com/laudebugs/laminapdf/pull/212) |

## Goal

`laminapdf-text/src/extract/cmap.rs` has its own 150-line PDF lexer that skips `#xx` name decoding and ignores `Limits::max_string_length`, although laminapdf-syntax's `Lexer` is a dependency and laminapdf-content already wraps it tolerantly. laminapdf-build's PNG decoder re-implements the row filters laminapdf-syntax has for Flate predictors. This todo reuses both.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `laminapdf-text/src/extract/cmap.rs:285-455`, `laminapdf-syntax/src/lexer.rs`, `laminapdf-content/src/lexer.rs:7` (the tolerant wrapper to copy), `laminapdf-build/src/surface/png.rs:353,376`, `laminapdf-syntax/src/filters.rs:180,223`.

## Scope

**In scope**

- CMap tokens from a tolerant wrapper over `laminapdf_syntax::Lexer` (on an error, skip one byte), with a test for a `#xx` name in a CMap.
- `pub fn unfilter_row(filter, raw, prev, out, bpp)` in laminapdf-syntax, used by `png.rs` (inflating stays separate: exact size versus budget).
- A ban in `check maintainability`: no lexer outside laminapdf-syntax (the `one-lexer` rule, which also bans a second PNG row filter).

**Out of scope** (and which todo covers it)

- Splitting `png.rs` ([Q2r](../ready/Q2r-split-surface-modules.md)).

## Tasks

- [x] Claim: the work PR's first commit moves this file to `ongoing/` (CLAUDE.md's current rule), from a fresh worktree off `main`.
- [x] Write the `#xx` test and watch it fail.
- [x] Replace the CMap lexer with the wrapper; run the CMap tests and the extraction facts.
- [x] Export `unfilter_row`; switch `png.rs`; run the image reference PDFs.
- [x] Add the ban.

## Acceptance criteria

- [x] The extraction facts and image reference PDFs are unchanged, or any change is explained.
- [x] About 150 fewer lines in laminapdf-text; one row-filter implementation.

## Post-merge checks

None.

## Tests

Existing CMap and PNG tests; the new `#xx` test.

## Docs to update

Plan §9.

## Risks and open questions

Stricter limits could reject malformed CMaps that worked before: keep skipping and warning as now.

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
