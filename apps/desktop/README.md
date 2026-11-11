# @laminapdf/desktop

The desktop reader: `@laminapdf/ui`'s reader chrome in a Tauri 2 window on macOS, Windows and Linux ([ADR 0008](../../docs/decisions/0008-desktop-shell.md)). This build opens one window with the reader's top bar and home screen, and parses documents in a separate engine process that restarts after a crash (todo [D2b](../../docs/todos/done/D2b-engine-process.md)). Showing a document's pages arrives with the tile transport ([D2d](../../docs/todos/ready/D2d-tile-transport-and-first-document.md)); until then, choosing a file in the UI shows a notice that says so.

## Build, run and test

The Cargo workspace includes the app, so building it on Linux needs WebKitGTK and D-Bus (`cargo xtask doctor` checks for them):

```sh
sudo apt-get install libwebkit2gtk-4.1-dev libdbus-1-dev pkg-config
```

Without a GPU driver on Linux, set `WEBKIT_DISABLE_DMABUF_RENDERER=1` (ADR 0008 §7). macOS and Windows need nothing beyond the repository's toolchain: the app uses WKWebView and WebView2, which the OS provides.

The frontend imports the built UI packages, so build them once, then run the app:

```sh
pnpm --filter "@laminapdf/desktop^..." run build      # @laminapdf/tokens, fonts, icons and ui
pnpm --filter @laminapdf/desktop run desktop:dev      # the engine and PDFium, then Vite on port 1420 and the app
pnpm --filter @laminapdf/desktop run desktop:build --debug --no-bundle   # what CI builds: target/debug/laminapdf-desktop
pnpm --filter @laminapdf/desktop test                 # the frontend's tests (Node, Tauri's IPC mocked)
cargo test -p laminapdf-desktop                       # the shell's tests and the capability guard
cargo xtask desktop-sidecar [--release]               # only the engine and PDFium (desktop:dev and desktop:build run it)
cargo xtask desktop-notices                           # target/desktop/THIRD_PARTY_NOTICES.md
```

`desktop:dev` and `desktop:build` merge `src-tauri/tauri.bundle.conf.json`, which bundles the engine binary (`src-tauri/binaries/`, Tauri's sidecar naming) and PDFium (`src-tauri/resources/pdfium/`); `cargo xtask desktop-sidecar` writes both, and Git ignores them. The base `tauri.conf.json` names neither, so plain `cargo build` and `cargo test` work on a fresh clone; documents then fail to open with `engine-start`, because no engine sits beside the app.

The engine runs sandboxed on every OS ([ADR 0008 §5](../../docs/decisions/0008-desktop-shell.md#5-sandboxing-per-os)): it can't open files, use the network or start processes. To see whether a problem comes from the sandbox, turn it off in a debug build and try again:

```sh
LAMINA_PDF_ENGINE_SANDBOX=off pnpm --filter @laminapdf/desktop run desktop:dev
target/debug/laminapdf-desktop-engine --sandbox-selftest          # what the sandbox blocks, probe by probe
cargo build -p laminapdf-desktop-engine --features seccomp-trap   # Linux: name a system call the filter refuses
```

The log names each engine's sandbox layers (`engine 4242 is ready; sandbox: seatbelt`) and says `off` when the variable is set. A release build ignores the variable, so a user's environment can't turn the sandbox off. On Linux the seccomp filter kills an engine that makes a system call it doesn't expect; the log then says `sandbox violation: SIGSYS`, and the `seccomp-trap` build prints the call's number instead of dying silently.

To measure the engine on a large file:

```sh
cargo xtask fixtures large-pdf                        # target/fixtures/1gb.pdf: 20,000 image pages
cargo build --release -p laminapdf-desktop-engine
cargo run --release -p laminapdf-desktop-engine --example measure -- \
    --engine target/release/laminapdf-desktop-engine --file target/fixtures/1gb.pdf --tiles 200
```

## How it fits together

- **`src/main.tsx`** loads the design tokens (`@laminapdf/tokens`), fonts and component styles, creates the desktop `Platform`, and renders the app. It is the only module that asks about its surroundings: whether it runs inside Tauri (`isTauri()`; `cargo xtask check ts` refuses the call anywhere else) and whether shortcuts show ⌘ or Ctrl.
- **`src/platform/desktop.ts`** is the desktop `Platform` (ADR 0010). It starts from the web platform, which works in any webview, and sets the four capability flags the desktop shell adds: saving in place, watching files, native printing and several windows. Until the shell implements them, the four matching methods reject with `NotYetSupportedError`, which names the feature.
- **`src/app.tsx`** puts the reader's top bar, home screen and status bar in the window; **`src/app-info.ts`** calls the shell's one command, `app_info`, whose version the status bar shows.
- **`src-tauri/`** is the Rust crate `laminapdf-desktop`, a member of the Cargo workspace. `src/lib.rs` builds the app and its main window; `src/commands.rs` holds the commands the webview may call; `src/error.rs` holds `DesktopError`, which reaches the UI as `{ kind, message }`.
- **`src-tauri/src/engine/`** runs documents in the engine process (ADR 0008 §2). Each window gets its own engine, `crates/laminapdf-desktop-engine`, started on its first document and sandboxed (`crates/laminapdf-desktop-sandbox`). The main process opens each file read-only and answers the engine's range requests with positioned reads (`bytesource.rs`), so neither the webview nor the engine ever sees a path. `documents.rs` maps each document to its file, its window and its kept password, and keeps each window's engine away from other windows' documents. When an engine crashes or stops answering, the protocol crate's supervisor starts a new one, and `mod.rs` reopens the window's documents, unlocking encrypted ones with the kept password (`secret.rs`). The window then gets `engine://restarted`, which says whether each document is open, locked or closed; a document that fails to reopen, or kills the new engine, is closed. A fourth crash within 60 s sends `engine://failed` instead.

## Security baseline

ADR 0008 §6 sets these rules, and every later todo keeps or tightens them:

- **The webview gets no plugin permissions.** The dialog, log and opener plugins are for Rust's use. The webview may call only the app's own commands, each allowed by name in `src-tauri/capabilities/main.json` (today `allow-app-info`, `allow-open-dialog`, `allow-page-info` and `allow-close-document`; not even `core:default`). The webview names documents by id, never by path.
- **The capability guard checks the files.** `src-tauri/tests/capabilities.rs` fails on a `shell:`, `fs:`, `http:`, `dialog:`, `process:`, `opener:` or `log:` permission, on a capability file missing from `app.security.capabilities`, and on an `e2e` capability outside `tauri.e2e.conf.json`.
- **The CSP allows the app's own files plus the tile and IPC schemes.** `tauri.conf.json` allows no inline script, no other web origin, and no frames. `withGlobalTauri` is off and the prototype is frozen.
- **The main process never parses a PDF.** It links only the protocol crate; CI's `desktop` job fails if `cargo tree` shows the parser, the renderer or PDFium under `laminapdf-desktop`.
- **Release builds have no test commands.** `debug_kill_engine` exists only with the Cargo feature `e2e`; CI checks the built app doesn't contain its name.

Tauri writes `src-tauri/gen/` on every build, and Git ignores it.
