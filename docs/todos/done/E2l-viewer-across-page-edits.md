# E2l — The viewer and edit session across edits that change the page list

| Field | Value |
| --- | --- |
| ID | `E2l` |
| Status | `done` |
| Plan | [E2 page organisation §5](../../plans/editor/e2-page-organisation.md#5-the-viewer-across-page-structure-edits) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1d](./E1d-edit-session-undo-ui.md), [E1g](./E1g-redraw-after-edit.md), [E2b](./E2b-page-structure-ops.md) |
| ADRs | Follows [0009](../../decisions/0009-web-client-architecture.md), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) |
| Branch | `feat/e2l-viewer-across-page-edits` |
| PR title | `feat(viewer): follow inserted, deleted and moved pages by ID` |
| Size | L (400–800): the viewer's state mapped by page ID instead of index, the worker's guard and the history steps, with browser and performance tests |
| PR | [laudebugs/laminapdf#604](https://github.com/laudebugs/laminapdf/pull/604) |

## Goal

Let the reader show edits that insert, delete or move pages. Today the document worker refuses an edit that changes the page count, because the view, its caches and the edit history find pages by index ([E1 plan §6](../../plans/editor/e1-editing-foundation.md#6-redrawing-after-an-edit)); its guard also misses a reorder, which keeps the count. This todo maps the viewer's state through page IDs, so every later E2 interface todo can apply page-structure ops.

## Context

- [E2 plan §5](../../plans/editor/e2-page-organisation.md#5-the-viewer-across-page-structure-edits)
- `packages/viewer/src/worker/edit.ts` (`samePages`, `touched`), `worker/session.ts` (steps record page indices), `src/edit/session.ts`, `src/view/viewer.ts` (`invalidate(pages, sizes)`), `src/thumbnails/strip.ts`, `src/history/history.ts`, `src/edit-log.ts`
- [ADR 0027 §6](../../decisions/0027-coordinates-units-and-page-indexing.md#6-page-indexing): indices in APIs, `PageId` where identity matters across edits

## Scope

**In scope**

- `EditResult` (and undo, redo, go-to-step results) carry the page IDs in their new order and each page's size whenever the edit changed the list, from the façade's `Edited::page_order`
- The viewer remaps by ID: layout, bitmap cache (moved pages keep their bitmaps), text, link and find caches, keep-top position, the thumbnail strip, the navigation history and the reading position; entries for a deleted page go to the nearest surviving page
- The worker's count guard is replaced by a check that the model's order after the edit matches what it reports
- The edit session's steps keep page IDs, not indices
- Three cases in the viewer's performance harness on the 1,000-page file: delete one page, move 200 pages, insert 10 blank pages; each held to the [E2 plan §5](../../plans/editor/e2-page-organisation.md#5-the-viewer-across-page-structure-edits) budget

**Out of scope** (and which todo covers it)

- Commands and the page grid → [E2h](../done/E2h-page-organiser-ui.md)
- Replaying source documents into a restarted worker → [E2m](../done/E2m-document-dialogs.md)

**Editor facts**

- **Operations shipped:** none (makes `C.01`, `C.03`, `C.04`, `C.13` showable)
- **Crates and packages:** `@laminapdf/viewer`, `@laminapdf/sdk` (`Edited.pageOrder`)
- **`Op` variants:** none
- **`SaveConstraint`:** not applicable
- **Web vs desktop:** the viewer is shared; the desktop engine's session follows in [E2n](../ready/E2n-desktop-organise.md)
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [x] `EditResult` with the new order and sizes; the SDK's worker client passes `Edited.pageOrder`
- [x] `viewer.reorder(order, sizes)` (or `invalidate` taking IDs) and the remapping of each cache
- [x] Thumbnail strip, navigation history and reading position by ID
- [x] Session steps by ID; undo and redo report the order
- [x] The three performance cases, reported in the harness

## Acceptance criteria

- [x] Browser tests in Chromium, Firefox and WebKit: after `DeletePages`, `MovePages`, `InsertBlankPages` and `ReversePages`, then undo and redo of each, every page shows its own content, size and text layer, find results land on the right pages, Back and Forward reach the right pages, and the history panel names the right pages
- [x] A reorder that keeps the count redraws correctly (the case the old guard missed)
- [x] The three performance cases are reported, and each meets the plan's budget or the report says by how much it misses
- [x] Driven in the built reader in a real browser, as a user would, before merge (CLAUDE.md working agreements)

## As built

- **The worker** (`src/worker/edit.ts`) keeps the page order it knows (`EditState.order`) and stops refusing page-list edits. An edit that reports a page order (`Edited.pageOrder`) must leave the pages in it, and one that reports none must keep the count, or the edit is taken back. The pages are read only when the list may have changed or pages are drawn again. Each result carries the new list (`EditResult.order`) when it changed. The worker remembers which steps changed the list (`EditState.reorders`), since undo and redo report no order. Edits that only move pages (`movePages`, `reversePages`) draw nothing again, and nor do their undo and redo.
- **The session** keeps each step's page IDs (`EditStep.pageIds`). `historyOf` names a step's pages where they are now (`placed`), and leaves out deleted ones.
- **The viewer** follows the client's `edit` event (`#reorder`, over `layout/page-moves.ts`'s `pageMoves`):
  - its bitmaps are rekeyed to their pages' new indices (`BitmapCache.remap`, `viewKeyAt`);
  - the edit revisions, the pinned and pending places, `find.remap` and `history.remap` move with their pages, and history entries that became the same place merge;
  - the page at the top of the view stays there, and the current page moves first, so a fit zoom fits it;
  - a deleted page's state goes to its nearest surviving page;
  - the `reorder` event carries the `PageMoves`, and the thumbnail strip follows it (after a worker restart, with fresh pictures).
- **Tests:** `test/browser/page-list.browser.test.ts` runs delete, move, reverse and insert with undo and redo (each page's colour, size and text, find, Back and Forward, the history's page names). It also checks that a moved page in view keeps its pictures, and that an edit through the client moves the view. Unit tests cover `pageMoves`, `historyOf`'s names and the history's merge.
- **Measured:** in Chromium 153 on a 4-core 2.1 GHz Xeon, from page 500 of the 1,000-page file:
  - delete one page: 74 ms to the first new pixels;
  - move 200 pages: 69 ms;
  - insert 10 blank pages: 137 ms.

  All three are inside the 150 ms budget (E2 plan §5). On the same machine E1's quarter turn took 196 ms against 20 ms on an M1 Pro, so that case's miss there is the machine's.
- **Startup JavaScript:** the viewer's part adds about 1.1 KB gzipped to the reader's app shell. The owner raised the waiver to 280,700 on 2026-11-15.
- **In the reader:** no reader command changes the page list yet (E2h adds them), so the page-list edits were driven in Chromium through the viewer, in the browser tests and the performance harness's page.

## Post-merge checks

None.

## Tests

- `packages/viewer/test/browser/` cases for each op, undo and redo
- `packages/viewer/perf/` scenarios

## Docs to update

- The E2 plan §5 with the measured times
- [full-editor §E2](../../plans/full-editor.md#e2-page-organisation) and the [E2 plan §8](../../plans/editor/e2-page-organisation.md#8-todos): point this todo's link at `done/`
- `CLAUDE.md`'s `packages/viewer` entry

## Risks and open questions

- Bitmaps of moved pages: if a move changes how a page draws (it shouldn't), the per-page revision in the view key must still drop them

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
