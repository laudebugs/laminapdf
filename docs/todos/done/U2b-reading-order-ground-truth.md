# U2b — Reading-order ground-truth corpus and metric

| Field | Value |
| --- | --- |
| ID | `U2b` |
| Status | `done` |
| Plan | [UI/UX §6 U2. Reading quality](../../plans/ui-ux.md#u2-reading-quality) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [B9d](./B9d-text-runs-quads-and-page-text.md) |
| Blocks | [B9e](./B9e-separators-at-backward-jumps.md), [L3a](../ready/L3a-lamina-extract-reading-order.md), [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md), [U4b](../ready/U4b-reader-mode-from-structure-tree.md) |
| ADRs | Follows 0018 (test strategy: corpora, differential tests) and 0019 (fixture provenance) |
| Branch | `test/u2b-reading-order-ground-truth` |
| PR title | `test(text): add a reading-order ground-truth corpus and metric` |
| Size | M (fixtures and expected files don't count; ~300 lines of harness) |
| PR | [#79](https://github.com/laudebugs/lamina/pull/79) |

## Goal

No public benchmark measures reading order, yet column-aware copy, reflow and screen-reader order all depend on it. Build a small, licence-clean corpus with hand-checked reading order and a metric that CI tracks, so copy-as-paragraph (U2a) and reader mode (U4b, U4c) are held to a number instead of anecdotes.

## Context

- [Research: text extraction](../../research/product/pdf-app-product-and-ux.md#five-conclusions-from-the-comparison) — py-pdf's benchmark doesn't explain its metric; pdf_oxide's reports pass rates, not correctness.
- [Engines notes](../../research/product/notes/engines-sdks.md) — the benchmarks and their limits.
- [B9d](../done/B9d-text-runs-quads-and-page-text.md) — the extraction API whose order this measures.
- [ADR 0018](../../decisions/0018-test-strategy.md), [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) — corpus rules and provenance.

## Scope

**In scope**

- `fixtures/reading-order/`: 30–50 files covering one and two columns, three-column newspapers, academic papers with footnotes and figures, sidebars, tables, rotated text, RTL (Arabic, Hebrew) and CJK vertical text, headers and footers; tagged and untagged variants where possible.
- Expected order per file as JSON: ordered blocks (paragraph text, role: heading/paragraph/caption/footnote/header/footer/table) checked by hand, with the method recorded.
- A metric: normalised edit distance over block order plus a character-level score inside blocks (document the formula), reported per file and as a corpus mean.
- `cargo xtask` or a test in `laminapdf-text` that prints the score table and fails below a threshold set from the first run (ratchet: the threshold can only go up).
- Provenance in `fixtures/README.md` (generated files preferred; third-party files only with a permissive licence).

*As built:* 22 generated files rather than 30–50 mixed ones; tagged variants wait for the structure tree writer (B8a), vertical CJK for vertical layout in the builder. The first run found that cells drawn right to left on one baseline come out joined without a space (`table-reversed`): an extraction fix for a B9 follow-up, measured here, not made. Every block's JSON also carries its box (page space), for geometry-aware scorers (U4c).

**Out of scope** (and which todo covers it)

- Improving the extraction algorithm itself → B9 follow-ups (this PR measures).
- Reflow-specific scoring of block roles → [U4c](../ready/U4c-approximate-reader-mode.md) reuses the corpus.

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Generate synthetic multi-column fixtures with `laminapdf-build` (exact ground truth from the generator): 11 layouts (one, two and three columns, an academic page with caption and footnote, a sidebar, a table, running headers and footers over two pages, a `/Rotate 90` page, Arabic, Hebrew in two columns read right first, horizontal Chinese), each drawn in reading order and in reverse: 22 files (`crates/laminapdf-build/tests/common/reading_order.rs`, written by its `reading_order` example through `cargo xtask golden --update`).
- [ ] Pick real-world files with permissive licences (arXiv CC-BY papers, government publications) and hand-annotate order; two-person check for each. *Not done here:* the two-person check needs a second annotator; the generated corpus has exact ground truth meanwhile. Left for a follow-up with the owner (the risks below already prefer generated files).
- [x] Write the metric with unit tests on toy inputs (identity → 1.0, reversed → known value).
- [x] Wire the run into the full lane (X4) and the score into the PR summary: the corpus test is an ordinary test (`crates/laminapdf/tests/reading_order.rs`), so it runs in the fast lane on every PR (stricter than the full lane alone) and appends its table to `$GITHUB_STEP_SUMMARY`.
- [x] Record the first scores and set the ratchet threshold (mean 0.787, `fixtures/reading-order/threshold.json` with its history; per-file scores in `fixtures/README.md`).

## Acceptance criteria

- [x] Every fixture has provenance and a licence in `fixtures/README.md`; cargo-deny/licence checks unaffected.
- [x] The metric's unit tests pass; the corpus run prints per-file scores and fails below the threshold (and above it by more than 0.005: the ratchet is raised with the change that raises the score).
- [x] The threshold and first scores are recorded in the plan or the fixture README.

## Post-merge checks

None.

## Tests

Unit tests for the metric; a corpus test over `fixtures/reading-order/` in the full lane.

## Docs to update

`fixtures/README.md`; tick the corpus item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Hand annotation is slow and subjective: prefer generated fixtures, and record disagreements rather than hiding them.
- Publishing the corpus may be worth a short post (the research found none); decide with the owner.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
