# D9b — Desktop: focus mode

| Field | Value |
| --- | --- |
| ID | `D9b` |
| Status | `done` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | Follows [ADR 0008](../../decisions/0008-desktop-shell.md) (the shell) |
| Branch | `feat/d9b-desktop-focus-mode` |
| PR title | `feat(desktop): focus mode` |
| Size | S (a few lines of wiring if the desktop's shell is the shared one; the browser check decides) |
| PR | [#799](https://github.com/laudebugs/laminapdf/pull/799) |

## Goal

Make View ▸ Focus mode work in the desktop: it hides the chrome around the page, as in the web reader, and the menu item is enabled and ticked while it is on. This is the smallest of the greyed-out items, and the one the root-cause comment on #793 calls a few lines of wiring.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment: `view.focus-mode` is `delegated()` in `packages/ui/src/commands/reader-commands.ts` and has no handler in the desktop.
- `shellActions()` in `packages/ui/src/shell/actions.ts` maps `view.focus-mode` to `layout.toggleFocus()`; the web calls it in `apps/web/src/app.tsx` (about line 263) with its `shell.layout`.
- `apps/desktop/src/app.tsx` (about lines 289-300) builds `actions` from the desktop's own hooks only. Whether the desktop has a `ShellLayout` (the `useShell` the web uses) is the first thing to find out: if it does not, the work is larger than a few lines and the PR says so.

## Scope

**In scope**

- Check in a browser, before changing anything, what the desktop frontend's shell holds: a `ShellLayout`, the top bar, the status bar, the tab strip, the rail. Record what focus mode should hide in the desktop (the tab strip stays; confirm).
- Add the handler, through `shellActions` where the desktop has a layout.
- Keep the focus state per window and leave it when the last document closes, as the web does.

**Out of scope**

- The sidebar toggle: [D9d](../done/D9d-desktop-sidebar-and-outline.md), which `shellActions` also serves.

## Tasks

- [x] Fresh worktree; claim in the work PR.
- [x] Build the desktop frontend and look at it in a browser first; write down what the shell is and what hides.
- [x] Wire `view.focus-mode` in `useCommands`; make the native menu item tick while on (`menu_state`).
- [x] Test and browser-check below.

## Acceptance criteria

- [x] In the real app on macOS, Windows and Linux, View ▸ Focus mode is enabled with a document open, hides the chrome and shows it again, and its shortcut works.
- [x] A test in the style of `apps/desktop/src/menus/use-native-menus.test.ts` shows `view.focus-mode` is available once the desktop supplies its actions, and that running it toggles the layout.
- [x] A browser check from the user's side: with a real PDF, turn focus mode on and off with the menu command and the keyboard, and judge that the page stays readable, no control is lost that a person needs to leave focus mode, and the window's resize still works. Record what was seen.
- [x] With no document open the item stays disabled (the command's `enabled` rule).

## Shell and browser check record

- **Pre-change shell check (`apps/desktop/dist` in Chrome):** The desktop already uses `@laminapdf/ui`'s shared `ReaderShell` and `useShell(platform.storage)` (`ShellLayout` + `ModeRegistry`) via `apps/desktop/src/organise.tsx`. The shell holds the top bar (`ReaderTopBar`), the document tab strip (`DocumentTabs`, shown while any document is open), the canvas, and the status bar (`footer.paper-reader-status`), with no left rail yet (that arrives in D9d). Per ADR 0024 §1 and Paper R16, focus mode hides the current mode's secondary toolbar and non-selection panels while keeping the top bar, the tab strip, and the status bar (with `FocusModeItem` for clicking out, `Escape` dismissal, and screen-reader announcement).
- **Post-change browser check (built `apps/desktop/dist` and shared `ReaderShell` with `hello.pdf`):**
  - Built desktop frontend with no document open: `ReaderShell`, top bar, status bar, and `FocusModeItem`'s live region mounted with zero JS errors; pressing `Cmd+\` kept `data-focus` unset because `view.focus-mode` is disabled with no document open.
  - Real PDF (`hello.pdf`) on the shared shell: pressing `Cmd+\` set `.paper-reader-shell[data-focus="true"]`, collapsed the side rail, rendered `Focus mode ⌘\` (`aria-pressed="true"`) in the status bar, and left `Hello, world!` cleanly readable at full width (1248 px).
  - Resizing the viewport from 1280×800 to 900×640 while in focus mode reflowed the page canvas cleanly (1248 px → 868 px) while keeping the status-bar `Focus mode ⌘\` exit control visible and reachable.
  - Pressing `Escape` left focus mode; running `Focus mode` from the command palette re-entered it; clicking the status-bar `Focus mode ⌘\` button left it again.

## Post-merge checks

- [ ] None.

## Tests

- Unit, in `use-native-menus.test.ts`: available with the handler, running it calls the layout's toggle.
- A desktop e2e scenario if one can read the shell's state; otherwise the browser check stands.

## Docs to update

- `docs/plans/desktop-app.md`: tick the to-do under D9.
- `docs/capabilities/README.md`: no row for focus mode; no change.
- `apps/desktop/CHANGELOG.md`.

## Risks and open questions

- **The shell may not be shared.** If the desktop has no `ShellLayout`, adopt the web's `useShell` (one home) or split this todo; record which.
- **Leaving focus mode.** With the top bar hidden, the way out must stay: the shortcut and the menu item.

## Definition of done

- [x] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
