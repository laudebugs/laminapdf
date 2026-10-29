# U2a — Selection surfaces and copy modes (copy as paragraph, copy exact, column selection)

| Field | Value |
| --- | --- |
| ID | `U2a` |
| Status | `ready` |
| Plan | [UI/UX §6 U2. Reading quality](../../plans/ui-ux.md#u2-reading-quality) |
| Round / milestone | Round U (UI/UX) — M2 Web reader v0.1 |
| Depends on | [C5a](./C5a-text-layer-and-selection.md), [U1b](./U1b-command-registry-and-palette.md), [U2b](./U2b-reading-order-ground-truth.md), [U3a](./U3a-responsive-mobile-and-touch.md) |
| Blocks | [C7c](./C7c-performance-gates-and-launch.md) |
| ADRs | Implements 0024 (selection surfaces); follows 0009 |
| Branch | `feat/u2a-selection-surfaces` |
| PR title | `feat(ui): add the selection toolbar, the touch action bar and copy modes` |
| Size | M (300–600 lines incl. tests) |
| PR | — |

## Goal

Fix the friction users report most after speed: copying from multi-column PDFs grabs the wrong order and pastes hard line breaks. Add a desktop selection toolbar and a touch bottom action bar (never covering the OS selection menu), make "copy as paragraph" the default, and add "copy exact" and rectangular column selection.

## Context

- [UI/UX plan §5.3](../../plans/ui-ux.md#53-selection-and-copy-r4-r5) and ADR 0024.
- [Research: selection menus](../../research/product/pdf-app-product-and-ux.md#selection-menus-float-them-on-desktop-never-over-the-os-menu-on-mobile) — Android ActionMode, iOS UIEditMenuInteraction, Wikimedia T202723.
- [Competitive teardown: friction](../../research/product/notes/competitive-teardown.md) — multi-column copy, hard line breaks, Okular's "copy without line breaks", Preview's Option-drag.
- [C5a](./C5a-text-layer-and-selection.md) — the text layer and the selection events with rects and runs.
- [U2b](./U2b-reading-order-ground-truth.md) — the reading-order metric copy-as-paragraph is held to.

## Scope

**In scope**

- `SelectionToolbar` (fine pointer): above the first selected line, flips below on collision, appears on `pointerup`, `role="toolbar"`, Shift+F10 to focus, Escape to dismiss; actions Copy, Copy exact, Search for, and editor actions when their modes exist; can be turned off in settings.
- `SelectionActionBar` (coarse pointer): replaces U3a's phone bottom bar while a selection exists (Copy, Share, Search for, More); never positioned over the native menu.
- Copy modes in the clipboard path: paragraph (reading order from lamina's extraction, joined lines, de-hyphenated, paragraph breaks kept) and exact (layout line breaks); both also write `text/html` with paragraphs.
- Rectangular selection with Alt/Option-drag (desktop) that selects within a column box.
- Commands registered in U1b: `copy`, `copy-exact`, `select-column-mode`.

**Out of scope** (and which todo covers it)

- Highlight, underline, comment actions → [E4f](./E4f-annotation-tools-ui.md) adds them to these surfaces.
- Reading-order corpus and metric → [U2b](./U2b-reading-order-ground-truth.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Subscribe to C5a's selection events; compute placement with a small collision routine (no positioning library unless its licence is checked).
- [ ] Implement both copy modes on top of the extraction runs; unit-test joining, hyphen repair (soft hyphen, hyphen at line end followed by lowercase), and ligature expansion.
- [ ] Implement rectangular selection and its hit-testing against text runs.
- [ ] Hide the toolbar on scroll start and while the user drags; reshow on `pointerup`.
- [ ] Touch: detect `pointer: coarse`; swap the bottom bar; verify on iOS Safari and Android Chrome that the native menu still appears and is not covered.

## Acceptance criteria

- [ ] On U2b's multi-column fixtures, copy-as-paragraph meets the U2b threshold; copy exact reproduces line breaks.
- [ ] Pasting a copied paragraph into a plain textarea gives no line breaks inside paragraphs (test).
- [ ] The toolbar never covers selected text (geometry test over 20 selection positions, including page edges).
- [ ] Playwright mobile profiles: the action bar appears on a touch selection and never overlaps the selected text's rects (emulation doesn't render the OS selection menu); on a real iPhone (Safari) and Android phone (Chrome), the native menu stays visible and uncovered — a recorded manual check in the PR.
- [ ] Keyboard: Shift+F10 focuses the toolbar; every toolbar action is also in the context menu and palette.

## Post-merge checks

None.

## Tests

Unit tests for the copy pipeline in `packages/ui` (or `@lamina/viewer` if the logic lives there); Vitest browser tests for the toolbar; Playwright desktop and mobile profiles.

## Docs to update

Tick the selection and copy items in [UI/UX §9](../../plans/ui-ux.md#9-to-dos); a note in `apps/web/ACCESSIBILITY.md` once it exists.

## Risks and open questions

- Paragraph joining is wrong for verse, code and tables: fall back to exact when the extraction marks a block as preformatted or tabular.
- Browser clipboard permissions differ: use the async clipboard API in the user gesture, with the `copy` event as fallback.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
