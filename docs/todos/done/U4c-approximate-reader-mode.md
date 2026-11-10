# U4c — Approximate reader mode for untagged PDFs

| Field | Value |
| --- | --- |
| ID | `U4c` |
| Status | `done` |
| Plan | [UI/UX §6 U4. Trust and reflow](../../plans/ui-ux.md#u4-trust-and-reflow) |
| Round / milestone | Round U (UI/UX) — after M2 |
| Depends on | [U4b](./U4b-reader-mode-from-structure-tree.md) |
| ADRs | Follows 0024; records any model or heuristic choice in the PR (no ML model without a licence check of its weights) |
| Branch | `feat/u4c-approximate-reader-mode` |
| PR title | `feat(reflow): add the approximate reader mode for untagged pdfs` |
| Size | L (500–900 lines incl. tests) |
| PR | [#376](https://github.com/laudebugs/laminapdf/pull/376) |

## Goal

Extend reader mode to the other half of PDFs, which carry no tags, with a pure-Rust layout heuristic over lamina's extraction, clearly labelled "Approximate" so users know when structure was guessed.

## Context

- **Design (Paper):** [R14 Reader mode](../../design/paper/reader-b.md#r14-reader-mode). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.8](../../plans/ui-ux.md#58-reader-mode-reflow-and-local-intelligence-r15-r16); [differentiation notes](../../research/product/notes/differentiation-mvp.md) — layout models and their licences (DocLayout-YOLO AGPL-3.0; Marker weights OpenRAIL-M with commercial limits; Docling code MIT, weights unchecked).
- [U4b](../done/U4b-reader-mode-from-structure-tree.md) — `Block` model and the reading view; [U2b](../done/U2b-reading-order-ground-truth.md) — scoring.
- [B9d](../done/B9d-text-runs-quads-and-page-text.md) — text runs with fonts, sizes and positions.

## Scope

**In scope**

- Core: `reflow::approximate(doc) -> Vec<Block>`: column detection (whitespace rivers / XY-cut), line and paragraph grouping, heading detection from font size and weight, header/footer removal by repetition across pages, caption and footnote heuristics, figure and table regions as crops (from image bboxes and ruling lines).
- A confidence score per page; pages below a threshold show as page images within the reader view rather than wrong text.
- UI: the "Approximate" label with an explanation, and a link to "View original page" on every block.

**Out of scope** (and which todo covers it)

- ML layout models → a later todo after a weights-licence audit (none in the core).
- OCR for scanned pages → [E13c](../ready/E13c-ocr-text-layer.md) / [E13d](../ready/E13d-local-ocr.md); scanned pages show as images until then.

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Implement the heuristics step by step, each with a unit test on generated fixtures.
- [x] Score against U2b's untagged fixtures (order and block roles); record per-file results.
- [x] Implement per-page confidence and the image fallback.
- [x] UI label, explanation and per-block "view original".

## Acceptance criteria

- [x] On U2b's untagged fixtures the approximate mode reaches a threshold set in this PR from the first run (ratchet) and never scores below U2a's copy-as-paragraph order on the same files.
- [x] Low-confidence pages fall back to images (test with a deliberately hard fixture).
- [x] The label and explanation are visible and announced; axe zero serious/critical violations.

## Post-merge checks

None.

## Tests

Rust unit and corpus tests; Playwright flows reuse U4b's.

## Docs to update

Module docs; tick the approximate-mode item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## As built

- **The core** (`laminapdf-extract`, `src/reflow/approximate.rs`): tier 2 runs whenever tier 1 would have refused (no tags, or a tree that fails U4b's checks), so `Reflow` gains `source` (`tags` or `approximate`) and `reason` now says why the tree wasn't used. Only a file with nothing to show is unavailable, as `nothing`. The blocks are extraction's in `ReadingOrder::Layout`. The heuristics the scope lists already exist there: XY-Cut++ columns, lines and paragraphs, headings by size and weight, running heads and feet by repetition (L3a, L3d), the column-order doubts (T3b) and ruled tables (L3b). Tier 2 adds the rest per page. Tables become crops that keep their text. Images at least half an inch on a side, and under 85% of the page, become figures (overlapping images join into one), placed before the first text below them in their column. A short paragraph starting "Figure", "Fig." or "Table" beside a crop becomes its caption.
- **Confidence and the image fallback**: a page's confidence is its text blocks' confidence weighted by characters. Layout blocks score 0.75 and T3b's doubted ones 0.5, so the threshold of 0.65 keeps pages with up to about two fifths of their text doubted. A page under it becomes a `page-image` block (`uncertain`), with its text kept for searching. So does a scanned page (`scanned`: text-layer status `needs-ocr` or `ocr-layer`, or no text under an image covering the page). The hard fixtures (`narrow-gutter`, `wrapped-grid`, `pull-quote`) fall back this way (`tests/approximate.rs`).
- **Scoring**: `crates/laminapdf/tests/reading_order.rs` scores the approximate view over U2b's 28 files with U2b's metric, its blocks' text standing for the lines, against `approximate.mean` in `fixtures/reading-order/threshold.json`. That ratchet starts at 0.9256. The test fails on any file where the view trails copy as paragraph (extraction's blocks, running heads and feet kept), and none does. Every file scores 1.000 but the three hard layouts, which keep extraction's order inside their page images: `narrow-gutter` 0.545, `pull-quote` 0.714, `wrapped-grid` 0.700.
- **The view** (`@laminapdf/ui`'s `ReaderView`; `apps/web/src/reader-mode.tsx`): the badge turns to a warning "Approximate" and a `role="status"` notice says why, in R14's words, announced on entry. Each block has "View original page", named "View original page (page n)", over its top corner on hover or focus and always shown on touch. It leaves reader mode at the block and outlines the block's box on the page for two seconds (`reader-origin.ts`, a viewer page layer). A page-image block shows the page, one sentence of reason and "Open page n". The status bar says "Reader mode · approximate". The gallery has an "approximate" state; axe passes in its three themes and both directions.
- **Fixes on the way**: a crop of a whole page asked the renderer for one pixel past the page (the clip rounded up, the page's pixels down), so page images drew nothing. `reader-crop.ts` now keeps a crop's clip inside the page.
- **Size**: the extract module is 724,812 B gzipped against its 725,000 B budget, 188 B of headroom. The next feature in that module will need a budget amendment.
- **Deviations**:
  - *No new layout heuristics.* Extraction already had them, so tier 2 is rules over its output; no model runs (ADR 0030 §6). Footnotes aren't detected without a model or tags.
  - *Figures come from images only.* A figure drawn with paths isn't cropped, so it doesn't appear in the column.
  - *Broken trees read approximately.* R14's *Not available* state now only covers a file with nothing to show; the notice names the tags as the reason.
  - *No "How Reader mode works" link*: the help page isn't built. Find in the column isn't built either, so a page image's text isn't searchable yet.
  - *The gallery screenshots were regenerated here with Playwright*, not in the pinned Docker image, which this environment lacks; only the six `reader-mode--*` files changed.

## Risks and open questions

- Heuristic quality plateaus: keep the interface model-agnostic so a licence-clean model can plug in later.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
