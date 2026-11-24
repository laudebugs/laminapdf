# Resolution: the Mac App Store build offered "Check for updates…"

[Diagnosis](detail.md).

## The fix

One constant says what the store build is: `update::policy::APP_STORE` (`cfg!(feature = "app-store")`), which `Install::current` already used.

- **Native menus.** `menu_bar` takes `Context::updates` (`!APP_STORE` for the running build) and drops the `UPDATE_COMMANDS` (`app.check-for-updates`, `app.restart-to-update`) when it is false. `tidy` removes the separator the item leaves. `menus.json` and its generator are unchanged, so the direct build's menus are the same.
- **Palette.** `updateActions` (`src/updates.tsx`) gives `app.check-for-updates` and `app.restart-to-update` handlers only when the main process's update status is known and does not say `off: "app-store"`. A delegated command with no handler is hidden by the registry, so the palette and any other view of it leave it out. The webview learns the build from the status it already receives, not from a second constant.
- **Settings › Updates** is left as it was: in the store build it is only the note that the store updates the app.

## How it was checked

- Rust, `menu::tests`, run with and without `--features app-store`: the direct bar lists the check on macOS and on Windows and Linux; the store bar lists no update command on either; no stray separator is left; and the running build lists them unless it is the store build. With the filter removed, the two store tests fail.
- Vitest, `src/updates.test.ts`: in the store build no update action exists, even with an update ready, and Settings › Updates has no button, field or switch; the direct build has both actions and Check now. With the guard removed the first test fails.

## Not checked

The native menu bar and the real store build: this session had no Mac window to look at, and Settings and the palette only run inside the Tauri shell (without it nothing updates, so a browser shows neither). The checks read the menu model and render the settings to HTML.

## Left for later

The updater's code is still linked in the store build and unreachable. Compiling it out is a separate follow-up (runbook §8.3).
