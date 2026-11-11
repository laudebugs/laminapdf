# U4b — Reader mode (reflow) from the structure tree

| Field | Value |
| --- | --- |
| ID | `U4b` |
| Status | `done` |
| Plan | [UI/UX §6 U4. Trust and reflow](../../plans/ui-ux.md#u4-trust-and-reflow) |
| Round / milestone | Round U (UI/UX) — after M2 |
| Depends on | [B8a](./B8a-structure-tree.md), [C5a](./C5a-text-layer-and-selection.md), [U1c](./U1c-inverted-l-shell-and-modes.md), [U2b](./U2b-reading-order-ground-truth.md) |
| ADRs | Follows 0024 (reader mode surface), 0005 (typed views); may propose a `lamina-reflow` crate per [code organization](../../guidelines/code-organization.md) |
| Branch | `feat/u4b-reader-mode-tagged` |
| PR title | `feat(reflow): add reader mode for tagged pdfs` |
| Size | L (500–900 lines incl. tests) |
| PR | [#362](https://github.com/laudebugs/laminapdf/pull/362) |

## Goal

Give phone readers and people who need larger text a one-tap reader mode that turns tagged PDFs into adjustable, accessible text — on the device, with no page cap — derived from the structure tree, with tables, figures and formulas kept as page crops so nothing is silently mangled.

## Context

- **Design (Paper):** [R14 Reader mode](../../design/paper/reader-b.md#r14-reader-mode). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.8](../../plans/ui-ux.md#58-reader-mode-reflow-and-local-intelligence-r15-r16); [research: reflow](../../research/product/pdf-app-product-and-ux.md#reflow-organiser-dark-mode-and-perceived-speed) — Acrobat Reflow can't fix bad reading order (Fondazione LIA 2024); Liquid Mode needs Adobe's cloud and skips files over 200 pages; half of web PDFs tagged since mid-2025, fewer than 1% PDF/UA.
- ISO 32000-2 §14.7–§14.8 (logical structure, tagged PDF) and the PDF Association's "Deriving HTML from PDF" (v1.0, 2019) — read before writing code (spec first).
- [B8a](../done/B8a-structure-tree.md) — the structure-tree reader; [C5a](../done/C5a-text-layer-and-selection.md) — text runs and positions; [U2b](../done/U2b-reading-order-ground-truth.md) — scoring.
- [PDF research ch. 06](../../research/pdf/06-feature-inventory.md) — where reflow sits in the feature inventory.

## Scope

**In scope**

- Core (bytes-in/bytes-out): `reflow::from_structure(doc) -> Vec<Block>` where `Block` is heading (level), paragraph, list (items), quote, caption, figure (page + bbox + `/Alt`), table (page + bbox, plus cells when the table is well-formed), formula (page + bbox + `/Alt`), note, and link spans; each block carries its source page and bbox for "back to page".
- Sanity checks that need no ground truth, so they run on any user's file: the tree is empty or mostly `Span`/`NonStruct`; marked content covers too little of the page's text; structure order disagrees strongly with the order lamina's extraction infers from layout (a rank-correlation score). Thresholds are calibrated on U2b's tagged fixtures (good and deliberately broken), not computed from U2b's ground truth at runtime. A failing file falls back to U4c when it exists, else shows "Reader mode isn't available for this document".
- UI: a Reader toggle (command, bottom bar on phones, top bar on desktop), a reading view with font size, line spacing, measure and page mode (Original/Sepia/Dark via CSS), position sync both ways, and figure/table crops rendered through `RenderBackend` clip rects.
- Accessibility: semantic HTML (headings, lists, `figure`/`figcaption`, `table` when cells are known), language from `/Lang`.

**Out of scope** (and which todo covers it)

- Untagged files → [U4c](../done/U4c-approximate-reader-mode.md).
- Exporting reflowed text as HTML/EPUB → a later E3i extraction follow-up.

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Read the spec sections and the PDF Association document; write the role-mapping table (standard structure types, role map) in the module docs.
- [x] Implement `from_structure` with unit tests over generated tagged fixtures (from B8 builders) and real tagged files.
- [x] Implement the sanity checks with tests on deliberately broken trees; calibrate thresholds on U2b's corpus and record them.
- [x] Bind through `laminapdf-wasm` and the worker RPC; build the reading view in `@laminapdf/ui`.
- [x] Position sync: map scroll position to block and block to page bbox; Playwright test for round-trips.
- [x] Score reader-mode block order against U2b's tagged fixtures.

## Acceptance criteria

- [x] On U2b's tagged fixtures, reader-mode order meets the U2b threshold.
- [x] Toggling reader mode and back returns to the same page region (within one block) in Playwright.
- [x] Tables, figures and formulas appear as crops with their `/Alt` text; none is reflowed as body text (test).
- [x] axe zero serious/critical violations in reader view; text scales to 200% without horizontal scroll (WCAG 1.4.10).
- [x] No document bytes leave the device (U4a indicator stays at 0 for local files).

## As built

- **The core** (`laminapdf-extract`, `src/reflow/`; the façade's `Document::reflow`): `reflow(doc, cancel)` → `Reflow { available, reason, lang, blocks, checks }`, built on extraction's tagged path (`ReadingOrder::Auto`), whose blocks already follow the tree on each page that passes ADR 0030 §2's checks. On top of it, `tree.rs` walks the tree once for each element's place in its order, the `Figure`, `Formula` and `Table` elements with their `/Alt` and marked content, and the share of content in no block-level element; `order.rs` scores the tree's order against XY-Cut++'s (Kendall's τ per page, weighted by blocks); `mod.rs` makes text blocks inside a crop element into one crop (a caption stays text), boxed by what its marked content paints (`Marks` keeps a box per MCID) and the text it takes in, marks paragraphs in a `BlockQuote` as quotes, and puts crops in their elements' place. The module docs hold the role-mapping table.
- **The checks and their thresholds**, calibrated on the new corpus: refused when the file isn't tagged (`untagged`), the tree holds no content (`empty`), more than half its content sits in no block-level element (`generic`), the per-page checks ignore it on more than a quarter of its tagged pages (`coverage` or `order`, by the commoner), or τ is below 0.5 (`order`). Every correct tree scores τ 1 but `pull-quote`'s, 0.52 (a layout XY-Cut++ misreads); the shuffled trees score 0.43, 0.2 and 0.2, the reversed ones −1. The margin is thin.
- **The corpus** (`fixtures/reflow/`, `laminapdf-build`'s `tests/common/reflow.rs` through the `reflow` example; `cargo xtask golden` writes and checks it with the reading-order corpus): every reading-order layout tagged in reading order and drawn in reverse; three layouts with trees reversed, shuffled, all `Span` and half untagged; and `mixed.pdf` with a figure and a formula with `/Alt`, a block quote, a list and a table. `tests/reflow.rs` requires every tagged file's exact order and kinds (score 1, above U2b's threshold), every broken one refused for its reason, the figure's crop within half a point of its image, and nothing of a crop in the text.
- **Bindings**: `WasmDocument.reflow()` (the extract module only: the text module, for builders, stays under its size budget), `@laminapdf/sdk`'s `doc.reflow()` with `Reflow` types (`extract-module-not-loaded` elsewhere), served by the worker client; `@laminapdf/viewer`'s `reflow` worker method and `client.reflow()`.
- **UI**: `@laminapdf/ui`'s `ReaderView` (the column, crops as `role="img"` named by their alternate text, the sticky head with Back and the badge, skeleton lines, the empty state) and `ReaderSettingsPanel`; the top bar's Reader toggle and Reader settings button; seven `view.reader-*` commands, the toggle on `R`. `apps/web/src/reader-mode.tsx` reads the reflow once per document through the engine, starts at the block at the view's top (the viewer's `pointAt`, a share of the page's height, in points), returns to the column's top block with `goToDestination`, draws crops with the document worker, and keeps the settings in platform storage.
- **Tests**: `e2e/reader-mode.e2e.test.ts` (Chromium and Firefox here): the mixed file's order and crops, the round trip (page 2 of the tagged report golden, the "Notes" heading at the column's top, then at the view's top within 80 px), the untagged refusal, Escape, 32 px text in a 640 px window without sideways scrolling, and axe; the gallery's `reader-mode` route (axe in three engines and three themes, screenshots).
- **Deviations**:
  - *The tagged fixtures are new.* U2b's files aren't tagged, so reader mode's order is scored on tagged copies of its layouts, required to be exact.
  - *No Appearance chips in the reader settings.* The page appearance is [U2c](./U2c-page-appearance-modes.md)'s View setting; the column takes its paper and ink, and while reader mode is on the Reader settings button takes the View button's place, so it is changed before entering or from the palette.
  - *Phone*: no bottom-bar Reader button or settings sheet; reader mode is in the palette, and the column works at phone width.
  - *Not built*: link spans in the text, "View original page" per block, Find in the column, the "How Reader mode works" help page, a table's cells as an HTML table, and a crop of an element over two pages (it shows its first page's part).
  - *The PDF Association's "Deriving HTML from PDF"* couldn't be reached from here; the role mapping follows ISO 32000-2 §14.8.4 and the repository's research notes.
  - *Reader mode is in the extract module only.* With the text module loaded too, documents open in the text module and `doc.reflow()` throws `extract-module-not-loaded`, saying why; putting it in the text module costs 8 KB over that module's budget, which is the owner's call.
  - *A tree is only refused as a whole.* A page the per-page checks ignore still shows in the layout's order when the rest of the tree passes.

## Post-merge checks

None.

## Tests

Rust unit and corpus tests for `from_structure`; Playwright reader-mode flows on desktop and mobile profiles.

## Docs to update

Module docs; [UI/UX §9](../../plans/ui-ux.md#9-to-dos) tick; feature row in the capability matrix (X2) if it exists by then.

## Risks and open questions

- Bad tags are common (fewer than 1% claim PDF/UA): the sanity checks and the U4c fallback carry the quality.
- Artifacts (headers, footers, page numbers) tagged as content: drop `Artifact` content and repeated page furniture.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
