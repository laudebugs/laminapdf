# @laminapdf/desktop

The desktop reader: `@laminapdf/ui`'s reader chrome around `@laminapdf/viewer` in a Tauri 2 window on macOS, Windows and Linux ([ADR 0008](../../docs/decisions/0008-desktop-shell.md)). Choose a file opens one PDF from the system's file dialog. A separate engine process parses and draws it, and restarts after a crash (todo [D2b](../../docs/todos/done/D2b-engine-process.md)). The engine's tiles reach the viewer over the app's own `lamina://` scheme (todo [D2d](../../docs/todos/done/D2d-tile-transport-and-first-document.md)). A locked document asks for its password. The app opens the PDFs the OS hands it: a double-click or "Open with", a second launch, Finder and the Dock on macOS, and files dropped on the window (todo [D3a](../../docs/todos/done/D3a-open-from-os.md)). The start window lists recent files, which also reach the OS's own recent lists, and a document whose file another app changes reloads at the same page (todo [D3b](../../docs/todos/ongoing/D3b-recent-files-and-file-watching.md)). Text selection, search, links and thumbnails come later: the engine doesn't read text yet.

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

### End-to-end tests

WebdriverIO drives the app's e2e build through the WebDriver server it embeds, so no driver needs installing ([ADR 0008 §8](../../docs/decisions/0008-desktop-shell.md#8-test-strategy-per-os)). The scenarios share one app and run in a fixed order. `e2e` runs them, then a second run that starts the app with a PDF's path, as a double-click does. Two scenarios need generated fixtures:

```sh
cargo xtask fixtures large-pdf --pages 1000 --out target/fixtures/1000-pages.pdf
cargo xtask fixtures large-pdf                                     # target/fixtures/1gb.pdf
pnpm --filter @laminapdf/desktop run desktop:build:e2e             # feature e2e + tauri.e2e.conf.json
pnpm --filter @laminapdf/desktop run e2e                           # both runs (Linux: under dbus-run-session and xvfb-run)
pnpm --filter @laminapdf/desktop run cold-start                    # five launches, median time to the empty state
```

Each OS needs only what building the app needs. Linux adds `xvfb` for a display, a D-Bus session bus so a second launch can find the running app, and `WEBKIT_DISABLE_DMABUF_RENDERER=1` without a GPU driver: run `dbus-run-session -- xvfb-run pnpm --filter @laminapdf/desktop run e2e`. Windows needs the WebView2 runtime, which Windows 10 and 11 include, and no driver, since the app embeds its own. A failed scenario leaves a screenshot in `target/desktop-e2e/`. The first launch after a fresh build is slow on macOS while the OS checks the new binary, so run the scenarios again before trusting a first-run failure. A locked Mac screen stops WebKit's animation frames, so nothing draws and every scenario that waits for a page fails: run them with the screen unlocked.

### Opening from the OS

Every way the OS hands the app a file goes through `src-tauri/src/open.rs` ([ADR 0008 §10](../../docs/decisions/0008-desktop-shell.md#10-opening-files-from-the-os)). From a terminal, start the built app with a path, then start it again with another while it runs: the second process hands its file over and exits.

```sh
target/debug/laminapdf-desktop fixtures/golden/examples/invoice.pdf
target/debug/laminapdf-desktop fixtures/golden/examples/hello.pdf   # opens in the first app
```

File associations exist only in an installed bundle. `bundle.fileAssociations` registers the app for PDFs as an alternate handler, never the default, and `src-tauri/linux/laminapdf.desktop` passes the files (`%F`) on Linux. [`e2e/MANUAL-OPEN.md`](e2e/MANUAL-OPEN.md) builds a dev bundle per OS and lists the checks WebDriver can't make: Finder, Explorer, the Dock and a real drag.

### Recent files and files that change on disk

The main process records each document as it opens, in `recent.json` in the app's config folder and in the OS's list: the Dock and File ▸ Open Recent on macOS, the taskbar's Jump List on Windows, and `recently-used.xbel` on Linux ([ADR 0008 §11](../../docs/decisions/0008-desktop-shell.md#11-recent-files-and-files-that-change-on-disk)). It also watches each open document's folder and reloads a file another app saved, once the change has settled. To see a reload, open a PDF and copy another one over it:

```sh
cp fixtures/golden/examples/hello.pdf /tmp/watched.pdf
target/debug/laminapdf-desktop /tmp/watched.pdf
cp fixtures/golden/examples/invoice_from_json.pdf /tmp/watched.pdf   # the window reloads: 2 pages
```

Settings (in the More menu) turns either off. The config folder is `~/Library/Application Support/com.laminapdf.desktop` on macOS, `%APPDATA%\com.laminapdf.desktop` on Windows and `~/.config/com.laminapdf.desktop` on Linux; e2e builds use `com.laminapdf.desktop.e2e`. [`e2e/MANUAL-OPEN.md`](e2e/MANUAL-OPEN.md) lists the checks of each OS's own recent list.

The engine runs sandboxed on every OS ([ADR 0008 §5](../../docs/decisions/0008-desktop-shell.md#5-sandboxing-per-os)): it can't open files, use the network or start processes. It gets the system's fonts from the main process, which lists them once at start-up. To see whether a problem comes from the sandbox, turn it off in a debug build and try again:

```sh
LAMINA_PDF_ENGINE_SANDBOX=off pnpm --filter @laminapdf/desktop run desktop:dev
cargo run -p laminapdf-desktop-engine --features selftest -- --sandbox-selftest   # what the sandbox blocks, probe by probe
cargo build -p laminapdf-desktop-engine --features seccomp-trap                  # Linux: name a system call the filter refuses
```

The log names each engine's sandbox layers (`engine 4242 is ready; sandbox: seatbelt`) and says `off` when the variable is set. A release build ignores the variable, so a user's environment can't turn the sandbox off, and the main process refuses an engine that reports a layer missing. On Linux the seccomp filter kills an engine that makes a system call it doesn't expect; the log then says `sandbox violation: SIGSYS`, and a `seccomp-trap` build also prints which call it was.

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
- **`src/platform/desktop.ts`** is the desktop `Platform` (ADR 0010). It starts from the web platform, which works in any webview, and sets the capability flags the desktop shell adds. `ownsKeyboard` turns on the shortcuts a browser would keep, such as ⌘− and ⌘W. `recent` (`recent.ts`) and `watch` go through the main process. Saving in place, native printing and several windows aren't built yet: their methods reject with `NotYetSupportedError`, which names the feature.
- **`src/app.tsx`** puts the reader's top bar, home screen, the document's view and status bar in the window, with the password dialog, the failure page, the repaired-file notice and the "Document process restarted" toast; **`src/app-info.ts`** calls `app_info`, whose version the status bar shows. **`src/recent.tsx`** is the start window's recent list, **`src/settings.tsx`** the Settings dialog with its Files section, and **`src/file-changes.tsx`** shows a reloaded document in place, or the strip for a file that went.
- **`src/engine/`** is the viewer's engine on the desktop. `desktop-engine-client.ts` is `DesktopEngineClient`, the `ViewerEngine` that `@laminapdf/viewer` draws from, as it draws from `ViewerClient` on the web. It fetches each tile from `lamina://localhost/tile/{doc}/{page}?…` (`http://lamina.localhost/…` on Windows; `tile-url.ts` builds the address), decodes the WebP, and closes a tile that lands after its render was aborted or its document went. An aborted render's ticket goes to `cancel_tiles`, so the engine skips it. The engine's restart events become the viewer's `restarted`, which redraws the pages on screen. `open.ts` holds the open dialog's files and the reader's opener, which shows a document or unlocks it first. `tauri-shell.ts` is the Tauri side the client runs on; the tests pass Tauri's mocks.
- **`src-tauri/`** is the Rust crate `laminapdf-desktop`, a member of the Cargo workspace. `src/lib.rs` builds the app and its main window; `src/commands.rs` holds the commands the webview may call; `src/error.rs` holds `DesktopError`, which reaches the UI as `{ kind, message }`.
- **`src-tauri/src/open.rs`** opens what the OS asks: the command line, a second launch (`tauri-plugin-single-instance`, the first plugin registered), macOS's `RunEvent::Opened` and drops on the window. `route_open` queues the paths until the webview calls `take_pending_opens`, then opens each request's last path, in order, in the window's engine and sends `open://document` with its id and name. `src/engine/open.ts`'s `listenForOsOpens` shows each in turn, and the app shows the drop overlay from `open://drag-enter`.
- **`src-tauri/src/opened.rs`** is where every open from a path goes, whoever asked: the window's engine opens the file, `recent.rs` and the OS's list record it (through `crates/laminapdf-desktop-os`, the edge crate that holds the FFI), and `watch/` watches it. `prefs.rs` keeps the two choices in `preferences.json`, and `store.rs` writes both JSON files atomically.
- **`src-tauri/src/watch/`** watches each open document's folder with `notify`. `stamp.rs` decides when a change has settled; `mod.rs`'s thread waits for it and asks `app.rs` to reload, which opens the changed file as a new document, closes the old one and sends the window `document://reloaded`, `document://missing` or `document://unreadable`.
- **`src-tauri/src/engine/tiles.rs`** answers the `lamina://` scheme (ADR 0008 §4). It parses the tile's address strictly, takes the window from the webview that asked (never from the address), has the window's engine draw the tile, checks the pixels against the size asked for, and sends them as lossless WebP, or raw RGBA when the address has no `fmt`. A failure answers `{ kind, message }` with a status the client maps back.
- **`src-tauri/src/engine/`** runs documents in the engine process (ADR 0008 §2). Each window gets its own engine, `crates/laminapdf-desktop-engine`, started on its first document and sandboxed (`crates/laminapdf-desktop-sandbox`). The main process opens each file read-only and answers the engine's range requests with positioned reads (`bytesource.rs`), so neither the webview nor the engine ever sees a path. `documents.rs` maps each document to its file, its window and its kept password, and keeps each window's engine away from other windows' documents. When an engine crashes or stops answering, the protocol crate's supervisor starts a new one, and `mod.rs` reopens the window's documents, unlocking encrypted ones with the kept password (`secret.rs`). The window then gets `engine://restarted`, which says whether each document is open, locked or closed; a document that fails to reopen, or kills the new engine, is closed. A fourth crash within 60 s sends `engine://failed` instead.

## Security baseline

ADR 0008 §6 sets these rules, and every later todo keeps or tightens them:

- **The webview gets no plugin permissions.** The dialog, log and opener plugins are for Rust's use. The webview may call only the app's own commands, each allowed by name in `src-tauri/capabilities/main.json` (today `allow-app-info`, `allow-open-dialog`, `allow-unlock-document`, `allow-page-info`, `allow-cancel-tiles`, `allow-close-document`, `allow-take-pending-opens`, the four `allow-recent-*` and `allow-preferences`, `allow-set-preferences`), plus `core:event:allow-listen` and `allow-unlisten` for the engine's, the open requests' and the file changes' events; not `core:default`. The webview names documents and recent files by id, never by path. Tauri itself sends dropped files' paths to the webview's `tauri://drag-*` events, which the app doesn't listen to, and no command opens a path the webview names, so the webview can see a dropped path but can't open it.
- **The capability guard checks the files.** `src-tauri/tests/capabilities.rs` fails on a `shell:`, `fs:`, `http:`, `dialog:`, `process:`, `opener:` or `log:` permission, on a capability file missing from `app.security.capabilities`, and on an `e2e` capability outside `tauri.e2e.conf.json`.
- **A window reads only its own documents.** The tile scheme takes the window from the webview that asked, and every command checks the document belongs to the calling window.
- **A password is typed once.** `unlock_document` moves it into memory that is wiped when dropped as soon as it arrives; the main process keeps it for restarts and never logs it.
- **The CSP allows the app's own files plus the tile and IPC schemes.** `tauri.conf.json` allows no inline script, no other web origin, and no frames. `withGlobalTauri` is off and the prototype is frozen.
- **The main process never parses a PDF.** It reads a file's first 64 KiB to fingerprint it and both ends to notice a change, and hashes them; it links only the protocol crate; CI's `desktop` job fails if `cargo tree` shows the parser, the renderer or PDFium under `laminapdf-desktop`.
- **Release builds have no test code.** The test commands (`debug_kill_engine`, `e2e_queue_pick`, `e2e_interactive`) and the WebdriverIO plugins exist only with the Cargo feature `e2e`, and only Vite's `e2e` mode bundles `e2e/probe/probe.ts`; CI checks the built app and its bundle hold none of them.

Tauri writes `src-tauri/gen/` on every build, and Git ignores it.
