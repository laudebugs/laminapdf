# D9h — Viewer: two-page and continuous page layouts

| Field | Value |
| --- | --- |
| ID | `D9h` |
| Status | `ready` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | [ADR 0009](../../decisions/0009-web-client-architecture.md) and the viewer's own record if the viewer's model changes: add an amendment (task below) |
| Branch | `feat/d9h-viewer-page-layouts` |
| PR title | `feat(viewer): single, two-page and continuous page layouts` |
| Size | L (400–800: the layout, the scroll and visible-range logic, the tile planning and the settings change in a package both apps use; split the layout class from the viewer wiring if you can) |
| PR | — |

## Goal

Give the reader, on the web and in the desktop, the three page layouts its View menu names: Single (one page at a time), Two pages (side by side, as a book opens) and Continuous (the pages in one scrolling column). Neither app has this today. The command `view.page-layout` exists in `packages/ui/src/commands/reader-commands.ts` with the choices `single`, `two-up` and `continuous`, but `apps/web/src` has no handler for it, so it works nowhere. It is a viewer feature first and a desktop menu item second.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment listed `view.page-layout` with the others; reading the web reader shows it is not parity, which is why this todo exists apart from [D9a](../ongoing/D9a-desktop-theme-and-view-settings.md).
- What exists: `packages/viewer/src/layout/page-layout.ts` builds "one continuous column of pages" with a gap, each sized from its displayed size times the zoom and turned by the view rotation. Offsets are prefix sums (`tops`), so the page at a scroll position is a binary search. `page-layout.test.ts` and `page-moves.ts` rest on that one column.
- `packages/viewer/src/view/viewer.ts` (about 2,250 lines) reads `PageLayout` for scroll (`scrollTop`, `visibleRange`, `offsetOf`), the page that is current, zoom-to-fit and the tiles it asks for (`packages/viewer/src/render/tiles.ts`). Every one of these assumes a single column.
- So Continuous is today's behaviour and needs only a name and a persisted choice. Single and Two pages are new:
  - **Single:** one page fills the view; scrolling past the page's end or a key moves to the next. The visible range is one page; the page to prefetch is the next.
  - **Two pages:** pages pair as (1) then (2, 3), (4, 5) and so on, or (1, 2), (3, 4) by an option.
  - **Rows:** two pages of different sizes share a row, so the layout needs a row model, not a column of boxes.
- The desktop asks the engine for tiles by page, scale and clip (`apps/desktop/src/engine/desktop-engine-client.ts`, `planTile`), and does not depend on where the viewer puts a page. So the engine needs no change as long as the viewer plans tiles per page; confirm that on the 1,000-page fixture.
- The tile and canvas budgets (`packages/viewer/src/view/budgets.ts`, `canvas/pool.ts`) assume one column's visible range; a row of two pages doubles the canvases in view.
- Whether the three layouts share one `PageLayout` with a row model, or three strategies behind one interface, is the main design choice. The PR decides, with the cost per zoom change as `page-layout.test.ts` counts it.

## Scope

**In scope**

- The layouts in `@laminapdf/viewer`: a layout choice option on the viewer, the row model, the scroll and visible-range logic, current-page tracking and keyboard paging for each.
- Zoom to fit width and to fit page in each layout, and the tiles planned for the pages in view.
- The `view.page-layout` handler in the web reader and in the desktop, with the choice kept (the web's localStorage; the desktop's `preferences.json`, as [D9a](../ongoing/D9a-desktop-theme-and-view-settings.md) keeps its choices), and the native menu items enabled and ticked once this lands.
- An ADR amendment task if the viewer's model changes (below).

**Out of scope**

- Facing pages with a cover page option, and right-to-left binding, unless they come free; say in the PR.
- Page layout while editing or in Organise mode.
- Touch gestures beyond what the viewer has.

## Tasks

- [ ] Fresh worktree; claim in the work PR.
- [ ] Read `page-layout.ts`, `viewer.ts` and `render/tiles.ts`; write the design (row model or strategies) in the PR before the code, with the cost per operation.
- [ ] Implement the layouts behind the viewer's option, with the single-column behaviour unchanged and tested.
- [ ] Wire the handler in the web (`apps/web/src/settings.tsx` or beside the page appearance) and in the desktop; make the menu tick.
- [ ] Measure scroll, zoom and tile counts on the 1,000-page fixture in each layout against the budgets.
- [ ] ADR note: if the viewer's page model changes from a column to rows, add an amendment to the ADR that records the viewer's layout (ADR 0009 or the one that owns `PageLayout`; find it) saying what the layouts are and how the visible range is defined.
- [ ] Tests, browser check, docs.

## Acceptance criteria

- [ ] In the web reader and the real desktop app on macOS, Windows and Linux, View ▸ Page layout offers Single, Two pages and Continuous, ticks the current one, and changes what the window shows; the choice survives a restart.
- [ ] Continuous is pixel-identical to today's layout (the existing viewer tests pass without change to their expectations).
- [ ] Single shows one page, moves to the next with the keyboard and the wheel at the page's end, and keeps the current page across a zoom change.
- [ ] Two pages shows pairs side by side, with pages of different sizes centred in their row, and an odd last page alone.
- [ ] The visible range, the current page and the page-number box are right in each layout, including after a rotation.
- [ ] The 1,000-page fixture scrolls inside the existing budgets in every layout; the numbers are recorded.
- [ ] A test in the style of `use-native-menus.test.ts` shows `view.page-layout` available with the desktop's actions.
- [ ] A browser check from the user's side with real PDFs (mixed page sizes, landscape pages, a long one): switch layouts with mouse and keyboard, zoom, rotate, and judge that nothing jumps. Record what was seen.

## Post-merge checks

- [ ] None.

## Tests

- Unit tests for each layout's geometry and searches, counted by operations as `page-layout.test.ts` does; property tests that the page at a scroll offset is the page whose box holds it.
- Viewer browser tests for scroll, current page and zoom in each layout.
- Web e2e and a desktop e2e scenario for the menu choice.

## Docs to update

- `docs/plans/desktop-app.md` and, if the web plan lists layouts, `docs/plans/web-app.md`: tick the to-do.
- The ADR amendment above.
- `docs/capabilities/README.md`: no row for page layout; no change.
- The reader's user docs if they list view options.

## Risks and open questions

- **A change to the package both apps use.** The web reader's behaviour in Continuous must not move; the old tests are the guard.
- **Size.** If it passes 800 lines, split: the layout class first, with no UI, then the viewer wiring and the menu.
- **Budgets.** Two pages in view double the canvases; the pool size or the backdrop ratio may need a rule per layout.
- **Prefetch.** Single mode must prefetch the neighbours, or paging shows blank pages.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
