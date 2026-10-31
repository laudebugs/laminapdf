# C3b — `@lamina/viewer` part 2: virtualiser, canvas pool, progressive tiles and anchored zoom

| Field | Value |
| --- | --- |
| ID | `C3b` |
| Status | `ready` |
| Plan | [web reader §6 C3. Viewer core](../../plans/web-app.md#c3-viewer-core) |
| Round / milestone | Round C (web reader) — M2 Web reader v0.1 |
| Depends on | [C3a](../ongoing/C3a-viewer-worker-rpc.md) |
| Blocks | [C3c](./C3c-viewer-perf-harness.md), [C4a](./C4a-ui-chrome-and-platform.md), [D1a](./D1a-desktop-ipc-spike.md), [D1b](./D1b-desktop-print-spike.md), [D2d](./D2d-tile-transport-and-first-document.md) |
| ADRs | Implements 0009 (framework-free viewer core, `ImageBitmap` transfer, pooled canvases) |
| Branch | `feat/c3b-viewer-virtualiser` |
| PR title | `feat(viewer): add the page virtualiser, canvas pool, progressive tiles and anchored zoom` |
| Size | L (600–1,500 lines incl. tests) |
| PR | — |

## Goal

Make the viewer scroll a 1,000-page document like a native app. A prefix-sum page layout mounts only visible pages ± a margin, canvases come from a fixed pool, each page paints a low-resolution pass first and then sharp tiles, and zoom (fit width, fit page, actual size, pinch, ctrl+wheel, keyboard) keeps the point under the cursor fixed. Everything is framework-free TypeScript with an imperative API and a subscribe-able view state, so `@lamina/ui` (React) and the desktop app can drive it without being on the hot path ([architecture principle 5](../../plans/architecture.md#principles-each-becomes-part-of-an-adr)).

## Context

- **Design (Paper):** [R02 Loading states](../../design/paper/reader-a.md#r02-loading-states). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.6](../../plans/ui-ux.md#56-perceived-performance-r6-r7) — the four-step pipeline (true-aspect placeholders → stretched low-res bitmap → 256–512 px sharp tiles, cancelled on scroll/zoom → CSS transform only during pinch) and the mobile caps (tile ≤ 5 MP, total bitmap budget) that [U3a](./U3a-responsive-mobile-and-touch.md) tunes.
- [web-frontend notes §2](../../research/architecture/notes/web-frontend.md) — prefix-sum virtualiser, zoom anchoring needs custom logic, pooled canvases, low-res then tiles, cap canvas area (Safari/iOS canvas memory limits).
- [rendering notes §4](../../research/architecture/notes/rendering.md) — visible ± N pages, placeholder sizes from page boxes, LRU bitmap cache, thumbnails on a lower-priority queue.
- [web-app.md §4 functional 2–3](../../plans/web-app.md#functional) and [non-functional targets](../../plans/web-app.md#non-functional-targets-are-recommendations-measured-from-c3-onward) (no main-thread task > 50 ms; sharp within 300 ms of stopping; ≤ 600 MB).
- [C3a](../ongoing/C3a-viewer-worker-rpc.md) — RPC, priorities, cancellation guarantees this builds on.

## Scope

**In scope**

- `src/layout/` — `PageLayout` (page sizes in PDF units × zoom × rotation → heights/widths, prefix sums, gaps; binary search `pageAt(y)`; `offsetOf(page)`), spread modes off for v0.1 (single continuous column).
- `src/view/` — `Viewer` class: `mount(container)`, `setDocument(rpcDoc)`, scroll container with a sized spacer, page shells (absolutely positioned `div`s) for visible ± margin pages, `IntersectionObserver`/scroll handler with rAF batching, view-state store (`currentPage`, `zoom`, `zoomMode`, `rotation`, `scrollTop`, `visibleRange`) with `subscribe()` for `useSyncExternalStore`.
- `src/canvas/` — `CanvasPool` (fixed max count, acquire/release, reset on reuse, total device-pixel area cap), `BitmapCache` (LRU keyed by page/scale/tile, byte-size accounting, `close()` on eviction).
- `src/render/` — progressive pipeline: low-res whole page (e.g. ≤ 0.5× DPR or ≤ 256 px wide) at `visible` priority, then sharp tiles (256–512 px, CSS px × `devicePixelRatio`) for the visible region; prefetch next/previous pages at `prefetch` priority; cancel everything for pages that leave the margin.
- `src/zoom/` — zoom modes (fit width, fit page, actual size = 100 % at 96/72 CSS px per pt, custom 10 %–800 %), ctrl/cmd+wheel and trackpad pinch (`wheel` with `ctrlKey`), touch pinch (pointer events), keyboard (`+`, `-`, `0`), all preserving the anchor point; view rotation (0/90/180/270, view-only).
- `examples/viewer-demo.html` — a minimal demo page (file input + viewer) used by C3c and manual testing.

**Out of scope** (and which todo covers it)

- Perf harness and CI budgets → [C3c](./C3c-viewer-perf-harness.md), [C7c](./C7c-performance-gates-and-launch.md).
- Toolbar/UI → [C4a](./C4a-ui-chrome-and-platform.md).
- Text, link and highlight layers → [C5a](./C5a-text-layer-and-selection.md), [C5b](./C5b-find-in-document.md), [C6b](./C6b-links-and-history.md) (this PR provides the per-page layer slots they mount into).
- Thumbnails → [C6a](./C6a-thumbnails-outline-page-labels.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] `PageLayout` with unit tests: prefix sums for 10,000 variable-size pages computed in < 5 ms equivalent work (asserted by operation count, not wall clock), `pageAt` by binary search, recompute on zoom/rotation in O(n) once.
- [ ] Page shells: one `div.lamina-page` per mounted page with `data-page-index`, fixed size from layout, child slots in z-order: `canvas` (raster), `div.lamina-layer-text`, `div.lamina-layer-annot`, `div.lamina-layer-highlight` (empty for now), with a `PageLayerHost` API to register layers (`onMount/onUnmount/onZoom`) so later todos plug in.
- [ ] Virtualisation: mount visible range ± margin (default 2 pages or 1 viewport, whichever is larger); unmount releases canvases and cancels in-flight requests via `AbortController` per page.
- [ ] `CanvasPool`: max N canvases (default 12), hard cap on total canvas pixels (default 64 Mpx, lower on iOS detected by capability, not UA sniffing where possible); reuse resets size and context state.
- [ ] Progressive render: low-res pass → `drawImage` scaled; then tiles for the visible part of the page at full DPR; tile bitmaps drawn into the page canvas when all are ready or progressively; stale tiles (old zoom) kept until the new ones arrive (no white flash) and then closed.
- [ ] `BitmapCache` with byte accounting (`width × height × 4`), default budget 256 MB, evicting farthest-from-viewport first; `close()` every evicted bitmap.
- [ ] Zoom anchoring: compute the document point under the anchor (cursor or viewport centre) before zoom, re-layout, then set `scrollTop/Left` so the point stays put; debounce sharp re-render until zoom gestures settle (e.g. 150 ms) and show CSS-scaled bitmaps meanwhile.
- [ ] Scroll performance: all DOM writes in a single rAF; no layout reads after writes in the same frame; passive listeners; `content-visibility`/`contain: strict` on page shells.
- [ ] Public API (named exports): `createViewer(options)`, `Viewer#open(doc)`, `goToPage(index, { anchor })`, `setZoom(value | mode, { anchor })`, `rotate(delta)`, `subscribe(listener)`, `getState()`, `destroy()`; events for `pagechange`, `zoomchange`, `renderstats` (for the harness).
- [ ] Demo page and README section.

## Acceptance criteria

- [ ] In Playwright (Chromium, Firefox, WebKit), scrolling the demo from page 1 to 1,000 of a generated 1,000-page fixture never mounts more than `visible + 2 × margin` page shells and never holds more than the pool's max canvases (asserted through `renderstats`).
- [ ] **Cancelled renders never paint**: a test scrolls fast past 200 pages; no `drawImage` occurs for a page that is unmounted at the time the result arrives (instrumented counter = 0).
- [ ] Zoom anchoring: after ctrl+wheel zoom 100 % → 200 % → 50 % at a fixed cursor position, the document point under the cursor moves by ≤ 1 CSS px (asserted from layout maths and screenshots).
- [ ] Fit width / fit page / actual size produce the expected scale for portrait, landscape and rotated pages (unit tests).
- [ ] After scrolling stops, every visible page reaches sharp tiles (state `sharp` in `renderstats`) — the timing budget itself is enforced by [C3c](./C3c-viewer-perf-harness.md).
- [ ] No React or other framework import in `packages/viewer` (lint rule / test scanning imports).

## Post-merge checks

None.

## Tests

Unit tests (layout, zoom maths, cache eviction, pool caps) in `packages/viewer/src/**/*.test.ts`; Vitest browser-mode integration tests for mount/unmount and cancellation; Playwright specs under `packages/viewer/e2e/` against the demo page in three engines. The 1,000-page fixture is generated at test setup with `@lamina/pdf`'s builder (not committed), seeded and deterministic.

## Docs to update

`packages/viewer/README.md` (layout, layers, public API, tuning defaults); tick "Implement the prefix-sum virtualiser and canvas pool", "Implement low-res then tiled sharp rendering…" and "Implement zoom modes and anchor-preserving zoom" in [web-app.md §9](../../plans/web-app.md#9-to-dos).

## Risks and open questions

- **Safari canvas memory limits** are undocumented; keep caps configurable and measure in C3c on WebKit.
- **Very tall documents** can exceed browsers' maximum element height (~33.5 M px in some engines) at high zoom; detect and switch to a scaled scroll space (scroll position mapped) if needed — test with 1,000 pages at 400 %.
- **Pinch on Safari** uses `gesturestart/gesturechange` rather than ctrl+wheel; handle both.
- Default margin, pool size and cache budget are recommendations; C3c tunes them with data.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
