# E1g — Redraw after an edit

| Field | Value |
| --- | --- |
| ID | `E1g` |
| Status | `done` |
| Plan | [E1 editing foundation §6](../../plans/editor/e1-editing-foundation.md#6-redrawing-after-an-edit) |
| Round / milestone | Editor phase 2 (E1–E3) — M5 Organiser |
| Depends on | [E1a](./E1a-editing-foundation-feature-plan.md) |
| ADRs | Follows ADR 0004 (the two-parsers rule), 0009 |
| Branch | `feat/e1g-redraw-after-edit` |
| PR title | `feat(viewer): redraw the pages an edit touches` |
| Size | M (200–400) |
| PR | [#399](https://github.com/laudebugs/laminapdf/pull/399) |

## Goal

PDFium draws the reader's pages from the bytes it opened, so an edit to Lamina PDF's model changes nothing on screen. This todo carries edits to the screen. After a transaction that touches pages, the document worker writes an incremental update of the model to memory and reopens PDFium's document from the new bytes. The viewer then redraws only the touched pages, with their new sizes. The todo measures what that costs, which the architecture plan names as a risk.

## Context

- [E1 plan §6](../../plans/editor/e1-editing-foundation.md#6-redrawing-after-an-edit): the approach and the proposed budget
- [architecture §6](../../plans/architecture.md#6-risks): the two-parsers risk
- `packages/viewer/src/worker/document.ts`: the `Open` record holds the Lamina PDF model and a separate `RenderDocument`
- `packages/viewer/src/canvas/cache.ts` (`BitmapCache.delete`, `prune`), `src/layout/` (`PageLayout`, fixed page sizes today), layers' `refresh(pages?)`

## Scope

**In scope**

- A worker method that applies ops to the model, then (when pages were touched) saves incrementally into memory and swaps the `RenderDocument` for one opened from those bytes, keeping the old one until the new one opens.
- A viewer API to redraw given pages and to take new page sizes: drop their bitmaps, re-measure the layout, keep the reader's position.
- Measurements in the viewer's performance harness (`cargo xtask viewer-perf`): time from apply to the first redrawn tile at 10, 100 and 1,000 pages.

**Out of scope**

- The edit session, its history and the interface: [E1d](../ongoing/E1d-edit-session-undo-ui.md).
- Rendering from Lamina PDF's own model: decided after these measurements (E2).

## Tasks

- [x] Worker: `applyEdit(ops)` (named after the other worker methods) over the model's `apply`, an in-memory incremental save, and a reopened `RenderDocument`; errors leave the old render document in place.
- [x] Viewer: `invalidate(pages, sizes?)`, which drops the pages' bitmaps, updates `PageLayout` for changed sizes and refreshes the layers on those pages.
- [x] The perf scenario and its report line.

## Acceptance criteria

- [x] Rotating a page through the worker redraws that page turned, with its new size, in Chromium, Firefox and WebKit; other pages' bitmaps are kept.
- [x] A metadata-only edit reopens nothing and redraws nothing.
- [x] The time to the first redrawn tile is measured and reported for 10, 100 and 1,000 pages, and the plan's budget is confirmed or changed from it.

## Post-merge checks

None.

## Tests

Browser tests in `packages/viewer/test/browser/` (rotate, redraw, sizes, kept bitmaps); the perf scenario.

## Docs to update

`packages/viewer` README; the E1 plan's §6 with the measured figures; CLAUDE.md's viewer line.

## Risks and open questions

- Reopening PDFium on every edit may be slow on large files; the measurement decides whether E2 reloads differently.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
