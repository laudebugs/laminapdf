# U6e — `@laminapdf/ui` overlays: dialogs, bottom sheets, popovers and menus, selection toolbar positioning, palette shell, toasts, z-order

| Field | Value |
| --- | --- |
| ID | `U6e` |
| Status | `ready` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6d](./U6d-core-components.md) |
| Blocks | [C4a](./C4a-ui-chrome-and-platform.md), [U1b](./U1b-command-registry-and-palette.md), [U6f](./U6f-gallery-and-visual-tests.md) |
| ADRs | Follows 0024 (selection surfaces per pointer type, sheets with detents, palette as an APG combobox, Escape order; from [U1a](../done/U1a-adr-0024-app-shell-ux.md)) and 0009 (accessible-primitives library; from [C2a](../done/C2a-render-adrs.md)); [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) for any positioning dependency |
| Branch | `feat/u6e-overlays-and-sheets` |
| PR title | `feat(ui): add paper dialogs, bottom sheets, menus, palette shell and toasts` |
| Size | L (≈ 600–800 lines of components and tests) |
| PR | — |

## Goal

Give every short-lived layer in lamina one implementation with correct focus, keyboard, pointer and stacking behaviour: modal dialogs that trap and return focus; phone bottom sheets with detents whose grabber is a real button (a single-pointer alternative to dragging); popovers and menus; the positioning logic of the desktop selection mini toolbar (above the selection, flipping on collision, reachable with Shift+F10); the command palette's shell (an APG combobox with a listbox) that [U1b](./U1b-command-registry-and-palette.md) fills with commands; and a toast region that stacks and times messages. Z-order tokens make the layering rules explicit, so the Escape order (innermost layer first) and "focus not obscured" hold across every screen.

## Context

- Design language [§7.9 selection mini toolbar](../../design/paper/design-language.md#79-selection-mini-toolbar-desktop-fine-pointer), [§7.10 command palette](../../design/paper/design-language.md#710-command-palette), [§7.16 bottom sheet](../../design/paper/design-language.md#716-bottom-sheet), [§7.18 dialog](../../design/paper/design-language.md#718-dialog-proposed), [§7.20 toast and notice](../../design/paper/design-language.md#720-toast-and-notice-proposed); [§4](../../design/paper/design-language.md#4-space-shape-depth) (floating shadows, `scrim`, and the proposed z-order table); [§6.2 phone shell](../../design/paper/design-language.md#62-phone-shell-390--844-artboard) (sheet detents: medium, large, collapsed; grabber 64 × 44 button); [§8.7 touch rules](../../design/paper/design-language.md#87-touch-rules); [§8.8 keyboard](../../design/paper/design-language.md#88-keyboard-and-shortcuts) (Escape closes the innermost layer; Shift+F10); [§8.9 checklist](../../design/paper/design-language.md#89-accessibility-checklist-wcag-22-aa).
- Design: F01 (mini toolbar), F02 (palette), F04 (Pages sheet) in [screens.md §F](../../design/paper/screens.md#f-foundations-the-chosen-paper-boards).
- [UI/UX plan §5.2](../../plans/ui-ux.md#52-command-registry-and-palette-r3) (palette), [§5.3](../../plans/ui-ux.md#53-selection-and-copy-r4-r5) (selection surfaces), [§5.4](../../plans/ui-ux.md#54-mobile-and-touch-r17) (sheets, safe areas).
- [U6d](./U6d-core-components.md) — buttons, icon buttons, toast visual, shortcut chip and the shared test helpers this todo builds on.
- Consumers: [U1b](./U1b-command-registry-and-palette.md) (palette contents, ranking and argument parsing), [U2a](./U2a-selection-surfaces-and-copy-modes.md) (when the mini toolbar shows and what it holds), [U3a](./U3a-responsive-mobile-and-touch.md) (which panels become sheets), [C4a](./C4a-ui-chrome-and-platform.md) (password dialog, zoom menu, overflow menu).
- WCAG 2.2 SC 2.1.2 (no keyboard trap), 2.4.3 (focus order), 2.4.11 (focus not obscured), 2.5.7 (dragging movements), 4.1.3 (status messages); WAI-ARIA APG: dialog (modal), menu button and menu, combobox with listbox popup, toolbar.

## Scope

**In scope**

- **Dialog** (§7.18): `role="dialog"` (or `alertdialog` for destructive confirms), `aria-modal="true"`, `aria-labelledby` the title, optional `aria-describedby`; focus moves to the first field or the least destructive action, is trapped, and returns to the opener (or a fallback the caller names if the opener is gone); Escape closes unless the dialog asks a required question; scrim click closes non-required dialogs; the rest of the app is `inert`; scroll lock without layout shift; `max-width` 440, 560 for forms, and a wide 820 variant for two-pane and live-preview dialogs (decision 11; page marks A03, document info O04, consent dialogs X01/X02/X05); actions right-aligned, primary last. On the phone it renders as a bottom sheet at the large detent.
- **BottomSheet** (§7.16, §6.2): detents `collapsed | medium | large` with the design language's positions; the grabber is a `<button aria-label="Resize sheet">` that cycles medium → large → medium on click, Enter or Space (WCAG 2.5.7); dragging snaps to the nearest detent with velocity; header with title, count and a 44 × 44 close button; modal (focus trapped, scrim) or non-modal (page stays interactive above a medium sheet; focus not trapped, but the sheet never covers the focused element: SC 2.4.11) as a prop; `env(safe-area-inset-bottom)`; reduced motion → no slide, instant change.
- **Popover** and **Menu** (menu button pattern: `aria-haspopup`, `aria-expanded`, arrow keys, typeahead, Home/End, Escape returns focus to the button; menu items with icon, label, shortcut chip, disabled with reason); **Tooltip** (shows on hover and focus, dismissible with Escape, never the only place for information, shows the shortcut chip).
- **Positioning**: one `useAnchoredPosition` (or the primitives library's overlay positioning) that places a layer against an element **or a virtual rectangle** (the selection's first line), flips on collision, shifts within the viewport and the canvas, and updates on scroll and zoom. The **selection mini toolbar** (§7.9) shell uses it: above the first selected line, below on collision, `role="toolbar"` with roving focus; Shift+F10 moves focus into it; Escape dismisses and returns focus to the document. When it shows and what it contains are [U2a](./U2a-selection-surfaces-and-copy-modes.md)'s.
- **Command palette shell** (§7.10): modal over the scrim, 96 px from the top, `max-width` 600; input with `role="combobox"`, `aria-expanded`, `aria-controls`, `aria-activedescendant` (focus stays in the input); `role="listbox"` with grouped options (section overlines "Recent", "Commands"), option rows (title with the matched part in 600, group name, shortcut chip, disabled with reason), empty state, a "pushed page" slot for commands that need arguments, and the hint footer; arrow keys, Home/End, Page Up/Down, Enter, Escape. It takes items and a filter result as props; no ranking or commands inside.
- **Toast region** (§7.20): one `role="status"` region (polite), bottom centre 24 px above the status bar (phone: above the bottom bar, replacing the page pill); stacks up to three, newest at the bottom, older ones collapse; 4 s timeout paused on hover and focus; optional action (e.g. "Undo") reachable by keyboard (F6 or a documented shortcut) without stealing focus; `toast()` API returning a dismiss handle.
- **Layer manager**: one stack of open layers so Escape closes the innermost (mini toolbar → palette/dialog → menu …) and focus restoration is consistent; nested layers (a menu inside a dialog) work.
- **Z-order tokens** added to [`@laminapdf/tokens`](./U6a-paper-tokens.md) (`z.page`, `z.page-overlay`, `z.chrome`, `z.floating-toolbar`, `z.popover`, `z.sheet`, `z.modal`, `z.toast`) from the design language's proposed table, with owner sign-off as in U6a, and used by every overlay here.

**Out of scope** (and which todo covers it)

- Palette contents, fuzzy ranking, recents, argument parsing and Cmd/Ctrl+K → [U1b](./U1b-command-registry-and-palette.md).
- When the mini toolbar and the phone action bar appear, their buttons and copy modes → [U2a](./U2a-selection-surfaces-and-copy-modes.md); the phone bottom bar and action bar components → [U3a](./U3a-responsive-mobile-and-touch.md).
- Which panels become sheets on the phone (outline, find, view settings) → [U3a](./U3a-responsive-mobile-and-touch.md); the inspector as a sheet → [U3a](./U3a-responsive-mobile-and-touch.md) / [U1c](./U1c-inverted-l-shell-and-modes.md).
- Specific dialogs (password, properties, print, settings, close with unsaved changes) → their todos ([C4a](./C4a-ui-chrome-and-platform.md), [C6c](./C6c-print-download-properties.md), [C7a](./C7a-accessibility-and-theming.md), [E2i](./E2i-signed-gating-and-close.md)).
- F6 region cycling and the global shortcut dispatcher → [C7a](./C7a-accessibility-and-theming.md), [U1b](./U1b-command-registry-and-palette.md).
- Gallery pages and screenshots → [U6f](./U6f-gallery-and-visual-tests.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Re-read ADR 0024's Escape order, selection-surface and palette sections, and the APG dialog, menu button, combobox and toolbar patterns (primary W3C pages); note any gap with the primitives library's components and how this PR closes it.
- [ ] Add the z-order tokens to `@laminapdf/tokens` (JSON, regenerate, tests) and get the owner's approval of the proposed values.
- [ ] `src/overlays/layer-stack.ts`: a small stack (push/pop, `isTopmost`, Escape routing, focus restore targets) with unit tests.
- [ ] `Dialog` on the primitives library's modal (React Aria `Modal`/`Dialog` recommended) styled per §7.18; `inert` on the app root; tests for trap, return focus, Escape, required-question mode, scrim click, nested menu.
- [ ] `BottomSheet`: pointer handling with `pointerdown/move/up` and `setPointerCapture`, detent snapping, grabber button, modal and non-modal modes, resize on virtual keyboard (`visualViewport`), safe-area padding; tests for click-cycling, keyboard activation of the grabber, drag snapping (synthetic pointer events), reduced motion.
- [ ] `Popover`, `Menu`, `MenuButton`, `Tooltip` on the primitives library; tests for the APG keyboard model and typeahead.
- [ ] `useAnchoredPosition` with virtual-rect anchors: use the primitives library's positioning if it supports virtual anchors; otherwise `@floating-ui/dom` (MIT; licence-check). Tests for flip, shift and update on scroll.
- [ ] `SelectionToolbarShell` (positioning, roving focus, Shift+F10, Escape) with a fake selection rect in tests.
- [ ] `CommandPaletteShell` with the combobox semantics; tests asserting `aria-activedescendant` follows the active option, focus never leaves the input, disabled options are announced and skipped, sections are labelled groups.
- [ ] `ToastRegion` and `toast()`; tests for stacking, timeout pause on hover and focus, the action's keyboard reach, announcements via the live region.
- [ ] Phone variant: `Dialog` renders as a large-detent `BottomSheet` under the touch density (from [U6d](./U6d-core-components.md)).
- [ ] README section: which overlay to use when (dialog vs sheet vs popover vs notice), the layer order and Escape order.

## Acceptance criteria

- [ ] Dialog: focus is trapped, Escape closes, focus returns to the opener; `aria-modal`, labelled title; background is `inert` (tests in three engines).
- [ ] Bottom sheet: the grabber is a button that changes detent on click, Enter and Space without any drag (SC 2.5.7 test); dragging snaps to a detent; in non-modal mode the focused element is never covered by the sheet (test scrolls a focused input under the sheet and asserts it stays visible).
- [ ] Menus and the palette shell follow the APG keyboard models (tests list each key); the palette input keeps DOM focus while arrow keys change `aria-activedescendant`.
- [ ] The mini toolbar shell sits above a given rect, flips below when there is no room, stays inside the viewport, and Shift+F10 focuses it (tests).
- [ ] Toasts stack (max three visible), pause on hover/focus, and are announced politely; errors are never only in a toast (documented in the API; the `toast()` type has no `error` variant).
- [ ] Every overlay uses a z-order token; a test opens a sheet, a menu inside it and a toast, and asserts their computed stacking order and that Escape closes the innermost first.
- [ ] axe reports zero violations for each overlay open, in light, dark and high contrast.
- [ ] Matches the linked Paper screens (F01, F02, F04); any deviation is recorded in the design language.
- [ ] `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

`packages/ui/src/overlays/**/*.test.tsx` (Vitest browser mode, Chromium, Firefox, WebKit) with the shared state and axe helpers from [U6d](./U6d-core-components.md); unit tests for the layer stack and positioning maths; touch emulation for sheets (Playwright device descriptors in the browser runner).

## Docs to update

- `packages/ui/README.md` (overlays section).
- [Design language](../../design/paper/design-language.md): §4 z-order table no longer **(proposed)** once approved, and links to the tokens; §7.9, §7.10, §7.16, §7.18, §7.20 link their components and record deviations.
- [UI/UX plan](../../plans/ui-ux.md) §9: nothing to tick (U1b and U2a own the palette and selection to-dos).

## Risks and open questions

- **Palette library**: [U1b](./U1b-command-registry-and-palette.md) planned cmdk. cmdk renders its own input and list, so it would bypass this shell. Recommendation: the shell owns the ARIA and rendering, and U1b uses a ranking function only (cmdk's scoring or its own); agree this with whoever claims U1b and update U1b's text in this PR's claim or a board-only PR.
- **Order with C4a**: this todo should land before [C4a](./C4a-ui-chrome-and-platform.md) so its password dialog and menus use these overlays (see [U6d](./U6d-core-components.md)'s risks); C4a's **Depends on** gains U6e.
- **Bottom sheet gestures vs the viewer's**: the viewer owns pan and pinch on the canvas (UI/UX §5.4); the sheet only handles drags that start on the sheet or the grabber. Test on real iOS Safari and Android Chrome before [U3a](./U3a-responsive-mobile-and-touch.md) relies on it.
- **`inert` and the desktop webview**: `inert` is supported in current WebKit, Blink and Gecko; confirm in the Tauri webviews (WKWebView, WebView2, WebKitGTK) when [D2a](./D2a-adr-0008-and-desktop-scaffold.md) lands.
- **Toast action reachability** without moving focus is awkward for keyboard users; if no clean pattern exists, actions in toasts are limited to ones that are also available elsewhere (e.g. Undo in the Edit menu and `Mod Z`).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
