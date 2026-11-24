# Resolution: the Mac App Store build offered "Check for updates…"

[Diagnosis](detail.md).

## The fix

One constant says what the store build is: `update::policy::APP_STORE` (`cfg!(feature = "app-store")`), which `Install::current` already used.

- **Native menus.** `menu_bar` takes `Context::updates` (`!APP_STORE`, set in `build_context`, which the live menu bar uses) and drops `app.check-for-updates` from both menus when it is false. The filter also covers `app.restart-to-update`, which no menu lists. `tidy` is unchanged, and dropping the item leaves no separator over. `menus.json` and its generator are unchanged, so the direct build's menus are the same.
- **Palette.** `updateActions` (`src/updates.tsx`) gives `app.check-for-updates` and `app.restart-to-update` handlers only when the main process's update status is known and does not say `off: "app-store"`. The registry hides a delegated command that has no handler (`availability` in `packages/ui/src/commands/registry.ts`; its test is "hides a delegated command until the shell gives its handler" in `packages/ui/test/commands.test.ts`). The webview learns the build from the status it already receives, not from a second constant.
- **Restart to update.** `update_restart_to_install` now refuses in the store build with `UpdatesManaged`, as its sibling commands do.
- **Settings › Updates** is left as it was: in the store build it is only the note that the store updates the app.

## Effects in the direct build

- The palette offers Check for updates only after `update_status` answers. If that call is rejected, `useDesktopUpdates` only logs it, so the command stays hidden for that session. Before this fix it showed at once.
- In the native menu the item stays listed. `use-native-menus.tsx` reports a command with no handler as not available, so the menu shows it disabled until the status arrives, and the same if the call is rejected.

## How it was checked

- Rust, `menu::tests`, with and without `--features app-store`: the direct bar lists the check on macOS and on Windows and Linux; the store bar lists no update command on either; the macOS app menu runs from About to a separator and Settings, and the Windows and Linux Help menu ends with licences, a separator and About (so no separator is left over), and the direct Help menu keeps its separator, check and About; `build_context` (what `current` uses) lists the check on both platforms unless `APP_STORE`. `update::commands`' test: restart to install is refused for the store install only. A unit test of `tidy` (leading, trailing and doubled separators) is what guards `tidy`. The "no update command" test fails with the filter removed.
- Vitest, `src/updates.test.ts`: in the store build no update action exists, even with an update ready; the direct build has both actions; nothing is offered outside the shell or while the status is unknown. With the store guard removed, the first fails.

## Not checked

No Mac window was available; the checks read the menu model and render Settings to HTML. Settings and the palette run only inside the Tauri shell, so a browser shows neither.
