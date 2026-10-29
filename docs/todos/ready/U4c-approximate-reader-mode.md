# U4c — Approximate reader mode for untagged PDFs

| Field | Value |
| --- | --- |
| ID | `U4c` |
| Status | `ready` |
| Plan | [UI/UX §6 U4. Trust and reflow](../../plans/ui-ux.md#u4-trust-and-reflow) |
| Round / milestone | Round U (UI/UX) — after M2 |
| Depends on | [U4b](./U4b-reader-mode-from-structure-tree.md) |
| Blocks | None |
| ADRs | Follows 0024; records any model or heuristic choice in the PR (no ML model without a licence check of its weights) |
| Branch | `feat/u4c-approximate-reader-mode` |
| PR title | `feat(reflow): add the approximate reader mode for untagged pdfs` |
| Size | L (500–900 lines incl. tests) |
| PR | — |

## Goal

Extend reader mode to the other half of PDFs, which carry no tags, with a pure-Rust layout heuristic over lamina's extraction, clearly labelled "Approximate" so users know when structure was guessed.

## Context

- [UI/UX plan §5.8](../../plans/ui-ux.md#58-reader-mode-reflow-and-local-intelligence-r15-r16); [differentiation notes](../../research/product/notes/differentiation-mvp.md) — layout models and their licences (DocLayout-YOLO AGPL-3.0; Marker weights OpenRAIL-M with commercial limits; Docling code MIT, weights unchecked).
- [U4b](./U4b-reader-mode-from-structure-tree.md) — `Block` model and the reading view; [U2b](./U2b-reading-order-ground-truth.md) — scoring.
- [B9b](./B9b-text-extraction-core.md) — text runs with fonts, sizes and positions.

## Scope

**In scope**

- Core: `reflow::approximate(doc) -> Vec<Block>`: column detection (whitespace rivers / XY-cut), line and paragraph grouping, heading detection from font size and weight, header/footer removal by repetition across pages, caption and footnote heuristics, figure and table regions as crops (from image bboxes and ruling lines).
- A confidence score per page; pages below a threshold show as page images within the reader view rather than wrong text.
- UI: the "Approximate" label with an explanation, and a link to "View original page" on every block.

**Out of scope** (and which todo covers it)

- ML layout models → a later todo after a weights-licence audit (none in the core).
- OCR for scanned pages → [E13c](./E13c-ocr-text-layer.md) / [E13d](./E13d-local-ocr.md); scanned pages show as images until then.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Implement the heuristics step by step, each with a unit test on generated fixtures.
- [ ] Score against U2b's untagged fixtures (order and block roles); record per-file results.
- [ ] Implement per-page confidence and the image fallback.
- [ ] UI label, explanation and per-block "view original".

## Acceptance criteria

- [ ] On U2b's untagged fixtures the approximate mode reaches a threshold set in this PR from the first run (ratchet) and never scores below U2a's copy-as-paragraph order on the same files.
- [ ] Low-confidence pages fall back to images (test with a deliberately hard fixture).
- [ ] The label and explanation are visible and announced; axe zero serious/critical violations.

## Post-merge checks

None.

## Tests

Rust unit and corpus tests; Playwright flows reuse U4b's.

## Docs to update

Module docs; tick the approximate-mode item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Heuristic quality plateaus: keep the interface model-agnostic so a licence-clean model can plug in later.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
