# U6i — `@laminapdf/ui` bottom sheets and the dialog as a sheet on the phone

| Field | Value |
| --- | --- |
| ID | `U6i` |
| Status | `done` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6e](./U6e-overlays-and-sheets.md) |
| ADRs | Follows 0024 (the phone shell, sheets with detents, Escape order; from [U1a](../done/U1a-adr-0024-app-shell-ux.md)) and 0009 (accessible-primitives library; from [C2a](../done/C2a-render-adrs.md)) |
| Branch | `feat/u6i-sheets-positioning-and-palette-shell` |
| PR title | `feat(ui): add paper bottom sheets and the phone dialog` |
| Size | L (≈ 600–800 lines of components and tests; split on 2026-11-05 as its size required: anchored positioning with the selection toolbar shell, and the palette shell, moved to [U6j](./U6j-positioning-and-palette-shell.md)) |
| PR | [#247](https://github.com/laudebugs/laminapdf/pull/247) |

## Goal

Finish the overlays' phone layer. [U6e](../done/U6e-overlays-and-sheets.md) built the layer stack, the z-order, dialogs, menus, tooltips and the toast region. This todo builds the bottom sheet on that stack: detents whose grabber is a button (a single-pointer alternative to dragging), drag snapping, modal and non-modal modes, and the dialog shown as a sheet on the phone. [U6j](./U6j-positioning-and-palette-shell.md) builds the selection toolbar's positioning and the palette shell.

## Context

- Design language [§7.16 bottom sheet](../../design/paper/design-language.md#716-bottom-sheet), [§7.18 dialog](../../design/paper/design-language.md#718-dialog) (its phone form), [§6.2 phone shell](../../design/paper/design-language.md#62-phone-shell-390--844-artboard) (sheet detents: medium, large, collapsed; grabber 64 × 44 button), [§8.7 touch rules](../../design/paper/design-language.md#87-touch-rules).
- Design: F04 (Pages sheet) in [screens.md §F](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards).
- [UI/UX plan §5.4](../../plans/ui-ux.md#54-mobile-and-touch-r17) (sheets, safe areas).
- [U6e](../done/U6e-overlays-and-sheets.md): the layer stack, z-order tokens and `Dialog` the sheet joins; [U6d](../done/U6d-core-components.md) and [U6h](../done/U6h-display-components.md): the density context and the test helpers.
- Consumers: [U3a](../ready/U3a-responsive-mobile-and-touch.md) (which panels become sheets), [U6j](./U6j-positioning-and-palette-shell.md).
- WCAG 2.2 SC 2.4.11 (focus not obscured), 2.5.7 (dragging movements).

## Scope

**In scope**

- **BottomSheet** (§7.16, §6.2), on U6e's layer stack and z-order tokens:
  - detents `collapsed | medium | large` at the design language's positions;
  - the grabber is a `<button aria-label="Resize sheet">` (label from props) that cycles medium → large → medium on click, Enter or Space (WCAG 2.5.7);
  - dragging snaps to the nearest detent, and a fast drag goes one detent further;
  - a header with the title, a count and a 44 × 44 close button;
  - modal (focus trapped, scrim) or non-modal as a prop. A non-modal sheet leaves the page usable above it and doesn't trap focus, but never covers the focused element (SC 2.4.11);
  - `env(safe-area-inset-bottom)`, and under reduced motion an instant change with no slide.
- **Phone dialog**: U6e's `Dialog` renders as a large-detent `BottomSheet` on a phone-wide viewport, its actions full width.

**Out of scope** (and which todo covers it)

- Anchored positioning, the selection mini toolbar shell and the command palette shell → [U6j](./U6j-positioning-and-palette-shell.md).
- The layer stack, z-order tokens, `Dialog`, menus, tooltips and the toast region → [U6e](../done/U6e-overlays-and-sheets.md).
- Palette contents, fuzzy ranking, recents, argument parsing and Cmd/Ctrl+K → [U1b](../done/U1b-command-registry-and-palette.md).
- When the mini toolbar and the phone action bar appear, their buttons and copy modes → [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md); the phone bottom bar and action bar → [U3a](../ready/U3a-responsive-mobile-and-touch.md).
- Which panels become sheets on the phone → [U3a](../ready/U3a-responsive-mobile-and-touch.md).
- Gallery pages and screenshots → [U6f](../ready/U6f-gallery-and-visual-tests.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Re-read ADR 0024's phone-shell section and design language §6.2 and §7.16; note any gap with the primitives library's components and how this PR closes it.
- [x] `BottomSheet`: pointer handling with `pointerdown/move/up` and `setPointerCapture`, detent snapping, grabber button, modal and non-modal modes, resize on virtual keyboard (`visualViewport`), safe-area padding; tests for click-cycling, keyboard activation of the grabber, drag snapping (synthetic pointer events), reduced motion.
- [x] `Dialog` as a large-detent sheet on a phone-wide viewport.
- [x] README: the sheet in the overlays section.

## Acceptance criteria

- [x] Bottom sheet: the grabber is a button that changes detent on click, Enter and Space without any drag (SC 2.5.7 test); dragging snaps to a detent; in non-modal mode the focused element is never covered by the sheet (test scrolls a focused input under the sheet and asserts it stays visible).
- [x] A sheet, a menu inside it and a toast stack in the z-order tokens' order, and Escape closes the innermost first (test).
- [x] axe reports zero violations for each overlay open, in light, dark and high contrast.
- [x] Matches the linked Paper screen (F04); any deviation is recorded in the design language.
- [x] `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

The overlays' browser tests in `packages/ui` (Vitest browser mode, Chromium, Firefox, WebKit) with the shared helpers; touch emulation for sheets (Playwright device descriptors in the browser runner).

## Docs to update

- `packages/ui/README.md` (overlays section).
- [Design language](../../design/paper/design-language.md): §7.16 and §7.18 link their components and record deviations.

## Risks and open questions

- **Bottom sheet gestures vs the viewer's**: the viewer owns pan and pinch on the canvas (UI/UX §5.4); the sheet only handles drags that start on the sheet or the grabber. Test on real iOS Safari and Android Chrome before [U3a](../ready/U3a-responsive-mobile-and-touch.md) relies on it.

## Outcome

- `@laminapdf/ui` gains `BottomSheet`: medium (48 % down) and large (under the phone top bar) detents; a 64 × 44 grabber button that switches detent on a press, Enter or Space; drags of the grabber or header that snap, flick one detent further, or close from medium; a header with title, count and close button; a footer of full-width actions. It is modal by default, on React Aria's modal (scrim, inert page, `aria-modal`), and non-modal with `isModal={false}`, when it scrolls the page's focused element clear of itself unless focus is in a layer opened from it (`LayerStack.hasOpenedFrom`). An on-screen keyboard lifts the whole sheet.
- `Dialog` renders as a large-detent `BottomSheet` at 599 px or less, and keeps the layout it opened with until it closes.
- Tokens: `sheet-detent-large` (an alias of `phone-top-bar-height`, approved) and `sheet-detent-medium` (0.48).
- Decisions, recorded in the design language (§7.16, §7.18): a modal sheet draws in the scrim's layer (40), a non-modal one at 30; the sheet moves without a slide, so reduced motion changes nothing; it runs to the viewport's bottom, where F04 stops it above the phone bottom bar, which [U3a](../ready/U3a-responsive-mobile-and-touch.md) places.
- The browser tests run at 1280 × 800, since Vitest's default viewport is phone-wide; focus checks after an open or a close poll, for WebKit.
- The selection toolbar's positioning and the palette shell moved to [U6j](./U6j-positioning-and-palette-shell.md) when this todo was claimed.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
