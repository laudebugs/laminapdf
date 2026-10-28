# D4b — Native menus and accelerators from the shared shortcut map

| Field | Value |
| --- | --- |
| ID | `D4b` |
| Status | `ready` |
| Plan | [Desktop app §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C7a](./C7a-accessibility-and-theming.md), [D4a](./D4a-windows-tabs-and-session-restore.md) |
| Blocks | [D4c](./D4c-native-printing.md), [D5b](./D5b-updater-in-app.md) |
| Dependency notes | [D4a](D4a-windows-tabs-and-session-restore.md). Cross-plan: the shared shortcut map is a web **C7** to-do ("Keyboard shortcuts map shared with desktop", [web-app §C7](../../plans/web-app.md#c7-accessibility-offline-and-launch)). If C7 has not merged, this PR creates the map in `@lamina/ui` and C7 extends it — the parent board must pick one owner (see Risks). |
| ADRs | Follows ADR 0010 (Platform adapter), ADR 0008 |
| Branch | `feat/d4b-native-menus` |
| PR title | `feat(desktop): generate native menus and accelerators from the shared shortcut map` |
| Size | M–L (500–900 lines) |
| PR | — |

## Goal

Give lamina proper native menus on every OS — the macOS app menu and menu bar, and a window menu bar on Windows and Linux — generated from **one shortcut map** shared with the web reader, so a shortcut can never mean different things in the menu, the web app and the help dialog. Requirement 6 of the [desktop plan](../../plans/desktop-app.md#functional): File (Open, Open Recent, Close, Print, Export copy), Edit (Copy, Find), View (zoom, sidebars, page layout), Window, Help, all with native accelerators. D4's acceptance criterion "menu items and shortcuts match the shared map (test)" is met here.

## Context

- [Desktop plan §6 D4](../../plans/desktop-app.md#d4-windows-tabs-menus-and-printing) and §9 D4.
- Web plan C7: shared keyboard map and help dialog.
- Tauri menus: `tauri::menu::{MenuBuilder, SubmenuBuilder, MenuItemBuilder, CheckMenuItemBuilder, PredefinedMenuItem, AboutMetadata}`, `app.set_menu(menu)` (app-wide; macOS) vs `window.set_menu(menu)` (per window; Windows/Linux), `app.on_menu_event(|app, event| …)` with `event.id()`, accelerator strings (`"CmdOrCtrl+O"`, `"CmdOrCtrl+Shift+T"`), `MenuItem::set_enabled`, `set_text`, `CheckMenuItem::set_checked`.
- macOS HIG: app menu (About, Settings… `Cmd+,`, Services, Hide, Hide Others, Show All, Quit), Window menu (Minimize, Zoom, Bring All to Front, window list), Help menu with search field (system-provided when titled "Help").
- D3b `RecentFiles` (Open Recent submenu); D4a `WindowRegistry` (Window menu, Move Tab to Window).

## Scope

**In scope**

- Shared shortcut map in `@lamina/ui` (`packages/ui/src/shortcuts/map.ts`, if not already provided by C7): `ShortcutId` union, per-command `{ id, label, menu: 'file'|'edit'|'view'|'window'|'help'|null, mac: 'Cmd+O', other: 'Ctrl+O', webAllowed: boolean }`; exported as JSON at build time (`shortcuts.json`) so Rust reads the **same data** (embedded with `include_str!` from a generated file checked by CI, or passed from the frontend at startup — prefer build-time embedding to avoid a startup round trip).
- `src-tauri/src/menu.rs`: `fn build_menu(app: &AppHandle, ctx: &MenuContext) -> tauri::Result<Menu<Wry>>` producing:
  - macOS app menu: About Lamina (`PredefinedMenuItem::about` with `AboutMetadata`: version, copyright, licences link), Check for Updates… (enabled in D5b), Settings… (`Cmd+,`), Services, Hide/Hide Others/Show All, Quit.
  - File: Open… (`CmdOrCtrl+O`), Open Recent ▸ (from `RecentFiles`, "Clear Menu"), New Window (`CmdOrCtrl+Shift+N`), Close Tab (`CmdOrCtrl+W`), Close Window (`CmdOrCtrl+Shift+W`), Print… (`CmdOrCtrl+P`, enabled in D4c), Export Copy… (save the original bytes elsewhere — byte-identical copy via Rust `std::fs::copy` after a Rust-side save dialog), Quit on Windows/Linux (`Ctrl+Q`).
  - Edit: Copy, Select All (predefined, so text fields work), Find (`CmdOrCtrl+F`), Find Next/Previous.
  - View: Zoom In/Out/Actual Size/Fit Width/Fit Page, Sidebars toggles (check items), Page Layout ▸ (single, continuous, two-page — radio-like check items), Enter Full Screen (predefined on macOS).
  - Window: Minimize, Zoom (macOS), next/previous tab, Move Tab to New Window, Move Tab to Window ▸, window list.
  - Help: Lamina Help (opens the docs URL via `tauri-plugin-opener` from Rust), Keyboard Shortcuts (opens the shared help dialog), Report an Issue, View Licences (third-party notices from D6b).
- Menu events → `emit_to(focused window, "menu://command", ShortcutId)`; the UI dispatches through the same command registry used for keyboard shortcuts (no duplicate handlers).
- **Accelerator ownership**: when a menu accelerator exists, the webview's keydown handler for that shortcut is disabled on desktop (otherwise both fire on Windows/Linux); enforced by a `Platform.nativeMenuShortcuts` set.
- Dynamic state: enable/disable items per focused window (no document → Print/Close Tab disabled; zoom limits), check states for sidebars/layout; updated via a `menu_state(state)` command from the UI on focus/state change.
- Per-OS placement: one app menu on macOS (`app.set_menu`), per-window menu bar on Windows/Linux created with each window (D4a's builder), Linux GTK menu bar visible by default.
- Localisation hooks: labels come from the map (English now), structured for later i18n.

**Out of scope** (and which todo covers it)

- Printing behaviour: [D4c](D4c-native-printing.md) (this PR adds the item and event).
- "Check for Updates…" behaviour: [D5b](D5b-updater-in-app.md).
- Settings window content: [D5b](D5b-updater-in-app.md) / [D7b](D7b-settings-renderer-switch-and-download-page.md) (this PR opens an empty settings route).
- Context menus in the document view (later, shared with the editor).
- Tray icon (not planned).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2` and `rust-best-practices`.
- [ ] Create or extend the shared shortcut map + JSON export (`pnpm --filter @lamina/ui build:shortcuts`), with a CI check that the generated JSON is up to date.
- [ ] `menu.rs` builder + `on_menu_event` dispatch; `MenuContext` (recent files, windows, focus state); rebuild Open Recent and Window submenus on change.
- [ ] `menu_state` command (manifest + capability) and the UI side that reports state.
- [ ] Export Copy: Rust-side save dialog (`DialogExt::file().save_file`) and byte copy of the original file; never via the webview.
- [ ] Disable duplicate webview handlers for native accelerators.
- [ ] Settings route placeholder (`settings` window label, created on demand, own capability with only the commands it needs — D5b fills it).
- [ ] Tests (below); manual checklist per OS with screenshots of each top-level menu.

## Acceptance criteria

- [ ] A Rust test proves every menu item with an accelerator corresponds to a `ShortcutId` in the shared map and uses the map's accelerator for the current OS; and every map entry with `menu != null` appears in the menu.
- [ ] A TS test proves the help dialog lists exactly the map's entries.
- [ ] Each accelerator fires once (not twice) on Windows and Linux (e2e: press `Ctrl+=` and assert one zoom step).
- [ ] Open Recent lists the D3b entries and "Clear Menu" clears them; Window menu lists open windows; Move Tab to Window works from the menu (e2e).
- [ ] macOS app menu follows the HIG layout (manual screenshot); Windows/Linux show a menu bar per window.
- [ ] Items are disabled when not applicable (no document → Print, Close Tab disabled) (e2e via `menu_state` assertions or a test-only `menu_snapshot` command under `e2e`).

## Tests

- Rust unit tests: `build_menu` structure snapshot per OS (`insta` snapshot of a serialised menu tree; `#[cfg(target_os)]` variants), map ↔ menu consistency test.
- Vitest: command registry dispatch for `menu://command` events; help dialog consistency.
- WebdriverIO: accelerator single-fire, Open Recent, Window menu actions (menu clicks via test-only `e2e_menu_click(id)` since native menus can't be clicked through WebDriver).

## Docs to update

- `docs/plans/desktop-app.md` §9 D4: tick "Native menus and accelerators generated from the shared shortcut map".
- Web plan C7 to-do (shared map) — **do not edit the web plan here**; note in the PR description that the map exists so the C7 owner links it.
- `apps/desktop/README.md`: menu architecture.

## Risks and open questions

- **Shortcut map ownership** between C7 and D4b: whichever merges first creates `packages/ui/src/shortcuts/`; the parent board should record this cross-plan link.
- **Linux global menus** (Ubuntu Unity/KDE appmenu) may hide per-window menus; acceptable.
- **Accelerator conflicts** with webview defaults (e.g. `Ctrl+P` triggering webview print) — intercept in Rust/menu first; verify per OS.
- **Keyboard layouts** (non-US: `Cmd+=` vs `Cmd+Shift+=`): test with German/French layouts manually; record.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
