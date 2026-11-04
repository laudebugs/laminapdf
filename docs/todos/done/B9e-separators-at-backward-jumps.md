# B9e — Text extraction: a separator where the pen jumps back on a line

| Field | Value |
| --- | --- |
| ID | `B9e` |
| Status | `done` |
| Plan | [pdf-library §7 B9](../../plans/pdf-library.md#b9-text-extraction) |
| Round / milestone | Round B (library) — M2 Web reader v0.1 |
| Depends on | [B9d](./B9d-text-runs-quads-and-page-text.md), [U2b](./U2b-reading-order-ground-truth.md) |
| Blocks | None |
| ADRs | Follows [0018](../../decisions/0018-test-strategy.md) (the reading-order ratchet measures it) |
| Branch | `fix/b9e-backward-jump-separators` |
| PR title | `fix(text): separate text where the pen jumps back on a line` |
| Size | S (≈ 50–150 lines) |
| PR | [#84](https://github.com/laudebugs/lamina/pull/84) |

## Goal

Text drawn out of order on one baseline comes out glued: [U2b](../done/U2b-reading-order-ground-truth.md)'s `table-reversed` file draws each table row right to left, and `PageText::lines` joins the cells of a row with no space between them (`goblet.Signal`), so their words are lost (text score 0.572). A left-to-right line whose next run starts behind where the previous one ended is a new cell, column or overprint, never the same word: put a space there.

## Context

- `crates/lamina-text/src/extract/runs.rs`: `join` adds a space only when the pen jumps forward over a quarter of the size; right-to-left lines are ordered by position in `bidi_line` and are not affected.
- [U2b](../done/U2b-reading-order-ground-truth.md)'s corpus and ratchet (`fixtures/reading-order/threshold.json`, mean 0.787 at the start).

## Scope

**In scope**

- In left-to-right lines (`join`), a space between two runs when the next starts behind the previous one's end by more than a quarter of the size, unless either side already has whitespace.
- Unit tests in `extract_runs.rs`; the reading-order corpus rescored and the threshold raised with its history.

**Out of scope**

- Reordering cells or columns by position: reading order is [L3a](../ongoing/L3a-lamina-extract-reading-order.md).

## Tasks

- [x] The separator in `join`, with tests
- [x] Rescore the corpus, raise `threshold.json` (history entry)

## Acceptance criteria

- [x] `table-reversed` keeps every cell's words (text score 1.000); no other corpus file's score drops (corpus mean 0.787 → 0.797, threshold raised with its history)
- [x] Every text golden still extracts its drawn lines (`extract_fonts.rs`)

## Post-merge checks

None.

## Tests

`crates/lamina-text/tests/extract_runs.rs`; `crates/lamina/tests/reading_order.rs` (ratchet).

## Docs to update

- [x] `fixtures/README.md` (the first scores' note on `table-reversed`), the `lamina-text` README (`lines()`)

## Risks and open questions

- A producer that draws a word in pieces backwards (rare) would get a space inside it; the quarter-size threshold keeps kerning and small corrections out.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started (here: added straight to `ongoing/` by its claim PR)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
