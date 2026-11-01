# Q2i — One PDF lexer and one PNG row filter

| Field | Value |
| --- | --- |
| ID | `Q2i` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2r](./Q2r-split-large-rust-modules-surface.md) |
| ADRs | None |
| Branch | `refactor/q2i-one-lexer` |
| PR title | `refactor(text,build): CMap parsing on lamina-syntax's lexer; PNG unfiltering from lamina-syntax` |
| Size | M |
| PR | — |

## Goal

`lamina-text/src/extract/cmap.rs` has its own ~150-line PDF lexer that skips `#xx` name decoding and ignores `Limits::max_string_length`, though lamina-syntax's `Lexer` is a dependency (lamina-content already wraps it tolerantly). lamina-build's PNG decoder re-implements the None/Sub/Up/Average/Paeth row filters lamina-syntax has for Flate predictors. Reuse both.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina-text/src/extract/cmap.rs:285-455`, `lamina-syntax/src/lexer.rs`, `lamina-content/src/lexer.rs:7` (the tolerant wrapper to copy), `lamina-build/src/surface/png.rs:353,376`, `lamina-syntax/src/filters.rs:180,223`.

## Scope

**In scope**

- CMap tokenising through a tolerant wrapper over `lamina_syntax::Lexer` (on error skip one byte); a test for a `#xx` name in a CMap.
- `pub fn unfilter_row(filter, raw, prev, out, bpp)` in lamina-syntax; png.rs uses it (inflate stays separate: exact size vs budget).
- The ban "no lexer outside lamina-syntax" in `check maintainability`.

**Out of scope** (and which todo covers it)

- Splitting png.rs ([Q2r](./Q2r-split-large-rust-modules-surface.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Replace, test with the CMap and PNG fixtures, run the extraction facts and image goldens.

## Acceptance criteria

- [ ] Extraction facts and image goldens unchanged (or improved, explained).
- [ ] About 150 lines fewer in lamina-text; one row-filter implementation.

## Post-merge checks

None.

## Tests

Existing cmap and PNG tests; new `#xx` test.

## Docs to update

Plan §9.

## Risks and open questions

Stricter limits in CMap parsing could reject malformed CMaps that worked: keep tolerance (skip, warn) as before.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
