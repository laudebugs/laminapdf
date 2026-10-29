# U4b — Reader mode (reflow) from the structure tree

| Field | Value |
| --- | --- |
| ID | `U4b` |
| Status | `ready` |
| Plan | [UI/UX §6 U4. Trust and reflow](../../plans/ui-ux.md#u4-trust-and-reflow) |
| Round / milestone | Round U (UI/UX) — after M2 |
| Depends on | [B8a](./B8a-structure-tree.md), [C5a](./C5a-text-layer-and-selection.md), [U1c](./U1c-inverted-l-shell-and-modes.md), [U2b](./U2b-reading-order-ground-truth.md) |
| Blocks | [U4c](./U4c-approximate-reader-mode.md) |
| ADRs | Follows 0024 (reader mode surface), 0005 (typed views); may propose a `lamina-reflow` crate per [code organization](../../guidelines/code-organization.md) |
| Branch | `feat/u4b-reader-mode-tagged` |
| PR title | `feat(reflow): add reader mode for tagged pdfs` |
| Size | L (500–900 lines incl. tests) |
| PR | — |

## Goal

Give phone readers and people who need larger text a one-tap reader mode that turns tagged PDFs into adjustable, accessible text — on the device, with no page cap — derived from the structure tree, with tables, figures and formulas kept as page crops so nothing is silently mangled.

## Context

- [UI/UX plan §5.8](../../plans/ui-ux.md#58-reader-mode-reflow-and-local-intelligence-r15-r16); [research: reflow](../../research/product/pdf-app-product-and-ux.md#reflow-organiser-dark-mode-and-perceived-speed) — Acrobat Reflow can't fix bad reading order (Fondazione LIA 2024); Liquid Mode needs Adobe's cloud and skips files over 200 pages; half of web PDFs tagged since mid-2025, fewer than 1% PDF/UA.
- ISO 32000-2 §14.7–§14.8 (logical structure, tagged PDF) and the PDF Association's "Deriving HTML from PDF" (v1.0, 2019) — read before writing code (spec first).
- [B8a](./B8a-structure-tree.md) — the structure-tree reader; [C5a](./C5a-text-layer-and-selection.md) — text runs and positions; [U2b](./U2b-reading-order-ground-truth.md) — scoring.
- [PDF research ch. 06](../../research/pdf/06-feature-inventory.md) — where reflow sits in the feature inventory.

## Scope

**In scope**

- Core (bytes-in/bytes-out): `reflow::from_structure(doc) -> Vec<Block>` where `Block` is heading (level), paragraph, list (items), quote, caption, figure (page + bbox + `/Alt`), table (page + bbox, plus cells when the table is well-formed), formula (page + bbox + `/Alt`), note, and link spans; each block carries its source page and bbox for "back to page".
- Sanity checks that need no ground truth, so they run on any user's file: the tree is empty or mostly `Span`/`NonStruct`; marked content covers too little of the page's text; structure order disagrees strongly with the order lamina's extraction infers from layout (a rank-correlation score). Thresholds are calibrated on U2b's tagged fixtures (good and deliberately broken), not computed from U2b's ground truth at runtime. A failing file falls back to U4c when it exists, else shows "Reader mode isn't available for this document".
- UI: a Reader toggle (command, bottom bar on phones, top bar on desktop), a reading view with font size, line spacing, measure and page mode (Original/Sepia/Dark via CSS), position sync both ways, and figure/table crops rendered through `RenderBackend` clip rects.
- Accessibility: semantic HTML (headings, lists, `figure`/`figcaption`, `table` when cells are known), language from `/Lang`.

**Out of scope** (and which todo covers it)

- Untagged files → [U4c](./U4c-approximate-reader-mode.md).
- Exporting reflowed text as HTML/EPUB → a later E3i extraction follow-up.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Read the spec sections and the PDF Association document; write the role-mapping table (standard structure types, role map) in the module docs.
- [ ] Implement `from_structure` with unit tests over generated tagged fixtures (from B8 builders) and real tagged files.
- [ ] Implement the sanity checks with tests on deliberately broken trees; calibrate thresholds on U2b's corpus and record them.
- [ ] Bind through `lamina-wasm` and the worker RPC; build the reading view in `@lamina/ui`.
- [ ] Position sync: map scroll position to block and block to page bbox; Playwright test for round-trips.
- [ ] Score reader-mode block order against U2b's tagged fixtures.

## Acceptance criteria

- [ ] On U2b's tagged fixtures, reader-mode order meets the U2b threshold.
- [ ] Toggling reader mode and back returns to the same page region (within one block) in Playwright.
- [ ] Tables, figures and formulas appear as crops with their `/Alt` text; none is reflowed as body text (test).
- [ ] axe zero serious/critical violations in reader view; text scales to 200% without horizontal scroll (WCAG 1.4.10).
- [ ] No document bytes leave the device (U4a indicator stays at 0 for local files).

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

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
