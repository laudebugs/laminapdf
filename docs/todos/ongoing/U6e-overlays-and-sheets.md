# U6e — `@laminapdf/ui` overlays: the layer stack, z-order, dialogs, popovers, menus, tooltips and the toast region

| Field | Value |
| --- | --- |
| ID | `U6e` |
| Status | `ongoing` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6d](../done/U6d-core-components.md), [U6h](../done/U6h-display-components.md) |
| Blocks | [C4a](../ready/C4a-ui-chrome-and-platform.md), [U1b](../ready/U1b-command-registry-and-palette.md), [U6f](../ready/U6f-gallery-and-visual-tests.md), [U6i](../ready/U6i-sheets-positioning-and-palette-shell.md) |
| ADRs | Follows 0024 (Escape order; from [U1a](../done/U1a-adr-0024-app-shell-ux.md)) and 0009 (accessible-primitives library; from [C2a](../done/C2a-render-adrs.md)) |
| Branch | `feat/u6e-overlays-and-sheets` |
| PR title | `feat(ui): add the paper layer stack, dialogs, menus, tooltips and toasts` |
| Size | L (≈ 600–800 lines of components and tests; split on 2026-11-05 as its size required: bottom sheets, anchored positioning with the selection toolbar shell, and the palette shell moved to [U6i](../ready/U6i-sheets-positioning-and-palette-shell.md)) |
| PR | — |

## Goal

Give every short-lived layer in lamina one implementation with correct focus, keyboard, pointer and stacking behaviour: one stack of open layers so Escape closes the innermost first; modal dialogs that trap and return focus; popovers, menus and tooltips; and a toast region that stacks and times messages. Z-order tokens make the layering rules explicit. [U6i](../ready/U6i-sheets-positioning-and-palette-shell.md) builds the bottom sheets, the selection toolbar's positioning and the palette shell on this layer stack.

## Context

- Design language [§7.18 dialog](../../design/paper/design-language.md#718-dialog), [§7.20 toast and notice](../../design/paper/design-language.md#720-toast-and-notice); [§4](../../design/paper/design-language.md#4-space-shape-depth) (floating shadows, `scrim`, and the z-order table); [§8.8 keyboard](../../design/paper/design-language.md#88-keyboard-and-shortcuts) (Escape closes the innermost layer); [§8.9 checklist](../../design/paper/design-language.md#89-accessibility-checklist-wcag-22-aa).
- [U6d](../done/U6d-core-components.md) and [U6h](../done/U6h-display-components.md): the buttons, icon buttons, `Toast` look, `ShortcutChip` and the shared test helpers this todo builds on.
- Consumers: [C4a](../ready/C4a-ui-chrome-and-platform.md) (password dialog, zoom menu, overflow menu) and [U6i](../ready/U6i-sheets-positioning-and-palette-shell.md) (sheets, the mini toolbar and the palette on the same layer stack).
- WCAG 2.2 SC 2.1.2 (no keyboard trap), 2.4.3 (focus order), 2.4.11 (focus not obscured), 4.1.3 (status messages); WAI-ARIA APG: dialog (modal), menu button and menu, tooltip.

## Scope

**In scope**

- **Dialog** (§7.18): `role="dialog"` (or `alertdialog` for destructive confirms), `aria-modal="true"`, `aria-labelledby` the title, optional `aria-describedby`; focus moves to the first field or the least destructive action, is trapped, and returns to the opener (or a fallback the caller names if the opener is gone); Escape closes unless the dialog asks a required question; scrim click closes non-required dialogs; the rest of the app is `inert`; scroll lock without layout shift; `max-width` 440, 560 for forms, and a wide 820 variant for two-pane and live-preview dialogs (decision 11; page marks A03, document info O04, consent dialogs X01/X02/X05); actions right-aligned, primary last.
- **Popover** and **Menu** (menu button pattern: `aria-haspopup`, `aria-expanded`, arrow keys, typeahead, Home/End, Escape returns focus to the button; menu items with icon, label, shortcut chip, disabled with reason); **Tooltip** (shows on hover and focus, dismissible with Escape, never the only place for information, shows the shortcut chip).
- **Toast region** (§7.20): one `role="status"` region (polite), bottom centre 24 px above the status bar (phone: above the bottom bar, replacing the page pill); stacks up to three, newest at the bottom, older ones collapse; 4 s timeout paused on hover and focus; optional action (e.g. "Undo") reachable by keyboard (F6 or a documented shortcut) without stealing focus; `toast()` API returning a dismiss handle.
- **Layer manager**: one stack of open layers so Escape closes the innermost (menu → dialog …) and focus restoration is consistent; nested layers (a menu inside a dialog) work. U6i's sheets, mini toolbar and palette join the same stack.
- **Z-order tokens** added to [`@laminapdf/tokens`](../done/U6a-paper-tokens.md) (`z.page`, `z.page-overlay`, `z.chrome`, `z.floating-toolbar`, `z.popover`, `z.sheet`, `z.modal`, `z.toast`) from the design language's z-order table (the owner approved its values on 2026-11-05), and used by every overlay here.

**Out of scope** (and which todo covers it)

- Bottom sheets and the phone dialog as a sheet, anchored positioning with virtual rectangles and the selection mini toolbar shell, and the command palette shell → [U6i](../ready/U6i-sheets-positioning-and-palette-shell.md).
- Palette contents, fuzzy ranking, recents, argument parsing and Cmd/Ctrl+K → [U1b](../ready/U1b-command-registry-and-palette.md).
- When the mini toolbar and the phone action bar appear, their buttons and copy modes → [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md); the phone bottom bar and action bar components → [U3a](../ready/U3a-responsive-mobile-and-touch.md).
- Which panels become sheets on the phone (outline, find, view settings) → [U3a](../ready/U3a-responsive-mobile-and-touch.md); the inspector as a sheet → [U3a](../ready/U3a-responsive-mobile-and-touch.md) / [U1c](../ready/U1c-inverted-l-shell-and-modes.md).
- Specific dialogs (password, properties, print, settings, close with unsaved changes) → their todos ([C4a](../ready/C4a-ui-chrome-and-platform.md), [C6c](../ready/C6c-print-download-properties.md), [C7a](../ready/C7a-accessibility-and-theming.md), [E2i](../ready/E2i-signed-gating-and-close.md)).
- F6 region cycling and the global shortcut dispatcher → [C7a](../ready/C7a-accessibility-and-theming.md), [U1b](../ready/U1b-command-registry-and-palette.md).
- Gallery pages and screenshots → [U6f](../ready/U6f-gallery-and-visual-tests.md).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Re-read ADR 0024's Escape order, and the APG dialog, menu button and tooltip patterns (primary W3C pages); note any gap with the primitives library's components and how this PR closes it.
- [ ] Add the z-order tokens to `@laminapdf/tokens` (JSON, regenerate, tests), marked approved (the owner approved the table on 2026-11-05).
- [ ] `src/overlays/layer-stack.ts`: a small stack (push/pop, `isTopmost`, Escape routing, focus restore targets) with unit tests.
- [ ] `Dialog` on the primitives library's modal (React Aria `Modal`/`Dialog` recommended) styled per §7.18; `inert` on the app root; tests for trap, return focus, Escape, required-question mode, scrim click, nested menu.
- [ ] `Popover`, `Menu`, `MenuButton`, `Tooltip` on the primitives library; tests for the APG keyboard model and typeahead.
- [ ] `ToastRegion` and `toast()`; tests for stacking, timeout pause on hover and focus, the action's keyboard reach, announcements via the live region.
- [ ] README section: which overlay to use when (dialog vs popover vs notice vs toast), the layer order and Escape order.

## Acceptance criteria

- [ ] Dialog: focus is trapped, Escape closes, focus returns to the opener; `aria-modal`, labelled title; background is `inert` (tests in three engines).
- [ ] Menus follow the APG menu button model (tests list each key: arrows, Home/End, typeahead, Escape returning focus).
- [ ] Toasts stack (max three visible), pause on hover/focus, and are announced politely; errors are never only in a toast (documented in the API; the `toast()` type has no `error` variant).
- [ ] Every overlay uses a z-order token; a test opens a dialog, a menu inside it and a toast, and asserts their computed stacking order and that Escape closes the innermost first.
- [ ] axe reports zero violations for each overlay open, in light, dark and high contrast.
- [ ] Any deviation from the design language's §7.18 and §7.20 is recorded there.
- [ ] `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

`packages/ui/src/overlays/**/*.test.tsx` (Vitest browser mode, Chromium, Firefox, WebKit) with the shared state and axe helpers from [U6d](../done/U6d-core-components.md); unit tests for the layer stack.

## Docs to update

- `packages/ui/README.md` (overlays section).
- [Design language](../../design/paper/design-language.md): §4 z-order table no longer **(proposed)** (approved 2026-11-05), and links to the tokens; §7.18 and §7.20 link their components and record deviations.
- [UI/UX plan](../../plans/ui-ux.md) §9: nothing to tick (U1b and U2a own the palette and selection to-dos).

## Risks and open questions

- **Order with C4a**: this todo should land before [C4a](../ready/C4a-ui-chrome-and-platform.md) so its password dialog and menus use these overlays (see [U6d](../done/U6d-core-components.md)'s risks); C4a's **Depends on** gains U6e.
- **`inert` and the desktop webview**: `inert` is supported in current WebKit, Blink and Gecko; confirm in the Tauri webviews (WKWebView, WebView2, WebKitGTK) when [D2a](../ready/D2a-adr-0008-and-desktop-scaffold.md) lands.
- **Toast action reachability** without moving focus is awkward for keyboard users; if no clean pattern exists, actions in toasts are limited to ones that are also available elsewhere (e.g. Undo in the Edit menu and `Mod Z`).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
