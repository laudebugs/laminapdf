# U2c — Page appearance modes: Original, Sepia, Dark and High contrast

| Field | Value |
| --- | --- |
| ID | `U2c` |
| Status | `ready` |
| Plan | [UI/UX §6 U2. Reading quality](../../plans/ui-ux.md#u2-reading-quality) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [C7a](./C7a-accessibility-and-theming.md) |
| Blocks | [C7c](./C7c-performance-gates-and-launch.md) |
| ADRs | Implements 0024 (page appearance); extends 0004 (`RenderBackend` options) |
| Branch | `feat/u2c-page-appearance-modes` |
| PR title | `feat(render): add page appearance modes recoloured in the renderer` |
| Size | M (300–600 lines incl. tests) |
| PR | — |

## Goal

Let people read comfortably at night and in sepia without the broken photo negatives a CSS invert produces: recolour text and vector paths in the renderer, leave raster images untouched, keep annotation colours, and follow the OS high-contrast colours, separately from the UI theme.

## Context

- [UI/UX plan §5.5](../../plans/ui-ux.md#55-page-appearance-and-themes-r8) and ADR 0024.
- [Research: dark mode](../../research/product/pdf-app-product-and-ux.md#reflow-organiser-dark-mode-and-perceived-speed) — pdf.js `pageColors` with `Canvas`/`CanvasText`; PDFium's forced-colours path recolours text and paths only; PDF Expert's Night and Sepia themes.
- [C2b](./C2b-lamina-render-crate.md) — `RenderBackend` and its PDFium and hayro adapters; [C7a](./C7a-accessibility-and-theming.md) — UI themes and `forced-colors` handling.

## Scope

**In scope**

- `RenderOptions::appearance`: `Original | Sepia | Dark | HighContrast { fg, bg }` in `lamina-render`, implemented in the PDFium adapter (its colour-scheme rendering) and the hayro adapter (path/text colour mapping), with images untouched.
- Annotation appearance streams drawn with original colours in every mode (render annotations in a separate pass or flag them).
- UI: a page-mode menu and palette commands (U1b), per-document override, global default, stored locally; `forced-colors: active` selects High contrast automatically unless the user overrides.
- A `--page-backdrop` token for the gutter in each UI theme.

**Out of scope** (and which todo covers it)

- UI themes (light, dark, high contrast chrome) → [C7a](./C7a-accessibility-and-theming.md).
- Recolouring in reader mode (it uses CSS text colours) → [U4b](./U4b-reader-mode-from-structure-tree.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add the option to the trait with a default of `Original` (no behaviour change for existing callers).
- [ ] Implement in both adapters; golden renders per mode for five fixtures (text-only, vector drawing, photo page, mixed, annotated).
- [ ] Add the nightly render diff of modes between PDFium and hayro (X3b lane).
- [ ] UI menu, commands, persistence (try/catch around storage) and `forced-colors` detection.
- [ ] Check that C7a shipped no page filter of its own (its scope leaves page appearance to this todo).

## Acceptance criteria

- [ ] In Sepia and Dark, opaque images without an `/SMask` and with the Normal blend mode are pixel-identical to Original over their regions (golden diff); for images with transparency, soft masks or other blend modes, a renderer-level test checks their decoded samples bypass recolouring (their composited pixels may differ because the paper behind them changed).
- [ ] Annotation colours are unchanged in every mode (golden diff over annotation regions).
- [ ] Dark text/background contrast ≥ 7:1 for body text on the fixtures; High contrast uses the system colours under emulated `forced-colors`.
- [ ] Switching mode re-renders visible tiles only and keeps the scroll position (Playwright).

## Post-merge checks

None.

## Tests

Golden renders in `crates/lamina-render/tests/` per adapter and mode; Playwright mode switching; the nightly PDFium-vs-hayro diff includes modes.

## Docs to update

`crates/lamina-render` README; ADR 0004 amendment note if the trait changes; tick the page-mode item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Text drawn as images or Type3 bitmaps won't recolour: document it and show a hint in Dark mode on such pages.
- PDFium's colour-scheme API may not be exposed in the chosen WASM build: check during C1b; fall back to a post-process that skips image regions using the page's image bboxes.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
