# U2a — Selection surfaces and copy modes (copy as paragraph, copy exact, column selection)

| Field | Value |
| --- | --- |
| ID | `U2a` |
| Status | `done` |
| Plan | [UI/UX §6 U2. Reading quality](../../plans/ui-ux.md#u2-reading-quality) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C5a](./C5a-text-layer-and-selection.md), [U1b](./U1b-command-registry-and-palette.md), [U2b](./U2b-reading-order-ground-truth.md), [U3a](./U3a-responsive-mobile-and-touch.md) |
| ADRs | Implements 0024 (selection surfaces); follows 0009 |
| Branch | `feat/u2a-selection-surfaces` |
| PR title | `feat(ui): add the selection toolbar, the touch action bar and copy modes` |
| Size | M (300–600 lines incl. tests) |
| PR | [#363](https://github.com/laudebugs/laminapdf/pull/363) |

## Goal

Fix the friction users report most after speed: copying from multi-column PDFs grabs the wrong order and pastes hard line breaks. Add a desktop selection toolbar and a touch bottom action bar (never covering the OS selection menu), make "copy as paragraph" the default, and add "copy exact" and rectangular column selection.

## Context

- **Design (Paper):** [F01](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards), [F04](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [UI/UX plan §5.3](../../plans/ui-ux.md#53-selection-and-copy-r4-r5) and ADR 0024.
- [Research: selection menus](../../research/product/pdf-app-product-and-ux.md#selection-menus-float-them-on-desktop-never-over-the-os-menu-on-mobile) — Android ActionMode, iOS UIEditMenuInteraction, Wikimedia T202723.
- [Competitive teardown: friction](../../research/product/notes/competitive-teardown.md) — multi-column copy, hard line breaks, Okular's "copy without line breaks", Preview's Option-drag.
- [C5a](../done/C5a-text-layer-and-selection.md) — the text layer and the selection events with rects and runs.
- [U2b](../done/U2b-reading-order-ground-truth.md) — the reading-order metric copy-as-paragraph is held to.

## Scope

**In scope**

- `SelectionToolbar` (fine pointer): above the first selected line, flips below on collision, appears on `pointerup`, `role="toolbar"`, Shift+F10 to focus, Escape to dismiss; actions Copy, Copy exact, Search for, and editor actions when their modes exist; can be turned off in settings.
- `SelectionActionBar` (coarse pointer): replaces U3a's phone bottom bar while a selection exists (Copy, Share, Search for, More); never positioned over the native menu.
- Copy modes in the clipboard path: paragraph (reading order from lamina's extraction, joined lines, de-hyphenated, paragraph breaks kept) and exact (layout line breaks); both also write `text/html` with paragraphs.
- Rectangular selection with Alt/Option-drag (desktop) that selects within a column box.
- Commands registered in U1b: `copy`, `copy-exact`, `select-column-mode`.

**Out of scope** (and which todo covers it)

- Highlight, underline, comment actions → [E4f](../ready/E4f-annotation-tools-ui.md) adds them to these surfaces.
- Reading-order corpus and metric → [U2b](../done/U2b-reading-order-ground-truth.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Subscribe to C5a's selection events; compute placement with a small collision routine (no positioning library unless its licence is checked).
- [x] Implement both copy modes on top of the extraction runs; unit-test joining, hyphen repair (soft hyphen, hyphen at line end followed by lowercase), and ligature expansion.
- [x] Implement rectangular selection and its hit-testing against text runs.
- [x] Hide the toolbar on scroll start and while the user drags; reshow on `pointerup`.
- [x] Touch: detect `pointer: coarse`; swap the bottom bar; verify on iOS Safari and Android Chrome that the native menu still appears and is not covered.

## Acceptance criteria

- [x] On U2b's multi-column fixtures, copy-as-paragraph meets the U2b threshold; copy exact reproduces line breaks.
- [x] Pasting a copied paragraph into a plain textarea gives no line breaks inside paragraphs (test).
- [x] The toolbar never covers selected text (geometry test over 20 selection positions, including page edges).
- [x] Playwright mobile profiles: the action bar appears on a touch selection and never overlaps the selected text's rects (emulation doesn't render the OS selection menu)
- [ ] On a real iPhone (Safari) and Android phone (Chrome), the native menu stays visible and uncovered — a recorded manual check in the PR (the owner's: this work had no device)
- [x] Keyboard: Shift+F10 focuses the toolbar; every toolbar action is also in the context menu and palette.

## Post-merge checks

None.

## Tests

Unit tests for the copy pipeline in `packages/ui` (or `@laminapdf/viewer` if the logic lives there); Vitest browser tests for the toolbar; Playwright desktop and mobile profiles.

## Docs to update

Tick the selection and copy items in [UI/UX §9](../../plans/ui-ux.md#9-to-dos); a note in `apps/web/ACCESSIBILITY.md` once it exists.

## Risks and open questions

- Paragraph joining is wrong for verse, code and tables: fall back to exact when the extraction marks a block as preformatted or tabular.
- Browser clipboard permissions differ: use the async clipboard API in the user gesture, with the `copy` event as fallback.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

- **Copy modes** (`packages/viewer/src/layers/copy.ts`): copy as paragraph puts each selected piece in the reading-order block its span's centre lies in (or the nearest), orders blocks as the extraction does, joins a block's lines with a space (a soft hyphen, or a hyphen before a lowercase letter, joins the word again), expands the Latin ligatures, and leaves a blank line between blocks; copy exact keeps the layout's lines. A table block keeps its line breaks in both modes; the extraction marks no other block as preformatted, so verse and code are joined like paragraphs. Both modes write `text/plain` and `text/html` (a `<p>` per block, `<br>` between kept lines).
- **Reading order in the viewer:** a worker method `readingOrder(page)` returns the page's blocks (`ReadingBlock`: box and type) from `extract({ format: "json", pages: [page], drop: { furniture: false } })`, cached like the page text. The text layer asks for a page's blocks when a selection reaches it, so a copy right after a first selection joins that page in run order.
- **Copy from a command:** `Viewer.copySelection(mode)` runs `document.execCommand("copy")` in the gesture, so the copy event writes both formats in the mode asked for; where that fails, the async clipboard (`ClipboardItem` with both types, else `writeText`). The web app's Permissions-Policy blocks reading the clipboard, so the e2e test reads what the copy event wrote.
- **Selection events:** `TextSelection` gained `settled` (false while the pointer that makes it is down, sent again on `pointerup`) and `column`; the viewer also emits `selectionchange`. `joinPieces` went: its tests now test copy exact.
- **Column selection:** an Alt/Option-drag (or any mouse or pen drag while `columnSelect` is on) draws a rectangle and selects the whole runs whose centre is inside it, tinted by the layer, since a browser selection can't hold a rectangle. Escape, a new press or a browser selection ends it.
- **Deviation: command ids.** The registry needs `namespace.verb-object`, so the commands are `edit.copy` (U1b's, now with a handler), `edit.copy-exact` and `edit.select-column-mode`, plus `selection.search`, `selection.share` (only where `navigator.share` exists) and `selection.toggle-toolbar`. ADR 0024's appendix and the design language's command ids were amended; the reader registers 44 commands.
- **Deviation: settings and the context menu.** There is no settings page or context-menu component yet. The toolbar's off switch is the `selection.toggle-toolbar` command, kept in platform storage (`selection.toolbar`); the commands carry the `context-menu` home for when the menu exists, and the palette reaches each.
- **The toolbar** (`apps/web/src/selection.tsx`) shows on a settled mouse or pen selection, anchored to `selectionBox(rects)` (`@laminapdf/ui`): every rectangle of the selection, so above means above the first line and the flip goes below the last; 20 positions in `test/place.test.ts`. A scroll hides it until the next settled selection; Escape dismisses it. Search for opens find with the selected text in a new bar.
- **Touch:** a touch selection (or a keyboard one on a coarse pointer) swaps the phone's bottom bar for `SelectionActionBar` (Copy, Share, Search for, More with Copy exact); its buttons don't take the focus, so pressing one keeps the selection. On a tablet, which has no bottom bar, the OS menu alone serves.
- **Tests:** `layers/copy.test.ts` (joining, hyphen repair, soft hyphens, ligatures, tables, HTML); `test/browser/copy.browser.test.ts` scores copy as paragraph over U2b's corpus with the order and text measures of `crates/laminapdf/tests/reading_order.rs` simplified to one copied text (mean ≥ `threshold.json`'s), and covers column selection and settled events; `apps/web/e2e/selection.e2e.test.ts` drives the toolbar with a real mouse (no toolbar during the drag, none over the selection, Shift+F10, Escape, Copy, Search for, the palette) and the phone bar in the Pixel 8 and iPhone 15 profiles.
