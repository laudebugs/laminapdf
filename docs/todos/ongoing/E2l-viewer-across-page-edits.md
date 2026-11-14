# E2l — The viewer and edit session across edits that change the page list

| Field | Value |
| --- | --- |
| ID | `E2l` |
| Status | `ongoing` |
| Plan | [E2 page organisation §5](../../plans/editor/e2-page-organisation.md#5-the-viewer-across-page-structure-edits) ([full-editor §E2](../../plans/full-editor.md#e2-page-organisation)) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1d](../done/E1d-edit-session-undo-ui.md), [E1g](../done/E1g-redraw-after-edit.md), [E2b](../done/E2b-page-structure-ops.md) |
| ADRs | Follows [0009](../../decisions/0009-web-client-architecture.md), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) |
| Branch | `feat/e2l-viewer-across-page-edits` |
| PR title | `feat(viewer): follow inserted, deleted and moved pages by ID` |
| Size | L (400–800): the viewer's state mapped by page ID instead of index, the worker's guard and the history steps, with browser and performance tests |
| PR | — |

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

- Commands and the page grid → [E2h](../ready/E2h-page-organiser-ui.md)
- Replaying source documents into a restarted worker → [E2m](../ready/E2m-document-dialogs.md)

**Editor facts**

- **Operations shipped:** none (makes `C.01`, `C.03`, `C.04`, `C.13` showable)
- **Crates and packages:** `@laminapdf/viewer`, `@laminapdf/sdk` (`Edited.pageOrder`)
- **`Op` variants:** none
- **`SaveConstraint`:** not applicable
- **Web vs desktop:** the viewer is shared; the desktop engine's session follows in [E2n](../ready/E2n-desktop-organise.md)
- **Server features:** none
- **Capability-matrix rows to flip:** none

## Tasks

- [ ] `EditResult` with the new order and sizes; the SDK's worker client passes `Edited.pageOrder`
- [ ] `viewer.reorder(order, sizes)` (or `invalidate` taking IDs) and the remapping of each cache
- [ ] Thumbnail strip, navigation history and reading position by ID
- [ ] Session steps by ID; undo and redo report the order
- [ ] The three performance cases, reported in the harness

## Acceptance criteria

- [ ] Browser tests in Chromium, Firefox and WebKit: after `DeletePages`, `MovePages`, `InsertBlankPages` and `ReversePages`, then undo and redo of each, every page shows its own content, size and text layer, find results land on the right pages, Back and Forward reach the right pages, and the history panel names the right pages
- [ ] A reorder that keeps the count redraws correctly (the case the old guard missed)
- [ ] The three performance cases are reported, and each meets the plan's budget or the report says by how much it misses
- [ ] Driven in the built reader in a real browser, as a user would, before merge (CLAUDE.md working agreements)

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

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
