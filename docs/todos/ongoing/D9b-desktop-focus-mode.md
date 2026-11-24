# D9b — Desktop: focus mode

| Field | Value |
| --- | --- |
| ID | `D9b` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | Follows [ADR 0008](../../decisions/0008-desktop-shell.md) (the shell) |
| Branch | `feat/d9b-desktop-focus-mode` |
| PR title | `feat(desktop): focus mode` |
| Size | S (a few lines of wiring if the desktop's shell is the shared one; the browser check decides) |
| PR | — |

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

- The sidebar toggle: [D9d](../ready/D9d-desktop-sidebar-and-outline.md), which `shellActions` also serves.

## Tasks

- [ ] Fresh worktree; claim in the work PR.
- [ ] Build the desktop frontend and look at it in a browser first; write down what the shell is and what hides.
- [ ] Wire `view.focus-mode` in `useCommands`; make the native menu item tick while on (`menu_state`).
- [ ] Test and browser-check below.

## Acceptance criteria

- [ ] In the real app on macOS, Windows and Linux, View ▸ Focus mode is enabled with a document open, hides the chrome and shows it again, and its shortcut works.
- [ ] A test in the style of `apps/desktop/src/menus/use-native-menus.test.ts` shows `view.focus-mode` is available once the desktop supplies its actions, and that running it toggles the layout.
- [ ] A browser check from the user's side: with a real PDF, turn focus mode on and off with the menu command and the keyboard, and judge that the page stays readable, no control is lost that a person needs to leave focus mode, and the window's resize still works. Record what was seen.
- [ ] With no document open the item stays disabled (the command's `enabled` rule).

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

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
