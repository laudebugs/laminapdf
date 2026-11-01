# Q2i — One PDF lexer and one PNG row filter

| Field | Value |
| --- | --- |
| ID | `Q2i` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](../ongoing/Q2a-maintainability-checks-rust.md) |
| Blocks | [Q2r](./Q2r-split-surface-modules.md) |
| ADRs | None |
| Branch | `refactor/q2i-one-lexer-one-png-filter` |
| PR title | `refactor(text,build): CMap parsing on lamina-syntax's lexer; PNG unfiltering from lamina-syntax` |
| Size | M |
| PR | — |

## Goal

`lamina-text/src/extract/cmap.rs` has its own 150-line PDF lexer that skips `#xx` name decoding and ignores `Limits::max_string_length`, although lamina-syntax's `Lexer` is a dependency and lamina-content already wraps it tolerantly. lamina-build's PNG decoder re-implements the row filters lamina-syntax has for Flate predictors. This todo reuses both.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina-text/src/extract/cmap.rs:285-455`, `lamina-syntax/src/lexer.rs`, `lamina-content/src/lexer.rs:7` (the tolerant wrapper to copy), `lamina-build/src/surface/png.rs:353,376`, `lamina-syntax/src/filters.rs:180,223`.

## Scope

**In scope**

- CMap tokens from a tolerant wrapper over `lamina_syntax::Lexer` (on an error, skip one byte), with a test for a `#xx` name in a CMap.
- `pub fn unfilter_row(filter, raw, prev, out, bpp)` in lamina-syntax, used by `png.rs` (inflating stays separate: exact size versus budget).
- A ban in `check maintainability`: no lexer outside lamina-syntax.

**Out of scope** (and which todo covers it)

- Splitting `png.rs` ([Q2r](./Q2r-split-surface-modules.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write the `#xx` test and watch it fail.
- [ ] Replace the CMap lexer with the wrapper; run the CMap tests and the extraction facts.
- [ ] Export `unfilter_row`; switch `png.rs`; run the image reference PDFs.
- [ ] Add the ban.

## Acceptance criteria

- [ ] The extraction facts and image reference PDFs are unchanged, or any change is explained.
- [ ] About 150 fewer lines in lamina-text; one row-filter implementation.

## Post-merge checks

None.

## Tests

Existing CMap and PNG tests; the new `#xx` test.

## Docs to update

Plan §9.

## Risks and open questions

Stricter limits could reject malformed CMaps that worked before: keep skipping and warning as now.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
