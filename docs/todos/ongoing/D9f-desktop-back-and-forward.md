# D9f — Desktop: Back and Forward through the reading history

| Field | Value |
| --- | --- |
| ID | `D9f` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | Follows [ADR 0008](../../decisions/0008-desktop-shell.md) |
| Branch | `feat/d9f-desktop-back-forward` |
| PR title | `feat(desktop): back and forward through the reading history` |
| Size | S (under 200 lines, if the viewer's history store works in the desktop as it stands) |
| PR | — |

## Goal

Make Go ▸ Back and Go ▸ Forward work in the desktop: after a person follows a link or an outline entry, jumps to a page or searches, Back returns to where they were and Forward goes on. Both items are disabled today.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment.
- `nav.back`, `nav.forward` and `nav.follow-link` in `packages/ui/src/commands/reader-commands.ts`.
- The web's `useHistory` in `apps/web/src/app.tsx` (about lines 195-225): the viewer keeps a `history` store (`packages/viewer/src/history`) with `back` and `forward` entries, and the web calls `window.history.back()` so the browser's own Back button and the app agree. The desktop's webview has no address bar and no browser Back button, and its tabs are the app's own (`apps/desktop/src/tabs`), so the desktop calls the viewer's history directly. Check whether the viewer exposes a method for that; if not, the viewer gets one, and the web keeps its `window.history` link.
- Each tab has its own viewer and so its own history; switching tabs must not mix them.

## Scope

**In scope**

- `nav.back` and `nav.forward` handlers in the desktop, enabled only when the open tab has an entry to go to (the menu follows the state).
- The viewer's history per tab, kept across a tab switch (tabs keep their viewer state; check) and cleared when the document reloads as another one.
- Mouse back and forward buttons and the Cmd+[ and Cmd+] shortcuts, if the command table gives them (`reader-commands.ts`).

**Out of scope**

- Following links (`nav.follow-link`) in the desktop: the engine serves no links yet (`pageLinks` returns none). This todo works with what moves the viewer: the outline once [D9d](../ready/D9d-desktop-sidebar-and-outline.md) lands, find results, page jumps and the page box.
- A history across sessions.

## Tasks

- [ ] Fresh worktree; claim in the work PR.
- [ ] Read `packages/viewer/src/history` and decide whether the viewer needs a direct `back()` and `forward()`.
- [ ] Register the handlers in `useCommands` with the enablement the history's state gives; add the state to `menu_state`.
- [ ] Tests, browser check, docs.

## Acceptance criteria

- [ ] In the real app on macOS, Windows and Linux, after a jump (for example the Go to page box, or an outline entry once D9d is in), Go ▸ Back returns to the earlier place and Go ▸ Forward returns again; both are disabled when there is nowhere to go.
- [ ] Each tab keeps its own history, and a tab's history survives switching away and back.
- [ ] A test in the style of `use-native-menus.test.ts` shows `nav.back` and `nav.forward` are available only when the history has an entry on that side.
- [ ] A browser check from the user's side: jump through three pages, go back twice and forward once, and judge that the page and the scroll position match each time. Record what was seen.

## Post-merge checks

- [ ] None.

## Tests

- Unit: the handlers call the viewer's history; availability follows its state; two tabs do not share one.
- `use-native-menus.test.ts` as above.
- A desktop e2e scenario if the harness can jump pages.

## Docs to update

- `docs/plans/desktop-app.md`: tick the to-do under D9.
- `docs/capabilities/README.md`: no row for navigation history; no change.
- `apps/desktop/CHANGELOG.md`.

## Risks and open questions

- **Little to go back to at first.** With no links served, a history entry comes from jumps; the item is useful mainly once D9d and the engine's links arrive. The PR says so in the changelog.
- **Web and desktop paths.** The web's use of `window.history` must keep working; the new viewer method must not change it.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
