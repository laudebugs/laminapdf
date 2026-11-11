# @laminapdf/desktop

The desktop reader: `@laminapdf/ui`'s reader chrome around `@laminapdf/viewer` in a Tauri 2 window on macOS, Windows and Linux ([ADR 0008](../../docs/decisions/0008-desktop-shell.md)). Choose a file opens one PDF from the system's file dialog. A separate engine process parses and draws it, and restarts after a crash (todo [D2b](../../docs/todos/done/D2b-engine-process.md)). The engine's tiles reach the viewer over the app's own `lamina://` scheme (todo [D2d](../../docs/todos/ongoing/D2d-tile-transport-and-first-document.md)). A locked document asks for its password. Text selection, search, links and thumbnails come later: the engine doesn't read text yet, and dropped files arrive with opening from the OS (D3a).

## Build, run and test

The Cargo workspace includes the app, so building it on Linux needs WebKitGTK and D-Bus (`cargo xtask doctor` checks for them):

```sh
sudo apt-get install libwebkit2gtk-4.1-dev libdbus-1-dev pkg-config
```

Without a GPU driver on Linux, set `WEBKIT_DISABLE_DMABUF_RENDERER=1` (ADR 0008 §7). macOS and Windows need nothing beyond the repository's toolchain: the app uses WKWebView and WebView2, which the OS provides.

The frontend imports the built viewer and UI packages, and the viewer is built over `@laminapdf/sdk` and its WebAssembly, so build them once, then run the app:

```sh
cargo xtask desktop-frontend                          # the WebAssembly, @laminapdf/sdk, the viewer, ui, tokens, fonts and icons
pnpm --filter @laminapdf/desktop run desktop:dev      # the engine and PDFium, then Vite on port 1420 and the app
pnpm --filter @laminapdf/desktop run desktop:build --debug --no-bundle   # what CI builds: target/debug/laminapdf-desktop
pnpm --filter @laminapdf/desktop test                 # the frontend's tests (Node, Tauri's IPC mocked)
pnpm --filter @laminapdf/desktop run typecheck:wasm   # the frontend's types, over the built viewer
cargo test -p laminapdf-desktop                       # the shell's tests and the capability guard
cargo xtask desktop-sidecar [--release]               # only the engine and PDFium (desktop:dev and desktop:build run it)
cargo xtask desktop-notices                           # target/desktop/THIRD_PARTY_NOTICES.md
```

`desktop:dev` and `desktop:build` merge `src-tauri/tauri.bundle.conf.json`, which bundles the engine binary (`src-tauri/binaries/`, Tauri's sidecar naming) and PDFium (`src-tauri/resources/pdfium/`); `cargo xtask desktop-sidecar` writes both, and Git ignores them. The base `tauri.conf.json` names neither, so plain `cargo build` and `cargo test` work on a fresh clone; documents then fail to open with `engine-start`, because no engine sits beside the app.

To measure the engine on a large file:

```sh
cargo xtask fixtures large-pdf                        # target/fixtures/1gb.pdf: 20,000 image pages
cargo build --release -p laminapdf-desktop-engine
cargo run --release -p laminapdf-desktop-engine --example measure -- \
    --engine target/release/laminapdf-desktop-engine --file target/fixtures/1gb.pdf --tiles 200
```

Add `#raw` to the window's address (in the dev tools) to fetch tiles as raw RGBA instead of WebP, which ADR 0008 §4 keeps for diagnosis.

## How it fits together

```text
 webview (src/)                     main process (src-tauri/)               engine process
 ─────────────                      ─────────────────────────               ──────────────
 @laminapdf/viewer                  commands: open_dialog,                  laminapdf-desktop-engine
   └ DesktopEngineClient ─invoke──▶   unlock_document, cancel_tiles … ─┐      parses and renders
       fetch lamina://localhost/ ──▶ scheme handler: checks the URL,   ├─▶  (stdin/stdout frames,
         tile/{doc}/{page}?…          the window owns the doc ─────────┘      laminapdf-desktop-
       ◀── WebP tile ───────────────  encodes the engine's RGBA  ◀────────    protocol)
       ◀── engine://restarted ─────   restarts a crashed engine             reads ranges of the
                                      answers its range reads ──────────▶   file over the pipe
```

- **`src/main.tsx`** loads the design tokens (`@laminapdf/tokens`), fonts and component styles, creates the desktop `Platform`, and renders the app. It is the only module that asks about its surroundings: whether it runs inside Tauri (`isTauri()`; `cargo xtask check ts` refuses the call anywhere else) and whether shortcuts show ⌘ or Ctrl.
- **`src/platform/desktop.ts`** is the desktop `Platform` (ADR 0010). It starts from the web platform, which works in any webview, and sets the four capability flags the desktop shell adds: saving in place, watching files, native printing and several windows. Until the shell implements them, the four matching methods reject with `NotYetSupportedError`, which names the feature.
- **`src/app.tsx`** puts the reader's top bar, home screen, the document's view and status bar in the window, with the password dialog, the failure page, the repaired-file notice and the "Document process restarted" toast; **`src/app-info.ts`** calls `app_info`, whose version the status bar shows.
- **`src/engine/`** is the viewer's engine on the desktop. `desktop-engine-client.ts` is `DesktopEngineClient`, the `ViewerEngine` that `@laminapdf/viewer` draws from, as it draws from `ViewerClient` on the web. It fetches each tile from `lamina://localhost/tile/{doc}/{page}?…` (`http://lamina.localhost/…` on Windows; `tile-url.ts` builds the address), decodes the WebP, and closes a tile that lands after its render was aborted or its document went. An aborted render's ticket goes to `cancel_tiles`, so the engine skips it. The engine's restart events become the viewer's `restarted`, which redraws the pages on screen. `open.ts` holds the open dialog's files and the reader's opener, which shows a document or unlocks it first. `tauri-shell.ts` is the Tauri side the client runs on; the tests pass Tauri's mocks.
- **`src-tauri/`** is the Rust crate `laminapdf-desktop`, a member of the Cargo workspace. `src/lib.rs` builds the app and its main window; `src/commands.rs` holds the commands the webview may call; `src/error.rs` holds `DesktopError`, which reaches the UI as `{ kind, message }`.
- **`src-tauri/src/engine/tiles.rs`** answers the `lamina://` scheme (ADR 0008 §4). It parses the tile's address strictly, takes the window from the webview that asked (never from the address), has the window's engine draw the tile, checks the pixels against the size asked for, and sends them as lossless WebP, or raw RGBA when the address has no `fmt`. A failure answers `{ kind, message }` with a status the client maps back.
- **`src-tauri/src/engine/`** runs documents in the engine process (ADR 0008 §2). Each window gets its own engine, `crates/laminapdf-desktop-engine`, started on its first document. The main process opens each file read-only and answers the engine's range requests with positioned reads (`bytesource.rs`), so neither the webview nor the engine ever sees a path. `documents.rs` maps each document to its file, its window and its kept password, and keeps each window's engine away from other windows' documents. When an engine crashes or stops answering, the protocol crate's supervisor starts a new one, and `mod.rs` reopens the window's documents, unlocking encrypted ones with the kept password (`secret.rs`). The window then gets `engine://restarted`, which says whether each document is open, locked or closed; a document that fails to reopen, or kills the new engine, is closed. A fourth crash within 60 s sends `engine://failed` instead.

## Security baseline

ADR 0008 §6 sets these rules, and every later todo keeps or tightens them:

- **The webview gets no plugin permissions.** The dialog, log and opener plugins are for Rust's use. The webview may call only the app's own commands, each allowed by name in `src-tauri/capabilities/main.json` (today `allow-app-info`, `allow-open-dialog`, `allow-unlock-document`, `allow-page-info`, `allow-cancel-tiles` and `allow-close-document`), plus `core:event:allow-listen` and `allow-unlisten` for the engine's events; not `core:default`. The webview names documents by id, never by path.
- **The capability guard checks the files.** `src-tauri/tests/capabilities.rs` fails on a `shell:`, `fs:`, `http:`, `dialog:`, `process:`, `opener:` or `log:` permission, on a capability file missing from `app.security.capabilities`, and on an `e2e` capability outside `tauri.e2e.conf.json`.
- **A window reads only its own documents.** The tile scheme takes the window from the webview that asked, and every command checks the document belongs to the calling window.
- **A password is typed once.** `unlock_document` moves it into memory that is wiped when dropped as soon as it arrives; the main process keeps it for restarts and never logs it.
- **The CSP allows the app's own files plus the tile and IPC schemes.** `tauri.conf.json` allows no inline script, no other web origin, and no frames. `withGlobalTauri` is off and the prototype is frozen.
- **The main process never parses a PDF.** It links only the protocol crate; CI's `desktop` job fails if `cargo tree` shows the parser, the renderer or PDFium under `laminapdf-desktop`.
- **Release builds have no test commands.** `debug_kill_engine` exists only with the Cargo feature `e2e`; CI checks the built app doesn't contain its name.

Tauri writes `src-tauri/gen/` on every build, and Git ignores it.
