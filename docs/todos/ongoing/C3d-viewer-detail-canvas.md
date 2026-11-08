# C3d — `@laminapdf/viewer`: a sharp canvas over the part in view, so every zoom is sharp

| Field | Value |
| --- | --- |
| ID | `C3d` |
| Status | `ongoing` |
| Plan | [web reader §6 C3. Viewer core](../../plans/web-app.md#c3-viewer-core) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C3b](../done/C3b-viewer-virtualiser-and-zoom.md) |
| ADRs | Follows 0009 (framework-free viewer core, `ImageBitmap` transfer, pooled canvases) and 0004 (bounded tiles) |
| Branch | `feat/c3d-viewer-detail-canvas` |
| PR title | `feat(viewer): draw sharp tiles into a canvas over the part in view, so every zoom stays sharp` |
| Size | M (300–500 changed lines incl. tests) |
| PR | [laudebugs/lamina#317](https://github.com/laudebugs/laminapdf/pull/317) |

## Goal

Keep the page in view sharp at every zoom up to 800 %. Today each page is one canvas the size of the whole page, and every canvas shares the pool's 64 MP cap. A page's pixels grow with the square of the zoom: a Letter page on a 2× screen is 14 MP at 200 %, 55 MP at 400 % and 220 MP at 800 %. So above about 300 % the pool lowers every page's resolution (the visible page gets about 1.92× at 400 %), and Safari's limit of about 16.7 MP per canvas is reached near 230 %. With a backdrop for the whole page, capped in pixels, and a canvas over the part in view at the full pixel ratio, the cost of the sharp pixels depends on the viewport, not the zoom.

## Context

- [`docs/bugs/viewer-blurry-at-zoom/resolution.md`](../../bugs/viewer-blurry-at-zoom/resolution.md): the 1-px bug's fix, which left this as later work; `pagePixels` from that fix is the page's size in pixels.
- [C3b](../done/C3b-viewer-virtualiser-and-zoom.md): the virtualiser, the canvas pool, the low pass and the sharp tiles this todo builds on.
- [UI/UX plan §5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7): the progressive render pipeline (low pass, then sharp tiles).
- pdf.js solves the same problem with a "detail canvas" over a capped page canvas.

## Scope

**In scope**

- Each mounted page keeps one backdrop canvas, at the sharp scale when the page fits a per-page pixel cap, else at the scale that fits it.
- When the backdrop is below the sharp scale (the cap, or the pool's fit), a page in view gets a detail canvas: the tile-aligned part of the page in view, at the full pixel ratio, placed over the backdrop. The sharp tiles are drawn into it; the backdrop gets its own tiles over the whole page.
- Tiles in view take the horizontal scroll into account, not only the vertical one.
- Detail canvases come from their own pool, so the pixels across both pools stay at today's 64 MP.

**Out of scope**

- Retrying a tile that failed (a separate change; see the bug's resolution).
- Zoom above 800 %.

## Tasks

- [ ] `src/render/tiles.ts`: the backdrop's pixel ratio under the per-page cap; the union of a tile list (the detail canvas's rectangle).
- [ ] `src/view/viewer.ts`: the frame reads `scrollLeft`; tiles in view are clipped horizontally; a page view holds an optional detail canvas (rectangle, scale, drawn tiles), placed in percentages of the shell so it stretches with it during a zoom gesture; it is re-placed when the tiles in view leave its rectangle, redrawing the cached tiles at once; released when the page leaves the view, unmounts or changes geometry.
- [ ] `CreateViewerOptions.detailPool`; the default pools split today's 64 MP.
- [ ] Render stats count the detail canvases.

## Acceptance criteria

- [ ] At 400 % and 800 % on a 2× screen, the page in view is drawn at the full pixel ratio (the detail canvas's pixels per CSS px equal the device pixel ratio), and the page reaches `sharp`.
- [ ] Up to the zoom at which a page fits the cap, nothing changes: no detail canvas, sharp tiles in the page canvas.
- [ ] Scrolling at 400 % keeps the view sharp once it stops, with no draw on an unmounted page.
- [ ] The sweep test of the tile planning covers the detail rectangle: every tile lies inside it and passes `planTile`.

## Post-merge checks

None.

## Tests

- Node (`src/render/tiles.test.ts`): the backdrop ratio under the cap; the detail rectangle covers the tiles in view and stays inside the page.
- Browser (`test/browser/viewer.browser.test.ts`, Chromium, Firefox, WebKit): 400 % and 800 % reach sharp with a detail canvas at the full ratio; scrolling at 400 % sharpens again; 100 % has no detail canvas.

## Docs to update

- `packages/viewer/README.md`, CLAUDE.md's viewer line, the web reader plan's C3 todos line, and the bug's resolution (left for later → done in C3d).

## Risks and open questions

- Resizing a canvas clears it: on re-placing the detail canvas, the cached tiles must be drawn in the same frame, or the view flickers to the backdrop.
- The backdrop's own tiles cost worker time after the tiles in view; they run at the pass's priority, after the detail tiles.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
