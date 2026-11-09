# U2c — Page appearance modes: Original, Sepia, Dark and High contrast

| Field | Value |
| --- | --- |
| ID | `U2c` |
| Status | `done` |
| Plan | [UI/UX §6 U2. Reading quality](../../plans/ui-ux.md#u2-reading-quality) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C2b](./C2b-lamina-render-crate.md), [C7a](./C7a-accessibility-and-theming.md) |
| ADRs | Implements 0024 (page appearance); extends 0004 (`RenderBackend` options) |
| Branch | `feat/u2c-page-appearance-modes` |
| PR title | `feat(render): add page appearance modes recoloured in the renderer` |
| Size | M (300–600 lines incl. tests) |
| PR | [#375](https://github.com/laudebugs/laminapdf/pull/375) |

## Goal

Let people read comfortably at night and in sepia without the broken photo negatives a CSS invert produces: recolour text and vector paths in the renderer, leave raster images untouched, keep annotation colours, and follow the OS high-contrast colours, separately from the UI theme.

## Context

- **Design (Paper):** [R10 View settings](../../design/paper/reader-b.md#r10-view-settings). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.5](../../plans/ui-ux.md#55-page-appearance-and-themes-r8) and ADR 0024.
- [Research: dark mode](../../research/product/pdf-app-product-and-ux.md#reflow-organiser-dark-mode-and-perceived-speed) — pdf.js `pageColors` with `Canvas`/`CanvasText`; PDFium's forced-colours path recolours text and paths only; PDF Expert's Night and Sepia themes.
- [C2b](../done/C2b-lamina-render-crate.md) — `RenderBackend` and its PDFium and hayro adapters; [C7a](../done/C7a-accessibility-and-theming.md) — UI themes and `forced-colors` handling.

## Scope

**In scope**

- `RenderOptions::appearance`: `Original | Sepia | Dark | HighContrast { fg, bg }` in `laminapdf-render`, implemented in the PDFium adapter (its colour-scheme rendering) and the hayro adapter (path/text colour mapping), with images untouched.
- Annotation appearance streams drawn with original colours in every mode (render annotations in a separate pass or flag them).
- UI: a page-mode menu and palette commands (U1b), per-document override, global default, stored locally; `forced-colors: active` selects High contrast automatically unless the user overrides.
- A `--page-backdrop` token for the gutter in each UI theme.

**Out of scope** (and which todo covers it)

- UI themes (light, dark, high contrast chrome) → [C7a](../done/C7a-accessibility-and-theming.md).
- Recolouring in reader mode (it uses CSS text colours) → [U4b](../ready/U4b-reader-mode-from-structure-tree.md).

## Tasks

- [x] Claim via the work PR's first commit; a fresh worktree off the updated `main`.
- [x] Add the option to the trait with a default of `Original` (no behaviour change for existing callers).
- [x] Implement in both adapters; golden renders per mode for five fixtures (text-only, vector drawing, photo page, mixed, annotated).
- [ ] Add the nightly render diff of modes between PDFium and hayro (X3b lane). Left for later: see Notes.
- [x] UI menu, commands, persistence (try/catch around storage) and `forced-colors` detection.
- [x] Check that C7a shipped no page filter of its own (its scope leaves page appearance to this todo): none in the reader, the UI or the viewer.

## Acceptance criteria

- [x] In Sepia and Dark, opaque images without an `/SMask` and with the Normal blend mode are pixel-identical to Original over their regions (golden diff); for images with transparency, soft masks or other blend modes, a renderer-level test checks their decoded samples bypass recolouring (their composited pixels may differ because the paper behind them changed).
- [x] Annotation colours are unchanged in every mode (golden diff over annotation regions).
- [x] Dark text/background contrast ≥ 7:1 for body text on the fixtures; High contrast uses the system colours under emulated `forced-colors`.
- [x] Switching mode re-renders visible tiles only and keeps the scroll position (Playwright).

## Post-merge checks

None.

## Tests

Golden renders in `crates/laminapdf-render/tests/` per adapter and mode; Playwright mode switching; the nightly PDFium-vs-hayro diff includes modes.

## Docs to update

`crates/laminapdf-render` README; ADR 0004 amendment note if the trait changes; tick the page-mode item in [UI/UX §9](../../plans/ui-ux.md#9-to-dos).

## Risks and open questions

- Text drawn as images or Type3 bitmaps won't recolour: document it and show a hint in Dark mode on such pages.
- PDFium's colour-scheme API may not be exposed in the chosen WASM build: check during C1b; fall back to a post-process that skips image regions using the page's image bboxes.

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **Recoloured after drawing, not while painting.** PDFium's colour scheme forces every fill to one colour, which flattens charts and tinted boxes, and hayro's painting device is private. So each adapter draws the tile, then `recolour_tile` (`crates/laminapdf-render/src/appearance.rs`, ported to `@laminapdf/render`) recolours it pixel by pixel. This is the fallback the risks above name.
- **What stays as drawn:**
  - Opaque images keep every pixel. Each adapter finds them with its own parser, so the two-parsers rule holds.
  - See-through images (an `/SMask`, a `/Mask`, a blend mode or a soft mask) keep their samples, and the paper around them is recoloured. PDFium's `HasTransparency` only covers the drawing state, so PDFium images up to 1 MP are decoded to look for transparent samples. Larger ones are taken as opaque.
  - Annotations: what they change comes from a second drawing with them. One that changes at least half of its rectangle keeps the whole rectangle, so text under a highlight keeps its colour.
- **Colours:** Sepia maps each channel from ink to paper. Dark turns the lightness over and keeps the hue. High contrast squares the lightness, so greys move toward the ink. The token colours come from `PAGE_APPEARANCES`.
- **Tests:** the render goldens live in `laminapdf-render-pdfium`'s ignored tests (both backends), because `cargo xtask render-golden` runs them and writes the goldens. `crates/laminapdf-render/tests/` only checks the colour table. The web adapters run the same checks in Node and three browsers through `@laminapdf/test-utils`' render contract.
- **`--page-backdrop`:** not added. U6a's `gutter` token is already "the canvas behind pages" in each UI theme. The viewer's page shells take `--lamina-page-paper`, which the reader sets from the appearance's paper, so a page never flashes white in Dark.
- **The reader:** a document's choice wins over all documents'. A local file's choice is kept under a SHA-256 of its name and size, and a linked file's for the tab only (ADR 0011). With no choice made, `forced-colors: active` picks High contrast in the system's CanvasText and Canvas.
- **Left for later:**
  - The nightly hayro-vs-PDFium diff in each appearance. The pass is shared code that gives the same bytes in both adapters, and the render-golden job compares both adapters in every appearance on each PR. Adding it means an `--appearance` option in `render-diff` and in `cargo xtask render-diff`, history keys and reuse inputs per appearance, and a nightly step.
  - The notice for pages whose text is drawn as images.
