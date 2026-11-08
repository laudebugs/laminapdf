# U6j — `@laminapdf/ui` anchored positioning, the selection toolbar shell and the command palette shell

| Field | Value |
| --- | --- |
| ID | `U6j` |
| Status | `done` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6i](./U6i-sheets-positioning-and-palette-shell.md) |
| ADRs | Follows 0024 (selection surfaces per pointer type, sheets with detents, palette as an APG combobox, Escape order; from [U1a](../done/U1a-adr-0024-app-shell-ux.md)) and 0009 (accessible-primitives library; from [C2a](../done/C2a-render-adrs.md)); [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) for any positioning dependency |
| Branch | `feat/u6j-positioning-and-palette-shell` |
| PR title | `feat(ui): add anchored positioning, the selection toolbar shell and the palette shell` |
| Size | L (≈ 600–800 lines of components and tests) |
| PR | [#251](https://github.com/laudebugs/laminapdf/pull/251) |

## Goal

Finish the overlays. [U6e](../done/U6e-overlays-and-sheets.md) built the layer stack, dialogs, menus, tooltips and toasts, and [U6i](../done/U6i-sheets-positioning-and-palette-shell.md) the bottom sheets. This todo builds the two layers with their own geometry and keyboard model. The first is the positioning of the desktop selection mini toolbar: above the selection, flipping on collision, reachable with Shift+F10. The second is the command palette's shell, an APG combobox with a listbox, which [U1b](../done/U1b-command-registry-and-palette.md) fills with commands. It was split from U6i when U6i was claimed, because U6i's three parts together passed the 800-line budget.

## Context

- Design language [§7.9 selection mini toolbar](../../design/paper/design-language.md#79-selection-mini-toolbar-desktop-fine-pointer), [§7.10 command palette](../../design/paper/design-language.md#710-command-palette), [§7.16 bottom sheet](../../design/paper/design-language.md#716-bottom-sheet), [§6.2 phone shell](../../design/paper/design-language.md#62-phone-shell-390--844-artboard) (sheet detents: medium, large, collapsed; grabber 64 × 44 button), [§8.7 touch rules](../../design/paper/design-language.md#87-touch-rules), [§8.8 keyboard](../../design/paper/design-language.md#88-keyboard-and-shortcuts) (Shift+F10, Escape order).
- Design: F01 (mini toolbar) and F02 (palette) in [screens.md §F](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards).
- [UI/UX plan §5.2](../../plans/ui-ux.md#52-command-registry-and-palette-r3) (palette), [§5.3](../../plans/ui-ux.md#53-selection-and-copy-r4-r5) (selection surfaces), [§5.4](../../plans/ui-ux.md#54-mobile-and-touch-r17) (sheets, safe areas).
- [U6e](../done/U6e-overlays-and-sheets.md): the layer stack, z-order tokens and `Dialog` these join; [U6d](../done/U6d-core-components.md) and [U6h](../done/U6h-display-components.md): the density context, `ShortcutChip`, `EmptyState` and the test helpers.
- Consumers: [U1b](../done/U1b-command-registry-and-palette.md) (palette contents, ranking and argument parsing), [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md) (when the mini toolbar shows and what it holds), [U3a](../ongoing/U3a-responsive-mobile-and-touch.md) (which panels become sheets).
- WCAG 2.2 SC 2.4.11 (focus not obscured); WAI-ARIA APG: combobox with listbox popup, toolbar.

## Scope

**In scope**

- **Positioning**: one `useAnchoredPosition` (or the primitives library's overlay positioning) that places a layer against an element **or a virtual rectangle** (the selection's first line), flips on collision, shifts within the viewport and the canvas, and updates on scroll and zoom.
- **Selection mini toolbar shell** (§7.9): above the first selected line, below on collision, `role="toolbar"` with roving focus; Shift+F10 moves focus into it; Escape dismisses and returns focus to the document. When it shows and what it contains are [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md)'s.
- **Command palette shell** (§7.10): modal over the scrim, 96 px from the top, `max-width` 600; input with `role="combobox"`, `aria-expanded`, `aria-controls`, `aria-activedescendant` (focus stays in the input); `role="listbox"` with grouped options (section overlines "Recent", "Commands"), option rows (title with the matched part in 600, group name, shortcut chip, disabled with reason), empty state, a "pushed page" slot for commands that need arguments, and the hint footer; arrow keys, Home/End, Page Up/Down, Enter, Escape. It takes items and a filter result as props; no ranking or commands inside.

**Out of scope** (and which todo covers it)

- The layer stack, z-order tokens, `Dialog`, menus, tooltips and the toast region → [U6e](../done/U6e-overlays-and-sheets.md).
- Palette contents, fuzzy ranking, recents, argument parsing and Cmd/Ctrl+K → [U1b](../done/U1b-command-registry-and-palette.md).
- When the mini toolbar and the phone action bar appear, their buttons and copy modes → [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md); the phone bottom bar and action bar → [U3a](../ongoing/U3a-responsive-mobile-and-touch.md).
- Which panels become sheets on the phone → [U3a](../ongoing/U3a-responsive-mobile-and-touch.md).
- Gallery pages and screenshots → [U6f](../done/U6f-gallery-and-visual-tests.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Re-read ADR 0024's selection-surface and palette sections and the APG combobox and toolbar patterns (primary W3C pages); note any gap with the primitives library's components and how this PR closes it.
- [x] `useAnchoredPosition` with virtual-rect anchors: use the primitives library's positioning if it supports virtual anchors; otherwise `@floating-ui/dom` (MIT; licence-check). Tests for flip, shift and update on scroll.
- [x] `SelectionToolbarShell` (positioning, roving focus, Shift+F10, Escape) with a fake selection rect in tests.
- [x] `CommandPaletteShell` with the combobox semantics; tests asserting `aria-activedescendant` follows the active option, focus never leaves the input, disabled options are announced and skipped, sections are labelled groups.
- [x] README: the mini toolbar and the palette in the overlays section.

## Acceptance criteria

- [x] The palette shell follows the APG combobox model (tests list each key); the input keeps DOM focus while arrow keys change `aria-activedescendant`.
- [x] The mini toolbar shell sits above a given rect, flips below when there is no room, stays inside the viewport, and Shift+F10 focuses it (tests).
- [x] axe reports zero violations for each overlay open, in light, dark and high contrast.
- [x] Matches the linked Paper screens (F01, F02); any deviation is recorded in the design language.
- [x] `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

The overlays' browser tests in `packages/ui` (Vitest browser mode, Chromium, Firefox, WebKit) with the shared helpers; unit tests for the positioning maths.

## Docs to update

- `packages/ui/README.md` (overlays section).
- [Design language](../../design/paper/design-language.md): §7.9 and §7.10 link their components and record deviations.

## Risks and open questions

- **Palette library**: [U1b](../done/U1b-command-registry-and-palette.md) planned cmdk. cmdk renders its own input and list, so it would bypass this shell. Recommendation: the shell owns the ARIA and rendering, and U1b uses a ranking function only (cmdk's scoring or its own); agree this with whoever claims U1b and update U1b's text in this PR's claim or a board-only PR.

## Outcome

- `@laminapdf/ui` gains `SelectionToolbar` with `SelectionToolbarDivider` (§7.9) and `CommandPalette` (§7.10), and the placement rule they need, `placeAbove` in `src/overlays/place.ts`: a pure function, tested in Node, that centres a layer on a rectangle, above it or below on collision, kept a margin inside the bounds, and returns nothing once the rectangle has left them.
- Gaps with React Aria, closed here: its overlay positioning takes an element, not a rectangle such as a selection's first line, so the toolbar uses `placeAbove` rather than a dependency (`@floating-ui/dom` was not needed); its autocomplete gives the input virtual focus over the listbox but leaves out `role="combobox"` and `aria-expanded`, which the palette sets.
- The toolbar re-places itself after every render and on any scroll, resize or size change, hides while the selection is out of view, takes Shift+F10 unless a modal has made it inert, and returns focus on Escape to wherever it came from. The palette shell answers every APG combobox key from its input, keeps a disabled row's reason in place of its group, merges overlapping match ranges and shows the empty state once every section is empty.
- Tokens: the palette's offset, width, input and row heights, and the toolbar divider's height, from the design files.
- Checked against F01 and F02 side by side: the toolbar matches; the palette matched once its active row dropped a second focus ring (the input keeps that ring). The selection toolbar and the palette take `raised`, as every floating layer does.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
