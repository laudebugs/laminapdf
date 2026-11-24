# D9a — Desktop: theme, accent, page appearance and page layout

| Field | Value |
| --- | --- |
| ID | `D9a` |
| Status | `ready` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | Follows [ADR 0008](../../decisions/0008-desktop-shell.md) (the shell) and [ADR 0014 §11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build) (the store build) |
| Branch | `feat/d9a-desktop-view-settings` |
| PR title | `feat(desktop): theme, accent, page appearance and page layout` |
| Size | L (400–800: the settings state and persistence, the Settings section, the View popover and the menu wiring; split page layout into its own PR if the viewer needs work) |
| PR | — |

## Goal

Let a person choose how the desktop reader looks: the theme (Light, Dark, High contrast, Match the system), the accent colour (Teal, Blue, Plum, Green), the page appearance (Original, Sepia, Dark, High contrast) and the page layout (Single, Two pages, Continuous). Today the View menu's Theme, Accent, Page appearance and Page layout items are greyed out in the desktop, and the Settings dialog has no place for them. The owner asked for the theme first.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment: the items are disabled because the webview registers no handler for these `delegated()` commands, and each one is a feature, not a wiring change.
- `packages/ui/src/commands/reader-commands.ts`: `view.theme`, `view.accent`, `view.page-layout` and `view.page-appearance` are `delegated()`; `menuState()` in `apps/desktop/src/menus/use-native-menus.tsx` enables a delegated command only when `ctx.actions[id]` exists.
- The web reader's version: `useSettings` in `apps/web/src/settings.tsx` (appearance state in localStorage, `applyAppearance`, the Appearance section, the View settings popover); `apps/web/src/page-appearance.tsx` and `page-appearance-choices.ts` (the page choices, kept per document and as a default). `AppearanceSettings`, `ViewSettingsButton`, `parseAppearance`, `applyAppearance` and `SettingsRegistry` already live in `@laminapdf/ui`; the desktop's `apps/desktop/src/settings.tsx` already uses the same registry.
- The engine already takes a page appearance: the tile request carries `appearance` (`apps/desktop/src/engine/desktop-engine-client.ts`, `crates/laminapdf-desktop-protocol/src/msg.rs`), so Sepia and Dark pages need no engine work.
- `apps/desktop/src-tauri/src/prefs.rs`: the desktop keeps its preferences in `preferences.json` in the main process.
- [Maintainability, one home per concept](../../guidelines/maintainability.md): the choices, their parsing and their application stay in `@laminapdf/ui`; the web and the desktop supply only storage.
- The Mac App Store build (0.0.205, under review) lists none of this, so it ships as an update to both builds.

## Scope

**In scope**

- Move what the web's `settings.tsx` and `page-appearance.tsx` hold beyond storage into `@laminapdf/ui` (or a shared hook there), so the desktop calls it rather than copies it. The web reader keeps its behaviour and its tests.
- Persistence in the desktop: decide in the work whether it goes in `preferences.json` through the main process (so every window and the native menu agree, and a new window starts right) or in the webview's localStorage per window; record the choice in the PR. Windows open at the same time follow one another.
- A Settings section for appearance beside Files and the motion choice, and the View settings popover if the desktop has a top-bar slot for it.
- `view.theme`, `view.accent`, `view.page-appearance` and `view.page-layout` handlers in the desktop's `useCommands`, so the native View menu items are enabled and ticked to the current choice (`menu_state` carries the checked state; check how `Context` in `menu.rs` reports it).
- Page layout in the desktop viewer (Single, Two pages, Continuous), if `@laminapdf/viewer` already supports it for the web; if the desktop's viewer needs engine or layout work, split that out into its own PR and say so here.
- The same in the Mac App Store build.

**Out of scope**

- Reader mode's own text settings: [D9e](./D9e-desktop-reader-mode.md).
- New themes or accents the web lacks.

## Tasks

- [ ] Fresh worktree; claim in the work PR.
- [ ] Find the pieces of `apps/web/src/settings.tsx` and `page-appearance.tsx` that are not storage and move them to `@laminapdf/ui`; the web uses them from there.
- [ ] Add the desktop's storage for the appearance and the page choices (decision above), and apply the theme to the document and to the native chrome where the window has any (title bar on Windows and Linux; check macOS).
- [ ] Add the Appearance section and register the four command handlers in `apps/desktop/src/app.tsx`.
- [ ] Make `menu_state` carry the checked item for the four choice groups; update `menu.rs` if it cannot yet.
- [ ] Update `use-native-menus.test.ts` and add a test for each command (below).
- [ ] Try it in the built app on each OS; try it in a browser from the user's side (CLAUDE.md).
- [ ] Docs below.

## Acceptance criteria

- [ ] In the real app on macOS, Windows and Linux, View ▸ Theme, Accent, Page appearance and Page layout are enabled with a document open, tick the current choice, and change what the window shows.
- [ ] The choice survives quitting and reopening, and a second window and a new tab use it.
- [ ] Match the system follows the OS when it switches between light and dark while the app runs.
- [ ] The Settings dialog shows the choices, and the same choices in the View popover or menu stay in step.
- [ ] A test in the style of `apps/desktop/src/menus/use-native-menus.test.ts` shows each of the four commands available (state `enabled`) once the desktop's actions exist, and that a menu event runs the handler with its argument.
- [ ] A browser check from the user's side: drive the built desktop frontend, change each choice with mouse and keyboard, and judge what a person sees (contrast of text on the page in High contrast, the page's colours in Sepia and Dark). Record what was seen.
- [ ] The web reader's own tests for these choices still pass without change to their expectations.
- [ ] The Mac App Store build has the same behaviour (checked with the store build's frontend, as it is the same code).

## Post-merge checks

- [ ] None.

## Tests

- Unit, in `apps/desktop/src/menus/use-native-menus.test.ts`: `menuState` enables the four commands and marks the current choice; `runMenuCommand` passes the argument.
- Unit for the storage (parse, defaults, a bad stored value falls back).
- `packages/ui/test/appearance.test.ts` and the browser settings test keep covering the shared part.
- A desktop e2e scenario (`apps/desktop/e2e/specs/`) that picks Dark and checks the document's theme attribute after a restart, if the e2e harness can restart the app.

## Docs to update

- `docs/plans/desktop-app.md`: tick the to-do under D9.
- `docs/capabilities/README.md`: no row covers reader view settings today, so no desktop cell changes; confirm when the work starts.
- `apps/desktop/CHANGELOG.md`; the download page's feature copy if it lists the settings.

## Risks and open questions

- **One home.** If moving the web's code into `@laminapdf/ui` grows the PR, split it: a first PR moves it with no desktop change.
- **Native chrome.** The title bar may not follow the theme on every OS; list what does and does not.
- **Page layout.** Two pages and Continuous may need viewer work the web has and the desktop lacks.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
