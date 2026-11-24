# The Mac App Store build offered "Check for updates…"

[Resolution](resolution.md).

## What we saw

The Mac App Store build's menus listed "Check for updates…": in the app menu on macOS, and in Help on Windows and Linux, where the store build does not run. The command palette listed "Check for updates" too. Choosing either opened Settings › Updates and sent a check that did nothing.

The store build had already turned its updater off: `Install::AppStore` gives `Off::AppStore`, `update::start` returns at once, the plugin is not registered, and `plugins.updater` is null in `tauri.appstore.conf.json`.

Apple's App Review Guideline 2.4.5(vii) says Mac App Store apps must not offer another way to update, so the item was a likely reason for rejection.

## How to reproduce

Build with `--features app-store` and open the app menu, or open the palette and type "update". Before the fix, `menu_bar` over the embedded `menus.json` lists `<cmd:app.check-for-updates>` whatever the build.

## Cause

`menus.json` is generated once, from `apps/desktop/src/menus/layout.ts`, for every build, and the main process built the native menus from all of it. The webview offered the `app.check-for-updates` action whenever it ran in the shell, whatever the install.

## Where an update was offered

- `menus.json`: "Check for updates…" in the macOS app menu and in the Windows and Linux Help menu (`app.check-for-updates`, twice).
- The command palette: `app.check-for-updates`, from the handler the webview gave it, and `app.restart-to-update` only while an update waits, never in the store build.
- Settings › Updates: already only a note in the store build, "Updates come from the App Store", with no button, channel or switch. Settings › About shows the version and channel, with no check.
- Checked and absent: a tray or dock menu, a shortcut for the command (it has none), an "update available" banner (the status bar's "Update ready" needs a downloaded update, which this build never has).
- The main process's `update_restart_to_install` command had no store guard, unlike `update_reset_install_id`; nothing in the store build could reach it, since no update is ever ready.
