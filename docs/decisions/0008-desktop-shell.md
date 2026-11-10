# 0008 — Desktop shell: Tauri 2, a sandboxed engine process, WebP tiles over `lamina://`, printing with the engine's rasters

- Status: Accepted when D2a's PR merges (proposed 2026-11-09)
- Date: 2026-11-09

## Context

Lamina PDF's desktop reader must reuse the web reader's UI (`@laminapdf/ui`, `@laminapdf/viewer`, [ADR 0009](0009-web-client-architecture.md)), link the Rust core natively, open 1 GB files, and never let a hostile PDF take down the app ([desktop plan §4](../plans/desktop-app.md#4-requirements)). It also has to start in under 300 ms and ship an installer under 15 MB with PDFium included, the two release gates the owner set on 2026-10-29. Every later desktop todo (engine process, tiles, files, menus, printing, updates, signing) builds on three things this ADR fixes: the shell, the process split and the security baseline. D2a's PR must get them right. D2d and D2e measure the 300 ms start, and D6b's release workflow fails any installer of 15 MB or more.

The research picked Tauri 2 and left four questions to measure ([report](../research/architecture/app-and-package-architecture.md#tauri-2-is-the-desktop-shell-with-electron-held-in-reserve-for-linux); [desktop notes Q1–Q4](../research/architecture/notes/desktop-shell.md#q1-shell-options-versions-size-memory-startup-webview-differences-licence-maturity-production-users)):

- **The shell.** Tauri lets `src-tauri` call the Rust core directly and share about 90 % of the UI with the web app. Its unpacked app was 57 MB against Electron's 323 MB in Better Stack's July 2026 test, though memory was closer (about 109 MB against 128 MB); the 15 MB gate applies to the compressed installer. Its known weakness is Linux: Tauri's own docs list blank windows, DMABUF and Wayland errors, and canvas content that "can silently fall back to a slow path" on WebKitGTK.
- **Tiles.** Which transport moves rendered tiles into the webview fastest, and does any webview become cross-origin isolated? The [`desktop-ipc` spike](../../spikes/desktop-ipc/README.md) (D1a) measured both on macOS 27 (M1 Pro), a hosted macOS 15 runner, Windows Server 2025 (WebView2 156) and Ubuntu 24.04 (WebKitGTK 2.52.6, X11 and Wayland).
- **Printing.** Tauri can only print the webview. The [`desktop-print` spike](../../spikes/desktop-print/README.md) (D1b) scored webview, native raster and native vector printing against fixtures on the same machines.
- **Isolation.** The plan asks for a sandboxed child process that parses and renders untrusted bytes, like Chrome's out-of-process PDF viewer, but leaves its granularity open ([open question 3](../plans/desktop-app.md#8-open-questions)).

The owner decided the app's identity on 2026-11-09: product name "Lamina PDF", bundle identifier `com.laminapdf.desktop`. The identifier is permanent, because updates, file associations and macOS permissions are keyed to it.

## Decision

### 1. Tauri 2, pinned to the 2.12 minor, distributed directly

- **Shell:** Tauri 2, pinned to the 2.12 minor: crate `tauri` 2.12.5, `tauri-build` 2.7.4, `@tauri-apps/api` and `@tauri-apps/cli` 2.12.4, the versions both spikes measured. A minor bump is its own PR that reruns the desktop CI jobs; Dependabot groups the `tauri` crates and the `@tauri-apps/*` packages ([ADR 0019](0019-supply-chain-and-licence-hygiene.md)).
- **Plugins:** `tauri-plugin-dialog` 2.8.3, `tauri-plugin-log` 2.10.1 and `tauri-plugin-opener` 2.7.2, pinned exactly like Tauri.
- **Identity:** product name and window title "Lamina PDF", identifier `com.laminapdf.desktop`, which never changes.
- **Version:** the app's version is the crate's, which is the workspace's lockstep version ([ADR 0017](0017-versioning-and-release.md)). `apps/desktop/package.json` is a private package at 0.0.0 outside Changesets.
- **Layout** ([ADR 0015](0015-monorepo-layout-and-tooling.md)): `apps/desktop` is the private pnpm package `@laminapdf/desktop`. Its crate `apps/desktop/src-tauri` (`laminapdf-desktop`, layer `app`, `publish = false`) joins the Cargo workspace through the `"apps/*/src-tauri"` glob. Building the workspace on Linux therefore needs WebKitGTK 4.1 and D-Bus (`libwebkit2gtk-4.1-dev libdbus-1-dev`). CI installs them on every Linux job that builds the workspace. The full CI run (the full lane) also builds the app on Ubuntu 22.04, the oldest Linux it supports, and on macOS and Windows.
- **Desktop only:** the crate has no mobile entry point (`tauri::mobile_entry_point`). A mobile shell is ADR 0025's question.
- **Distribution:** direct downloads with Lamina PDF's own updater (ADR 0014), Developer ID signing and notarisation on macOS. No Mac App Store: it requires App Sandbox on the whole app and disables the self-updater ([desktop notes Q4](../research/architecture/notes/desktop-shell.md#q4-file-system-file-associations-drag--drop-printing-sandboxing-multi-window)). Other stores stay an open question in the plan.
- **Licences:** Tauri 2.12 itself pulls in five MPL-2.0 crates (`selectors`, `cssparser`, `cssparser-macros`, `dtoa-short`, `option-ext`; [D1a, Licences](../../spikes/desktop-ipc/README.md#licences)). The owner granted per-crate exceptions on 2026-11-09 (#371, recorded in ADR 0019 and `deny.toml`). The desktop app's third-party notices carry the MPL-2.0 text (`cargo xtask desktop-notices`). RustSec marks four crates in Tauri's Linux tree unmaintained (no vulnerability): `proc-macro-error`, through the GTK 3 bindings, and `zvariant`, `zvariant_derive` and `zbus_names`, through `tauri-plugin-opener`'s zbus 5. `deny.toml` accepts them with a review date, as ADR 0019's policy for unmaintained crates asks.

### 2. Process model: a main process that owns files and a sandboxed engine per window

| | Main process (`laminapdf-desktop`) | Engine process (`laminapdf-desktop-engine`) |
| --- | --- | --- |
| Owns | Windows, menus, dialogs, the updater, file handles, file watching, the printer handle | Parsing and rendering: the `laminapdf` façade and `laminapdf-render`'s PDFium adapter ([ADR 0004](0004-render-backend.md)) |
| Sees paths | Yes; paths never reach the webview or the engine ([ADR 0010](0010-platform-adapter.md)) | No: it gets bytes only |
| Untrusted bytes | Moves them, never interprets them | Interprets them, sandboxed (§5) |
| Links | Tauri, the protocol crate; never the render stack (§3) | The core crates, PDFium; never Tauri or the webview |

- **Granularity:** one engine per window to start, the simplest design that keeps one window's crash out of the others. D2b measures an engine's memory, and a shared pool with per-document isolation is reconsidered only if those numbers miss the plan's idle-memory target (150 MB with one 10-page document).
- **Restart policy:** an engine that exits unexpectedly or misses its heartbeats is respawned with backoff. Its window's documents are reopened, encrypted ones with the password the main process keeps for them, and the UI shows "document process restarted". A fourth crash within 60 s marks that window's engine failed, and the UI shows an error instead of looping. D2b sets the timings.
- **How bytes reach the engine:** the main process opens each file read-only and answers the engine's range requests over the engine pipe with positioned reads. It never memory-maps a document: a mapping of a file another app truncates raises `SIGBUS` (or `EXCEPTION_IN_PAGE_ERROR` on Windows) in the main process and takes down every window. Positioned reads turn that into an I/O error. On Windows the handle allows other apps to write, rename and delete the file.
- **Large files:** D2b measures range reads against the 1 GB target (first page within 1.5 s). If they miss it, the fallback hands the engine a read-only handle, never a path, so the engine can map the file itself.
- **Watching:** the main process watches open files with `notify` in Rust and asks the engine to reload (D3b).

### 3. The engine is its own binary, and the protocol is its own crate

- **Engine:** a separate bin crate `crates/laminapdf-desktop-engine` (layer `app`, `publish = false`), shipped as a Tauri sidecar (Tauri's term for a helper binary in the bundle) through `bundle.externalBin`. Tauri then names the binary per target and signs it inside the macOS bundle. The main process spawns it with `std::process::Command`; `tauri-plugin-shell` stays unregistered.
- **Protocol:** the messages between the main process and the engine, and their framing, live in `crates/laminapdf-desktop-protocol` (layer `app`, `publish = false`), which both sides use. It depends only on `serde`, `postcard`, `thiserror` and `zeroize`, so the main process never links `laminapdf-render`, PDFium or the parser. D2b checks that with `cargo tree` in CI, by exact package name.

### 4. Tiles: lossless WebP over the `lamina://` scheme, 512 px, single-threaded

| Choice | Decision | The numbers behind it ([D1a](../../spikes/desktop-ipc/README.md#recommendation-inputs-to-adr-0008); medians of 5 runs, 512 px, DPR 2) |
| --- | --- | --- |
| Transport | A custom scheme: `lamina://localhost/tile/…`, `http://lamina.localhost/tile/…` on Windows. The webview `fetch`es it and decodes with `createImageBitmap(blob)`. `convertFileSrc("", "lamina")` gives the right base on each OS | WebP moved 701 tiles/s against 260 for the best raw transport on Windows, 820–897 against 331–332 on Linux, 397 against 319 on macOS 15, and 990 against 970 on the M1 Pro (a tie) |
| Format | Lossless WebP, encoded in Rust off the main thread | 0.8–1.7 ms to encode a tile against 2.6–5.2 ms to render it; a 1 MB PDF tile becomes 8–12 KB |
| Fallback | The same URL without `fmt`: raw RGBA from the same handler, no encoder | Within 2 % of the fastest raw transport on every machine; a photo-heavy page on a fast Mac is where it can win (1,021 against 828 tiles/s for synthetic tiles on the M1 Pro) |
| Not used | `tauri::ipc::Channel` for tiles | The slowest raw transport on every machine (211 tiles/s on Windows), the worst p95 latency there (107 ms), and it needs request batching |
| Tile size | 512 device pixels | WebP moved 1.7–2.4× more pixels per second at 512 px than at 256 px (Windows: 184 against 77 Mpx/s) |
| Isolation | No `app.security.headers`: no COOP or COEP; the desktop app runs the single-threaded build everywhere, and its `crossOriginIsolated` flag is `false` | [Only WebView2 became isolated](../../spikes/desktop-ipc/README.md#cross-origin-isolation) with `SharedArrayBuffer`. WKWebView on macOS 27 reports `crossOriginIsolated = true` without `SharedArrayBuffer`; macOS 15 and WebKitGTK ignore the headers |

The engine renders a tile and streams it to the main process; the scheme handler encodes it and answers. The handler answers `Access-Control-Allow-Origin: *`, because the scheme's origin differs from the page's (`lamina://localhost` against `tauri://localhost`). D2d builds this. The hosted runners had no GPU, so the gap between WebP and raw RGBA on Windows and Linux is an upper bound ([D1a, Limits](../../spikes/desktop-ipc/README.md#limits-of-this-spike)); D2e measures real GPUs.

### 5. Sandboxing intent per OS

The engine needs only its pipes, its loaded libraries and memory, because §2 keeps paths and file handles in the main process. The target is an engine that can't read files, use the network, start processes or touch other processes. D2c builds it and amends this ADR with a table of what each OS actually enforces, its known gaps and the OS versions it needs.

| OS | Intent |
| --- | --- |
| macOS | The App Sandbox entitlement on the engine process only, with no file or network entitlements. Fallback: a Seatbelt profile through `sandbox_init`, if App Sandbox breaks the pipes or loading PDFium |
| Windows | A restricted token at Low integrity, in a job object (memory limit, one process, kill on close) with child processes blocked; AppContainer with no capabilities if pipes and PDFium work there |
| Linux | `PR_SET_NO_NEW_PRIVS`, then seccomp-bpf and Landlock where the kernel supports them; otherwise no-new-privs and namespaces |

An engine the sandbox kills is treated as a crash (§2's restart policy).

### 6. Security baseline

D2a's scaffold sets this baseline, and every later todo keeps it or tightens it:

- **CSP** (`app.security.csp`), with each scheme in both its macOS/Linux form and its Windows form:

  ```text
  default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
  img-src 'self' blob: data: lamina: http://lamina.localhost;
  connect-src 'self' ipc: http://ipc.localhost lamina: http://lamina.localhost;
  worker-src 'self' blob:; font-src 'self'; object-src 'none'; frame-src 'none'; base-uri 'none';
  form-action 'none'
  ```

- **Webview hardening:** `freezePrototype: true`, `withGlobalTauri: false` (the UI imports `@tauri-apps/api` instead of reading a global), and no `app.security.headers` (§4).
- **Windows are created in Rust**, the first with the label `main` and the title "Lamina PDF"; the config declares none.
- **Capabilities:** `app.security.capabilities` lists every enabled capability file by name (`["main"]`), so a file nobody enabled can't sit there unnoticed. `capabilities/main.json` grants only `allow-app-info`. It leaves out `core:default`, because the scaffold's UI calls no window, event, path, menu or tray API. A later todo that needs a `core:*` permission grants that one permission in its own PR.
- **App commands only through an app manifest** in `build.rs`, so each command needs its own `allow-*` permission in a capability. The scaffold's one command is `app_info`.
- **Plugins for Rust only:** `dialog`, `log` and `opener` are registered for the main process's own use. The webview gets no `dialog:`, `shell:`, `fs:`, `http:`, `process:`, `opener:` or `log:` permission. A capability guard test fails when any capability names one of those prefixes, and when the capability list and the files disagree. The guard also checks `capabilities/e2e.json` (§8): only `tauri.e2e.conf.json` may enable it, and that config must list every base capability too.
- **PDF JavaScript is never executed.**
- **External links** open only through the `opener` plugin called from Rust, after the user confirms in the UI (`openExternal`, ADR 0010); http(s) and mailto only.

### 7. Linux fallback

- **Canvas2D only:** pages are Rust rasters drawn with `drawImage`; no WebGL or WebGPU path, which WebKitGTK may run in software or lack.
- **Renderer workarounds:** WebKitGTK's DMABUF renderer froze painting for good under Wayland without a GPU driver, in both attempts, and on X11 it dropped 48–93 frames per run against 3–11 with it off ([D1a, Linux renderer](../../spikes/desktop-ipc/README.md#linux-renderer)). So the app sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` before WebKitGTK starts when the GPU driver can't serve EGL. Settings offer a renderer switch (Automatic, Compatibility, Safe mode with `WEBKIT_DISABLE_COMPOSITING_MODE=1`; D7b).
- **Last resorts, in order:** Tauri with the CEF runtime once it is production-ready, Electron on Linux only, then Electron everywhere ([architecture §6](../plans/architecture.md#6-risks)).
- **Triggers for revisiting:** a blank or frozen window on a supported distribution that the renderer switch can't fix, beta crash reports or issues clustering on WebKitGTK, or D2e's real-GPU and NVIDIA runs missing the plan's targets.

### 8. Test strategy per OS

Playwright can't drive Tauri webviews, so the desktop build runs the reader's end-to-end scenarios with WebdriverIO and `@wdio/tauri-service`:

| OS | Driver |
| --- | --- |
| Windows, Linux | `tauri-driver` (with `msedgedriver` matching WebView2 on Windows, `WebKitWebDriver` under Xvfb on Linux) |
| macOS | The service's embedded WebDriver server (`tauri-plugin-wdio-webdriver`), compiled into test builds only behind the Cargo feature `e2e`, with its capability in `capabilities/e2e.json` |

Release builds contain neither the plugin nor the e2e commands, and CI checks that. The web build's Playwright suite keeps running on all three OSes. If the macOS embedded driver proves unreliable, macOS falls back to that Playwright run plus a recorded manual smoke checklist. D2e fills this section in with what runs where and records any such gap here.

### 9. Printing: the engine's rasters through each OS's print API

Printing follows [D1b's recommendation](../../spikes/desktop-print/README.md#recommendation-inputs-to-d4c-and-adr-0008), which D4c implements:

- **One path on every OS:** the engine renders each page at the printer's resolution, capped at 300 dpi, and streams it to the main process. The main process owns the printer handle and draws the pages through `NSPrintOperation`, GDI or `GtkPrintOperation`, behind the OS print dialog. On Windows and Linux every page of the mixed-size fixture printed at its own size and orientation, no page moved more than 2 pt, and SSIM was at least 0.95.
- **No webview print path, not even as a fallback.** WebView2's `PrintToPdf` put 0 of 50 pages at the right size, WebKitGTK printed 100 sheets for 50 pages, and `window.print()` does nothing in WKWebView inside a Tauri app.
- **Vector printing on Windows** (PDFium's `FPDF_RenderPage` onto the printer's device context) scored highest (SSIM 0.98–0.999, 16.6 MB against 41.6 MB for 50 pages). But it parses the untrusted PDF where the printer handle lives, so it may ship only behind a sandboxed print helper or an owner-accepted exception to this ADR.
- **macOS needs work in D4c:** AppKit snapped US Letter and A5 pages to A4 (2 of 4 sizes right). The imageable offset must be read per paper and orientation: a landscape page sat 17.3 pt off. And 50 pages at 300 dpi peaked at 1.7–1.8 GB.
- **D4c's fidelity bar:** print-to-PDF output at SSIM ≥ 0.95 (150 dpi, after alignment), each page's size and content within 2 pt.

**`hasNativePrint` restated.** ADR 0010's table defined the flag as "`print` can hand the PDF to the OS print system", but the desktop's print path hands over rendered pages, not the PDF. On desktop the flag is `true` on all three OSes. It promises:

- the OS print dialog: printer, page range, copies, and the driver's duplex and scaling;
- every page rendered by Lamina PDF's own renderer at print resolution, on paper of its own size and orientation;
- no browser headers, footers or margins.

It doesn't promise vector output. This ADR amends ADR 0010's `hasNativePrint` row and its `PrintJob` note to match.

## Consequences

- **The UI stays one codebase.** The desktop app is a `Platform` implementation and a Rust crate; the web packages gain no desktop code paths (ADR 0010).
- **A hostile PDF costs one window's engine,** not the app, and paths never leave the main process. The price is a second binary to build, sign and ship (universal on macOS), a framed protocol to maintain, and one copy per range read.
- **Tiles cost an encode:** 0.8–1.7 ms of CPU per tile in Rust, in exchange for 2.5–2.7× the tiles on Windows and Linux runners. Raw RGBA stays one query parameter away.
- **No threaded WebAssembly on desktop.** The webview runs the single-threaded build everywhere, as the web reader does in v0.1; heavy work runs natively in the engine instead.
- **The baseline is enforced by tests,** not by reviewers' memory: the capability guard, the app manifest, the `cargo tree` check and the release-build e2e guard. A todo that needs a new webview permission must change a capability file and pass the guard.
- **Printing gives up vector output** on every OS for now, and file sizes grow (41.6 MB for 50 pages on Windows). macOS mixed paper sizes stay D4c's problem.
- **Linux carries extra cost:** a renderer switch, environment workarounds and a documented Electron fallback.
- **The identifier is fixed.** Renaming the product later can change its display name, never `com.laminapdf.desktop`.

## Alternatives considered

- **Electron everywhere.** Uniform Chromium rendering on every OS, but the Rust core would be reached only through napi-rs or WebAssembly, a binding layer with no feature gain, and the bundle was 323 MB against Tauri's 57 MB. Kept as the Linux last resort (§7).
- **Native UIs per OS** (SwiftUI, WinUI 3, GTK4). Rejected: three UIs plus the web one is more than a small team can keep in step.
- **A Rust-native UI** (GPUI, Slint, egui, Iced). Rejected: each gives up the shared web UI; GPUI has no official crate and is pre-1.0, and Slint's royalty-free licence requires attribution.
- **Tauri with the CEF runtime.** Uniform rendering inside Tauri, but not production-ready in 2026 and each app ships its own CEF. First in §7's fallback order.
- **Re-executing the main binary with `--engine`** (Chrome-style). Rejected: the engine process would load the webview stack, which enlarges what an attacker reaches and complicates the sandbox.
- **A shared engine pool.** Deferred, not rejected: it saves memory with many windows but lets one document's crash affect others. Revisited with D2b's memory numbers.
- **Memory-mapping documents in the main process,** as the research suggested. Rejected: a file truncated underneath crashes every window, and a Windows mapping blocks other apps from replacing the file. Mapping inside the engine stays the fallback.
- **The parser in the main process** with only rendering isolated. Rejected: the parser reads untrusted bytes too.
- **`tauri::ipc::Channel` or `ipc::Response` for tiles.** Both carry raw RGBA only, which was limited to about 160 MB/s on the Windows runner and 200 MB/s on Linux; the channel was also the slowest raw transport everywhere.
- **PNG or QOI tiles.** PNG matched WebP within 8 % on Windows and macOS 15 but ran at half its speed on Linux and below raw on the M1 Pro. QOI needs a decoder in script on the main thread, and was the slowest transport on macOS 15.
- **COOP/COEP with a threaded build on Windows.** Rejected: one OS out of three, and two builds to test for one webview.
- **Webview printing** (`window.print()` or Tauri's print, the web reader's C6c path). Rejected on fidelity (§9). The plan's risk table once suggested shipping it first if native printing lagged; D1b's numbers supersede that.
- **PDFKit on macOS and CUPS with the original bytes on Linux.** Rejected: PDFKit moved content 18 pt and put every page on A4 portrait, and CUPS printed every page on the default paper. Both also parse the PDF outside the engine.
- **The Mac App Store.** Rejected for now: App Sandbox on the whole app and no self-updater.
