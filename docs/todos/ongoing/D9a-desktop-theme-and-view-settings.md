# D9a — Desktop: theme, accent colour and page appearance

| Field | Value |
| --- | --- |
| ID | `D9a` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D9](../../plans/desktop-app.md#d9-reader-parity-with-the-web-app) |
| Round / milestone | Round D (desktop): reader parity, after M3 |
| Depends on | None |
| ADRs | Follows [ADR 0008](../../decisions/0008-desktop-shell.md) (the shell) and [ADR 0014 §11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build) (the store build) |
| Branch | `feat/d9a-desktop-view-settings` |
| PR title | `feat(desktop): theme, accent colour and page appearance` |
| Size | M (200–400: the shared settings code moves, the preferences gain three fields, and the Settings section and menu wiring follow) |
| PR | — |

## Goal

Let a person choose how the desktop reader looks: the theme (Light, Dark, High contrast, Match the system), the accent colour (Teal, Blue, Plum, Green) and the page appearance (Original, Sepia, Dark, High contrast). Today View ▸ Theme, Accent and Page appearance are greyed out in the desktop, and the Settings dialog has no place for them. The owner asked for the theme first. Page layout is not part of this todo: the web reader has no handler for it either, and [D9h](./D9h-viewer-two-page-and-continuous-layouts.md) covers it.

## Context

- [#793](https://github.com/laudebugs/laminapdf/issues/793) and its root-cause comment: the items are disabled because the webview registers no handler for these `delegated()` commands, and each one is a feature, not a wiring change.
- `packages/ui/src/commands/reader-commands.ts`: `view.theme`, `view.accent` and `view.page-appearance` are `delegated()`.
- `menuState()` in `apps/desktop/src/menus/use-native-menus.tsx` enables a delegated command only when `ctx.actions[id]` exists.
- The web reader's version: `useSettings` in `apps/web/src/settings.tsx` keeps the appearance in localStorage, applies it with `applyAppearance` and shows the Appearance section and the View settings popover.
- `apps/web/src/page-appearance.tsx` and `page-appearance-choices.ts` hold the page choices, kept per document and as a default.
- `AppearanceSettings`, `ViewSettingsButton`, `parseAppearance`, `applyAppearance` and `SettingsRegistry` already live in `@laminapdf/ui`, and the desktop's `apps/desktop/src/settings.tsx` already uses the same registry.
- The engine already takes a page appearance: the tile request carries `appearance` (`apps/desktop/src/engine/desktop-engine-client.ts`, `crates/laminapdf-desktop-protocol/src/msg.rs`). Sepia and Dark pages need no engine work.
- [Maintainability, one home per concept](../../guidelines/maintainability.md): the choices, their parsing and their application stay in `@laminapdf/ui`, and the web and the desktop supply only storage.
- The Mac App Store build (0.0.205, under review) lists none of this, so it ships as an update to both builds.

## Scope

**In scope**

- Move whatever the web's `settings.tsx` and `page-appearance.tsx` hold beyond storage into `@laminapdf/ui`, so the desktop calls it and does not copy it. The web reader keeps its behaviour and its tests.
- **Persistence, decided here:** the choices go in the desktop's `preferences.json`, through the path the other preferences use. A second window and a new tab must follow a change, and localStorage cannot show that in a test, since each webview keeps its own. The files:
  - `apps/desktop/src-tauri/src/prefs.rs`: `Preferences` gains `theme`, `accent` and `page_appearance` (enums or short strings with defaults under `#[serde(default)]`). The struct is `Copy` today, so fields that are strings become enums, or the derive changes and `get()`'s callers are checked.
  - `apps/desktop/src-tauri/src/commands.rs`: `set_preferences` already saves and sends `preferences://changed` to every window; no new command.
  - `apps/desktop/src/settings.tsx`: the `Preferences` type and `usePreferences`, which read and set them.
  - `apps/desktop/src/app.tsx`: the three handlers write through `set_preferences` and apply the value on `preferences://changed`.
- A Settings section for appearance beside Files, and the View settings popover if the desktop has a top-bar slot for it.
- `view.theme`, `view.accent` and `view.page-appearance` handlers in the desktop's `useCommands`, so the native View menu items are enabled and ticked to the current choice. Check how `Context` in `menu.rs` reports a checked item through `menu_state`.
- The same in the Mac App Store build.

**Out of scope**

- Page layout (Single, Two pages, Continuous): [D9h](./D9h-viewer-two-page-and-continuous-layouts.md). Its menu items stay disabled until that todo lands.
- Reader mode's own text settings: [D9e](./D9e-desktop-reader-mode.md).
- New themes or accents the web lacks.

## Tasks

- [ ] Fresh worktree; claim in the work PR.
- [ ] Find the pieces of `apps/web/src/settings.tsx` and `page-appearance.tsx` that are not storage and move them to `@laminapdf/ui`; the web uses them from there.
- [ ] Add the three preference fields in `prefs.rs`, the matching TypeScript type and defaults, and a Rust test that an old `preferences.json` without them loads with the defaults.
- [ ] Apply the theme to the document, and to the native chrome where the window has any (title bar on Windows and Linux; check macOS).
- [ ] Add the Appearance section and register the three handlers in `apps/desktop/src/app.tsx`; apply a change that arrives as `preferences://changed`.
- [ ] Make `menu_state` carry the checked item for the three choice groups; update `menu.rs` if it cannot yet.
- [ ] Update `use-native-menus.test.ts` and add the tests below.
- [ ] Try it in the built app on each OS; try it in a browser from the user's side (CLAUDE.md).
- [ ] Docs below.

## Acceptance criteria

- [ ] In the real app on macOS, Windows and Linux, View ▸ Theme, Accent and Page appearance are enabled with a document open, tick the current choice and change what the window shows.
- [ ] The choice survives quitting and reopening, and a second window and a new tab show it at once (a test changes it in one window and reads it in another).
- [ ] Match the system follows the OS when it switches between light and dark while the app runs.
- [ ] The Settings dialog shows the choices, and the same choices in the View popover or menu stay in step.
- [ ] A test in the style of `apps/desktop/src/menus/use-native-menus.test.ts` shows each of the three commands available (state `enabled`) once the desktop's actions exist, and that a menu event runs the handler with its argument.
- [ ] A browser check from the user's side: drive the built desktop frontend, change each choice with mouse and keyboard, and judge what a person sees (text contrast on the page in High contrast, the page's colours in Sepia and Dark). Record what was seen.
- [ ] The web reader's own tests for these choices pass without change to their expectations.
- [ ] The Mac App Store build behaves the same (it is the same frontend code; check its build's menu).
- [ ] View ▸ Page layout stays disabled, and the PR says it waits on D9h.

## Post-merge checks

- [ ] None.

## Tests

- Unit, in `apps/desktop/src/menus/use-native-menus.test.ts`: `menuState` enables the three commands and marks the current choice; `runMenuCommand` passes the argument.
- Rust, in `prefs.rs`: the new fields round-trip and an old file loads with defaults.
- `packages/ui/test/appearance.test.ts` and the browser settings test keep covering the shared part.
- A desktop e2e scenario (`apps/desktop/e2e/specs/`) that picks Dark in one window and checks a second window's theme, and again after a restart if the harness can restart the app.

## Docs to update

- `docs/plans/desktop-app.md`: tick the to-do under D9.
- `docs/capabilities/README.md`: checked, no row in sections A to J covers reader view settings, so no desktop cell changes.
- `apps/desktop/CHANGELOG.md`; the download page's feature copy if it lists the settings.

## Risks and open questions

- **One home.** If moving the web's code into `@laminapdf/ui` grows the PR, split it: a first PR moves it with no desktop change.
- **Native chrome.** The title bar may not follow the theme on every OS; list what does and does not.
- **Match the system.** The webview reports the OS scheme through `prefers-color-scheme`; confirm it does so in WKWebView, WebView2 and WebKitGTK.
- **Preferences struct.** Adding fields touches every place that builds a `Preferences`; the compiler finds them.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
