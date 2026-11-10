# @laminapdf/desktop

The desktop reader: `@laminapdf/ui`'s reader chrome in a Tauri 2 window on macOS, Windows and Linux ([ADR 0008](../../docs/decisions/0008-desktop-shell.md)). This first build opens one window with the reader's top bar and home screen. Opening a document arrives with the engine process and the tile transport (todos [D2b](../../docs/todos/ready/D2b-engine-process.md) and [D2d](../../docs/todos/ready/D2d-tile-transport-and-first-document.md)); until then, choosing a file shows a notice that says so.

## Run it

Linux needs WebKitGTK and D-Bus to build the app, and so to build the Cargo workspace, which includes it (`cargo xtask doctor` checks for them):

```sh
sudo apt-get install libwebkit2gtk-4.1-dev libdbus-1-dev pkg-config
```

macOS and Windows need nothing beyond the repository's toolchain: the app uses WKWebView and WebView2, which the OS provides. Without a GPU driver on Linux, set `WEBKIT_DISABLE_DMABUF_RENDERER=1` (ADR 0008 §7).

The frontend imports the built UI packages, so build them once, then run the app:

```sh
pnpm --filter "@laminapdf/desktop^..." run build      # @laminapdf/tokens, fonts, icons and ui
pnpm --filter @laminapdf/desktop tauri dev            # Vite on port 1420 and the app, with hot reload
pnpm --filter @laminapdf/desktop tauri build --debug --no-bundle   # what CI builds: target/debug/laminapdf-desktop
pnpm --filter @laminapdf/desktop test                 # the frontend's tests (Node, Tauri's IPC mocked)
cargo test -p laminapdf-desktop                       # the shell's tests and the capability guard
cargo xtask desktop-notices                           # target/desktop/THIRD_PARTY_NOTICES.md
```

## How it fits together

- **`src/main.tsx`** loads the Paper tokens, fonts and component styles, makes one desktop platform, and renders the app. It is the only module that asks whether it runs inside Tauri (`isTauri()`); `cargo xtask check ts` refuses the call anywhere else.
- **`src/platform/desktop.ts`** is the desktop `Platform` (ADR 0010). It starts from the web platform, which works in any webview, and sets the four flags the desktop shell adds: saving in place, watching files, native printing and several windows. Until the todos that build them land, those four calls reject with `NotYetSupportedError`, which names the todo.
- **`src/app.tsx`** puts the reader's top bar, home screen and status bar in the window; **`src/app-info.ts`** calls the shell's one command, `app_info`, whose version the status bar shows.
- **`src-tauri/`** is the Rust crate `laminapdf-desktop`, a member of the Cargo workspace. `src/lib.rs` builds the app and its main window; `src/commands.rs` holds the commands the webview may call; `src/error.rs` holds `DesktopError`, which reaches the UI as `{ kind, message }`.

## Security baseline

ADR 0008 §6 sets these rules, and every later todo keeps them:

- **The webview gets no plugin permissions.** The dialog, log and opener plugins are for Rust's use. The webview may call only the app's own commands, each allowed by name in `src-tauri/capabilities/main.json` (today only `allow-app-info`; not even `core:default`).
- **The capability guard checks the files.** `src-tauri/tests/capabilities.rs` fails on a `shell:`, `fs:`, `http:`, `dialog:` or `process:` permission, on a capability file missing from `app.security.capabilities`, and on an `e2e` capability outside `tauri.e2e.conf.json`.
- **The CSP allows only the app's own files.** `tauri.conf.json` allows no inline script, no other origin, and no frames. `withGlobalTauri` is off and the prototype is frozen.

Tauri writes `src-tauri/gen/` on every build; it is not committed.
