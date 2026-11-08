# U6e — `@laminapdf/ui` overlays: the layer stack, z-order, dialogs, popovers, menus, tooltips and the toast region

| Field | Value |
| --- | --- |
| ID | `U6e` |
| Status | `done` |
| Plan | [Paper design §6 U6](../../plans/paper-design.md#u6-design-system) |
| Round / milestone | Round U (UI/UX) — U6 Paper design system |
| Depends on | [U6d](./U6d-core-components.md), [U6h](./U6h-display-components.md) |
| ADRs | Follows 0024 (Escape order; from [U1a](./U1a-adr-0024-app-shell-ux.md)) and 0009 (accessible-primitives library; from [C2a](./C2a-render-adrs.md)) |
| Branch | `feat/u6e-overlays-and-sheets` |
| PR title | `feat(ui): add the paper layer stack, dialogs, menus, tooltips and toasts` |
| Size | L (≈ 600–800 lines of components and tests; split on 2026-11-05 as its size required: bottom sheets, anchored positioning with the selection toolbar shell, and the palette shell moved to [U6i](./U6i-sheets-positioning-and-palette-shell.md)) |
| PR | [#240](https://github.com/laudebugs/laminapdf/pull/240) |

## Goal

Give every short-lived layer in lamina one implementation with correct focus, keyboard, pointer and stacking behaviour: one stack of open layers so Escape closes the innermost first; modal dialogs that trap and return focus; popovers, menus and tooltips; and a toast region that stacks and times messages. Z-order tokens make the layering rules explicit. [U6i](./U6i-sheets-positioning-and-palette-shell.md) builds the bottom sheets, the selection toolbar's positioning and the palette shell on this layer stack.

## Context

- Design language [§7.18 dialog](../../design/paper/design-language.md#718-dialog), [§7.20 toast and notice](../../design/paper/design-language.md#720-toast-and-notice); [§4](../../design/paper/design-language.md#4-space-shape-depth) (floating shadows, `scrim`, and the z-order table); [§8.8 keyboard](../../design/paper/design-language.md#88-keyboard-and-shortcuts) (Escape closes the innermost layer); [§8.9 checklist](../../design/paper/design-language.md#89-accessibility-checklist-wcag-22-aa).
- [U6d](./U6d-core-components.md) and [U6h](./U6h-display-components.md): the buttons, icon buttons, `Toast` look, `ShortcutChip` and the shared test helpers this todo builds on.
- Consumers: [C4a](./C4a-ui-chrome-and-platform.md) (password dialog, zoom menu, overflow menu) and [U6i](./U6i-sheets-positioning-and-palette-shell.md) (sheets, the mini toolbar and the palette on the same layer stack).
- WCAG 2.2 SC 2.1.2 (no keyboard trap), 2.4.3 (focus order), 2.4.11 (focus not obscured), 4.1.3 (status messages); WAI-ARIA APG: dialog (modal), menu button and menu, tooltip.

## Scope

**In scope**

- **Dialog** (§7.18): `role="dialog"` (or `alertdialog` for destructive confirms), `aria-modal="true"`, `aria-labelledby` the title, optional `aria-describedby`; focus moves to the first field or the least destructive action, is trapped, and returns to the opener (or a fallback the caller names if the opener is gone); Escape closes unless the dialog asks a required question; scrim click closes non-required dialogs; the rest of the app is `inert`; scroll lock without layout shift; `max-width` 440, 560 for forms, and a wide 820 variant for two-pane and live-preview dialogs (decision 11; page marks A03, document info O04, consent dialogs X01/X02/X05); actions right-aligned, primary last.
- **Popover** and **Menu** (menu button pattern: `aria-haspopup`, `aria-expanded`, arrow keys, typeahead, Home/End, Escape returns focus to the button; menu items with icon, label, shortcut chip, disabled with reason); **Tooltip** (shows on hover and focus, dismissible with Escape, never the only place for information, shows the shortcut chip).
- **Toast region** (§7.20): one `role="status"` region (polite), bottom centre 24 px above the status bar (phone: above the bottom bar, replacing the page pill); stacks up to three, newest at the bottom, older ones collapse; 4 s timeout paused on hover and focus; optional action (e.g. "Undo") reachable by keyboard (F6 or a documented shortcut) without stealing focus; `toast()` API returning a dismiss handle.
- **Layer manager**: one stack of open layers so Escape closes the innermost (menu → dialog …) and focus restoration is consistent; nested layers (a menu inside a dialog) work. U6i's sheets, mini toolbar and palette join the same stack.
- **Z-order tokens** added to [`@laminapdf/tokens`](./U6a-paper-tokens.md) (`z.page`, `z.page-overlay`, `z.chrome`, `z.floating-toolbar`, `z.popover`, `z.sheet`, `z.modal`, `z.toast`) from the design language's z-order table (the owner approved its values on 2026-11-05), and used by every overlay here.

**Out of scope** (and which todo covers it)

- Bottom sheets and the phone dialog as a sheet, anchored positioning with virtual rectangles and the selection mini toolbar shell, and the command palette shell → [U6i](./U6i-sheets-positioning-and-palette-shell.md).
- Palette contents, fuzzy ranking, recents, argument parsing and Cmd/Ctrl+K → [U1b](../ready/U1b-command-registry-and-palette.md).
- When the mini toolbar and the phone action bar appear, their buttons and copy modes → [U2a](../ready/U2a-selection-surfaces-and-copy-modes.md); the phone bottom bar and action bar components → [U3a](../ready/U3a-responsive-mobile-and-touch.md).
- Which panels become sheets on the phone (outline, find, view settings) → [U3a](../ready/U3a-responsive-mobile-and-touch.md); the inspector as a sheet → [U3a](../ready/U3a-responsive-mobile-and-touch.md) / [U1c](../ready/U1c-inverted-l-shell-and-modes.md).
- Specific dialogs (password, properties, print, settings, close with unsaved changes) → their todos ([C4a](./C4a-ui-chrome-and-platform.md), [C6c](../ready/C6c-print-download-properties.md), [C7a](../ready/C7a-accessibility-and-theming.md), [E2i](../ready/E2i-signed-gating-and-close.md)).
- F6 region cycling and the global shortcut dispatcher → [C7a](../ready/C7a-accessibility-and-theming.md), [U1b](../ready/U1b-command-registry-and-palette.md).
- Gallery pages and screenshots → [U6f](../done/U6f-gallery-and-visual-tests.md).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Re-read ADR 0024's Escape order, and the APG dialog, menu button and tooltip patterns (primary W3C pages); note any gap with the primitives library's components and how this PR closes it.
- [x] Add the z-order tokens to `@laminapdf/tokens` (JSON, regenerate, tests), marked approved (the owner approved the table on 2026-11-05).
- [x] `src/overlays/layer-stack.ts`: a small stack (push/pop, `isTopmost`, Escape routing, focus restore targets) with unit tests.
- [x] `Dialog` on the primitives library's modal (React Aria `Modal`/`Dialog` recommended) styled per §7.18; `inert` on the app root; tests for trap, return focus, Escape, required-question mode, scrim click, nested menu.
- [x] `Popover`, `Menu`, `MenuButton`, `Tooltip` on the primitives library; tests for the APG keyboard model and typeahead.
- [x] `ToastRegion` and `toast()`; tests for stacking, timeout pause on hover and focus, the action's keyboard reach, announcements via the live region.
- [x] README section: which overlay to use when (dialog vs popover vs notice vs toast), the layer order and Escape order.

## Acceptance criteria

- [x] Dialog: focus is trapped, Escape closes, focus returns to the opener; `aria-modal`, labelled title; background is `inert` (tests in three engines).
- [x] Menus follow the APG menu button model (tests list each key: arrows, Home/End, typeahead, Escape returning focus).
- [x] Toasts stack (max three visible), pause on hover/focus, and are announced politely; errors are never only in a toast (documented in the API; the `toast()` type has no `error` variant).
- [x] Every overlay uses a z-order token; a test opens a dialog, a menu inside it and a toast, and asserts their computed stacking order and that Escape closes the innermost first.
- [x] axe reports zero violations for each overlay open, in light, dark and high contrast.
- [x] Any deviation from the design language's §7.18 and §7.20 is recorded there.
- [x] `cargo xtask check` passes.

## Post-merge checks

None.

## Tests

`packages/ui/src/overlays/**/*.test.tsx` (Vitest browser mode, Chromium, Firefox, WebKit) with the shared state and axe helpers from [U6d](./U6d-core-components.md); unit tests for the layer stack.

## Docs to update

- `packages/ui/README.md` (overlays section).
- [Design language](../../design/paper/design-language.md): §4 z-order table no longer **(proposed)** (approved 2026-11-05), and links to the tokens; §7.18 and §7.20 link their components and record deviations.
- [UI/UX plan](../../plans/ui-ux.md) §9: nothing to tick (U1b and U2a own the palette and selection to-dos).

## Risks and open questions

- **Order with C4a**: this todo should land before [C4a](./C4a-ui-chrome-and-platform.md) so its password dialog and menus use these overlays (see [U6d](./U6d-core-components.md)'s risks); C4a's **Depends on** gains U6e.
- **`inert` and the desktop webview**: `inert` is supported in current WebKit, Blink and Gecko; confirm in the Tauri webviews (WKWebView, WebView2, WebKitGTK) when [D2a](../ready/D2a-adr-0008-and-desktop-scaffold.md) lands.
- **Toast action reachability** without moving focus is awkward for keyboard users; if no clean pattern exists, actions in toasts are limited to ones that are also available elsewhere (e.g. Undo in the Edit menu and `Mod Z`).

## Outcome

- `@laminapdf/tokens` holds the z-order as `--paper-z-*` and `Z_ORDER` (page 0, page overlay 1, chrome 10, floating toolbar 20, sheet 30, modal 40, popover 50, toast 60), plus the dialog widths (440, 560, 820), the menu width and item height, the tooltip offset and delay, and the toast region's offset. The owner approved the z-order table, the wide dialog, the menu and the tooltip on 2026-11-05.
- `@laminapdf/ui` gains `Dialog`, `Popover` with `PopoverTrigger`, `Menu` with `MenuTrigger`, `Tooltip`, and `ToastRegion` with `ToastQueue`, all on React Aria Components. `src/overlays/layer-stack.ts` keeps the open layers in the order they were opened from (a menu above its dialog, whichever registers first); `LayerScope` puts each overlay on it, and a document listener closes the innermost layer on an Escape no overlay handled.
- Decisions, recorded in the design language: a dialog starts on its first field (a checked radio counts for its group) rather than on itself; its close button is last in the tab order; `aria-modal` is set on the element, since React Aria drops the prop. Menus and popovers draw at z 50 rather than the menu spec's 30. Disabled menu items are skipped by the arrows and show their reason as a second line. The popover is modal while open, as React Aria's popovers are. At most three toasts show, and a fourth pushes the oldest out.
- React Aria writes an inline z-index of 100000 on popovers and tooltips; each overlay sets its own inline z-index from the token, which React Aria spreads after its own.
- The toast region is not a toast library: no error variant, and its action must also live in a menu or a shortcut, since F6 region cycling is [C7a](../ready/C7a-accessibility-and-theming.md)'s.
- Bottom sheets, the phone dialog as a sheet, anchored positioning with the selection toolbar shell and the palette shell moved to [U6i](./U6i-sheets-positioning-and-palette-shell.md) when this todo was claimed.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
