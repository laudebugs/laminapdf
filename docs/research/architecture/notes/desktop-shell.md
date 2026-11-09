# Desktop App Shell and Distribution for Lamina (as of 2026-10-28)

Scope: choosing a desktop shell (Tauri 2 / Electron / native / Rust-native UI / Flutter) and a distribution + auto-update pipeline hosted on Cloudflare, for a Rust-core PDF reader → editor whose TS web UI is shared with a Cloudflare-hosted web app.

Legend: **[UNVERIFIED]** = from background knowledge (training data, mid-2026 cutoff) not re-confirmed by a fetched source in this session; treat as a lead to check.

## Q1. Shell options: versions, size, memory, startup, webview differences, licence, maturity, production users

### Takeaway
Tauri 2 (core 2.12.5, 21 Oct 2026; v3 in alpha) is the best fit: it reuses the web UI, talks to the Rust core natively, and is far smaller on disk than Electron (runtime memory is closer than the headlines suggest). Its main risk is the Linux WebKitGTK webview: GPU, WebGL and WebGPU are fragile there. Electron (45, Chromium 156, 21 Oct 2026) is the uniform-rendering fallback.

### Cited Findings
**Versions / maturity**
- Tauri core latest stable **v2.12.5 (21 Oct)**, `@tauri-apps/cli` **v2.12.4 (21 Oct)**; pre-release **v3.0.0-alpha.5 (14 Oct)**, notes list only dependency upgrades. — [tauri releases](https://github.com/tauri-apps/tauri/releases)
- Electron latest majors: **45 (21 Oct 2026, Chromium 156.0.8078.48, V8 15.6, Node 24.22.0)**, 44 (25 Aug 2026, Chromium 152), 43 (2 Jul 2026), 42 (7 May 2026), 41 (10 Mar 2026), 40 (13 Jan 2026). Roughly an 8-week cadence. — [electronjs.org blog](https://www.electronjs.org/blog)
- Electron 39.x has reached end of support; only the latest three stable majors are supported. — [releases.electronjs.org v39.8.10](https://releases.electronjs.org/release/v39.8.10); [endoflife.date](https://endoflife.date/electron)
- Electron 39 made ASAR integrity (runtime validation of app.asar against a build-time hash) stable. — [Electron 39 blog](https://electronjs.org/blog/electron-39-0)
- Tauri updater plugin requires Rust ≥ 1.90. — [Tauri updater docs](https://v2.tauri.app/plugin/updater/)

**Size / memory / startup (benchmarks vary widely; methodology often missing)**
- Better Stack (Jul 2026) test app: Tauri bundle 57 MB vs Electron 323 MB; memory **Tauri ~109 MB vs Electron ~128 MB**; Tauri startup 311 ms. Notes "runtime memory footprints are much closer than on-disk sizes." — [Better Stack](https://betterstack.com/community/guides/scaling-nodejs/tauri-vs-electron-vs-deno-vs-electrobun/)
- Other 2026 blogs: hello-world 3.2 MB (Tauri) vs 85 MB (Electron); cold start ~380 ms vs ~1,420 ms; idle 42 MB vs 168 MB. Low-rigour, no reproducible methodology. — [tech-insider](https://tech-insider.org/tauri-vs-electron-2026/); [pkgpulse](https://www.pkgpulse.com/guides/electron-vs-tauri-2026)
- One 2026 analysis disputes the "96% smaller / 5× less RAM" claim as unreproduced. — [dasroot.net](https://dasroot.net/posts/2026/03/tauri-vs-electron-rust-cross-platform-apps/)

**Webview differences**
- **Linux/WebKitGTK (Tauri docs, updated 15 Jun 2026):** blank/white windows, resize flicker, resize crashes, `AcceleratedSurfaceDMABuf` errors and Wayland `Error 71`, mostly on NVIDIA through the DMABUF renderer. Workarounds: `nvidia_drm.modeset=1`; `__NV_DISABLE_EXPLICIT_SYNC=1`; `WEBKIT_DISABLE_DMABUF_RENDERER=1` (loses the fast path); `WEBKIT_DISABLE_COMPOSITING_MODE=1` (last resort). "WebGL and canvas content can silently fall back to a slow path." WebGL2 contexts can be software-backed, and `WEBGL_debug_renderer_info` reports "Apple GPU" on Linux, so you can't use it to detect this. Recommendation: ship a non-WebGL fallback on Linux plus a user-facing renderer switch. — [Tauri Linux graphics](https://v2.tauri.app/develop/debug/linux-graphics/)
- WebKitGTK WebGPU: not supported as of the 2023 bug; nothing found showing it has shipped. Tauri closed its WebGPU issue, saying the flag existed only in non-stable WebView2 channels. — [WebKit bug 257694](https://bugs.webkit.org/show_bug.cgi?id=257694); [tauri#6381](https://github.com/tauri-apps/tauri/issues/6381)
- WebKitGTK moved 2D rendering to Skia (Igalia) for performance. — [linuxadictos](https://en.linuxadictos.com/skia-support-was-added-webkitgtk-and-wpewebkit-for-rendering-2d-graphics.html)
- **macOS/WKWebView:** WebGPU ships on by default in Safari 26 on macOS 26 Tahoe and later; earlier macOS needs a flag. No source confirmed that WKWebView inherits the default. — [gpuweb Implementation Status](https://github.com/gpuweb/gpuweb/wiki/Implementation-Status); [WebKit bug 299237](https://bugs.webkit.org/show_bug.cgi?id=299237)
- **CEF option for Tauri:** Tauri maintains `cef-rs` (Rust CEF bindings). As of Mar 2026 CEF-backed webviews still lacked standard navigation controls in Tauri's public API. A shared CEF runtime is reportedly on the roadmap, but each app ships its own CEF today (size cost). Not production-confirmed. — [cef-rs](https://docsearch.algolia.com/mcp/docs/repo/tauri-apps/cef-rs); [HN thread via brianlovin](https://brianlovin.com/hn/48626137); [harbor task tauri#14661](https://hub.harborframework.com/tasks/abundant/tauri-apps__tauri-14661)
- HN commenters call Linux Tauri support "hellish" (opinion). — [HN thread](https://brianlovin.com/hn/48626137)

**Rust-native UI / others**
- GPUI (Zed) is still pre-1.0 with frequent breaking changes. There is no official standalone crate, only unofficial mirrors (`gpui-unofficial` 1.22.0-pre, 24 Sep 2026, "not maintained by the Zed team"). GPUI Kit v0.8.0 (19 Oct 2026) adds Linux WebView support. — [lib.rs gpui-unofficial](https://lib.rs/crates/gpui-unofficial); [realab/gpui-standalone](https://github.com/realab/gpui-standalone); [GPUI Kit releases](https://gpui-kit.com/releases/)
- Slint licence: GPL-3.0 OR Royalty-free 2.0 OR Slint Software 3.0 (paid). The royalty-free licence covers proprietary desktop/mobile/web apps but requires attribution (AboutSlint widget or badge), excludes embedded, and has had the attribution clause since Jun 2023. Versions up to 1.17.0 exist. — [Slint licence (espressif mirror 1.17.0)](https://components.espressif.com/components/slint/slint/versions/1.17.0/license?language=en); [Slint royalty-free licence PDF](https://slint.dev/agreements/slint-royalty-free-license.pdf)

**Production users**
- Tauri: Spacedrive, AppFlowy, Clash Verge (blog claims); Cloudflare and 1Password reportedly "used Tauri in production products" (low-quality secondary source). — [noqta](https://www.noqta.tn/en/blog/tauri-2-desktop-apps-rust-web-technologies-2026); [GUVI](https://www.guvi.in/blog/tauri-2-0-desktop-app-tutorial/)
- **Stirling-PDF** (open-source PDF tool) has a `frontend/src-tauri` Tauri desktop build, the closest domain peer. — [Stirling-PDF mirror Cargo.toml](https://github.com/Stirling-Tools/Stirling-PDF/tree/main/frontend/src-tauri)

### Inferences
- For lamina, Tauri gives about 90% UI reuse with the Cloudflare web app and native Rust calls. The PDF canvas must not depend on WebGL or WebGPU on Linux: use a CPU/WASM or Rust-rendered bitmap path (Canvas2D / `ImageBitmap`) as the baseline and treat GPU as an enhancement.
- If Linux rendering quality becomes a support burden, the fallbacks are, in order: (a) Tauri with the CEF runtime once it matures, (b) Electron on Linux only (two shells, more CI), (c) Electron everywhere.
- Native per-platform UIs (SwiftUI/WinUI 3/GTK4) mean three UIs plus the web one, which is not justified for a small team. The Rust-native UIs (Slint/egui/Iced/GPUI) give up the shared web UI. GPUI's API churn makes it risky.

### Gaps
- Exact current Slint, Iced, egui and Dioxus versions, and the Dioxus "native" (Blitz) status, were not verified this session.
- No fetched data on Flutter desktop for this use case, or on accessibility comparisons. [UNVERIFIED]: Electron inherits Chromium's mature a11y tree. Tauri relies on each OS webview's a11y (WKWebView/WebView2 good, WebKitGTK via ATK/AT-SPI weaker). egui uses AccessKit (partial). GPUI's a11y is limited.
- [UNVERIFIED] Windows WebView2 is evergreen Chromium (WebGPU and OffscreenCanvas on by default in stable Edge since ~2023). WKWebView OffscreenCanvas shipped in Safari 17. Needs a check.
- Electron per-release support window details were not fetched.

## Q2. Code sharing between web app and desktop app

### Takeaway
Use one TS UI package that targets both the browser (Rust core compiled to WASM, running in a Worker) and Tauri (Rust core native over IPC), behind a "platform adapter" interface chosen at runtime.

### Cited Findings
- Tauri frontend calls go through `invoke()` and the Channel APIs. Desktop-only features come from plugins (fs, dialog, updater, deep-link, single-instance). — [Tauri calling Rust](https://v2.tauri.app/develop/calling-rust/); [Tauri plugins](https://v2.tauri.app/fr/plugin/)
- Stirling-PDF keeps its Tauri shell inside the web `frontend/` package (`frontend/src-tauri`), sharing one frontend between web and desktop. — [Stirling-PDF mirror](https://github.com/Stirling-Tools/Stirling-PDF/tree/main/frontend/src-tauri)

### Inferences
- Suggested monorepo layout: `packages/core` (Rust: PDF parse/render/edit), `packages/core-wasm` (wasm-bindgen build for web), `packages/ui` (TS components plus an `Engine` interface), `apps/web` (Cloudflare Workers/Pages, `Engine` = WASM in a Web Worker), `apps/desktop` (Tauri, `src-tauri` depends on `core` natively, `Engine` = invoke/channel/custom-protocol adapter).
- Feature detection: check `window.__TAURI_INTERNALS__` / `isTauri()` from `@tauri-apps/api/core` at startup **[UNVERIFIED API name]**, then expose capability flags (`canWriteInPlace`, `canWatchFiles`, `hasNativePrint`, `multiWindow`). The web build uses the File System Access API where present (Chromium) and falls back to download/upload.

### Gaps
- No published case study found on a shared engine adapter between WASM and Tauri IPC for a document app.

## Q3. Performance: native Rust over IPC vs WASM in webview; bitmaps; 1 GB files; mmap; file watching

### Takeaway
Use raw-binary IPC, never JSON. Return page tiles as `tauri::ipc::Response` or stream them over Channels, or serve them from a custom URI scheme (`lamina://page/…`) that the webview fetches like images. Keep 1 GB PDFs memory-mapped in Rust instead of copying them into the webview. That is the main advantage of native over in-webview WASM, which is bound by wasm32's 4 GB memory and a copy into the JS heap.

### Cited Findings
- "Return values that implement serde::Serialize are serialized to JSON… This can slow down your application if you try to return large data." Use `tauri::ipc::Response` for raw binary. Commands can take `tauri::ipc::Request` with `InvokeBody::Raw`, and the frontend sends `ArrayBuffer`/`Uint8Array` as the payload. — [Tauri calling Rust](https://v2.tauri.app/develop/calling-rust/)
- "The Tauri channel is the recommended mechanism for streaming data." Events "only support JSON payloads… For ordered, high-throughput data delivery, consider using Channels." — [Tauri calling Rust](https://v2.tauri.app/develop/calling-rust/)
- "Commands without the async keyword are executed on the main thread." Async commands are preferred for heavy work. — [Tauri calling Rust](https://v2.tauri.app/develop/calling-rust/)
- A JSON payload encodes bytes as one number per byte. Third-party `tauri-conduit` benchmark at 64 KB: ~202 µs (binary path) vs 2.27 ms (built-in invoke). Self-reported numbers. — [tauri-conduit](https://github.com/userFRM/tauri-conduit)
- Tauri v2 IPC still goes over fetch, which adds latency, and large payloads have been reported slow. — [tauri discussion #11915](https://github.com/orgs/tauri-apps/discussions/11915)
- Tauri 2.0.0-rc.3 added a dedicated IPC response body type "for performance reasons." — [tauri release rc.3](https://v2.tauri.app/zh-cn/release/tauri/v2.0.0-rc.3)
- wry custom protocols support streaming and partial content (Range). — [tauri-runtime-wry release](https://v2.tauri.app/release/tauri-runtime-wry/v0.2.1/); [zenn.dev experiment](https://zenn.dev/maemon/scraps/2132b28a6c49e8)

### Inferences
- Render tiles in Rust (pdfium/mupdf/own renderer) to RGBA or compressed (PNG/WebP/QOI) bytes. Deliver them either as `<img src="lamina://tile/{doc}/{page}/{zoom}">` (browser cache and decoding off the main thread) or as raw RGBA over a Channel into `ImageData`/`createImageBitmap`. An A4 page at 2× DPR is about 1654×2339×4 ≈ 15 MB raw, so tile and/or compress, and only transfer the visible viewport.
- Use `memmap2` for 1 GB PDFs, and use the `notify` crate in Rust for file watching rather than the fs plugin's watch from JS **[UNVERIFIED: fs plugin has `watch` behind a feature flag]**.
- Keep WASM in the webview for the web app only. A desktop WASM fallback is useful just for parity testing.

### Gaps
- No official Tauri benchmark comparing Channel, custom protocol and `ipc::Response` throughput. Prototype-measure it on all three OS webviews, since WebKitGTK custom-scheme performance may differ.
  - *Update (2026-11-09, [`desktop-ipc` spike](../../../../spikes/desktop-ipc/README.md)):* measured on all three webviews. In medians of 5 runs at 512 px, lossless WebP over a `lamina://` scheme moved 2.5–2.7× more tiles per second than the best raw transport on the Windows and Linux runners (no GPU) and tied on an M1 Pro; `ipc::Response` and a raw scheme were within 6 % of each other at 512 px, DPR 2, and the channel was the slowest raw transport. WebKitGTK fetches custom schemes as the others do. Only WebView2 becomes cross-origin isolated with `SharedArrayBuffer`.

## Q4. File system, file associations, drag & drop, printing, sandboxing, multi-window

### Takeaway
Tauri supports file associations (`bundle.fileAssociations`), `RunEvent::Opened` on macOS, and single-instance plus deep-link argument forwarding on Windows/Linux. Mac App Store distribution requires App Sandbox. Plan on direct (Developer ID) distribution first, because MAS means no self-updater and sandboxed file access.

### Cited Findings
- `RunEvent::Opened` delivers file URLs on macOS/iOS/Android. Handle cold start by storing the URLs in state behind a command, and handle the running case by emitting an event. — [Tauri file associations](https://v2.tauri.app/learn/mobile-file-associations/)
- Tao added macOS `Event::Opened` for file association and deep link. — [tao v0.21.0](https://v2.tauri.app/release/tao/v0.21.0/)
- Deep-link: on macOS links must be registered in config (no runtime registration). On Windows/Linux the link arrives as a CLI argument to a new process, so use single-instance with its `deep-link` feature to forward it. — [Tauri deep-linking](https://v2.tauri.app/de/plugin/deep-linking/)
- single-instance fixed the macOS cwd of the second instance (v2.2.4). — [single-instance release](https://v2.tauri.app/release/single-instance/v2.2.4/)
- MAS requires the `com.apple.security.app-sandbox` entitlement, a Mac App Store Connect provisioning profile embedded as `embedded.provisionprofile`, `ITSAppUsesNonExemptEncryption`, and a universal `.app` packaged as a `.pkg` with `productbuild` and uploaded with `xcrun altool`. — [Tauri App Store](https://v2.tauri.app/distribute/app-store/)

### Inferences
- The same file-associations config covers Windows (registry via NSIS/MSI) and Linux (`.desktop` MimeType) **[UNVERIFIED]**. On Windows/Linux, "Open with" passes the path as argv, so single-instance is needed to route it to the existing window.
- [UNVERIFIED] MAS sandbox: you need `com.apple.security.files.user-selected.read-write` plus security-scoped bookmarks to reopen recent files. Tauri's updater must be disabled in MAS builds. MAS is optional.
- [UNVERIFIED] Printing: Tauri has no first-class print API beyond `window.print()` on the webview. Printing PDFs faithfully likely needs a native per-OS path (NSPrintOperation, Windows print APIs, GTK print) or sending the original PDF to the OS print dialog. Electron has `webContents.print`/`printToPDF`. Worth a dedicated spike.
- Drag and drop: Tauri emits native drag-drop events with file paths **[UNVERIFIED event names]**. Multi-window: supported through `WebviewWindowBuilder`.

### Gaps
- Not verified: the macOS second-launch path (Opened vs single-instance), the print API status, and Linux portal/Flatpak file access.
  - *Update (2026-11-09, [`desktop-print` spike](../../../../spikes/desktop-print/README.md)):* printing measured on all three OSes. Tauri's only print API prints the webview, and that path is the least faithful: WebView2's `PrintToPdf` put every page on US Letter, WebKitGTK split each page across two sheets, and `window.print()` does nothing in WKWebView. PDFium rasters through AppKit, GDI or GTK printed every page at its own size on Windows and Linux; AppKit snapped mixed paper sizes to A4.

## Q5. Auto-update and hosting on Cloudflare R2 + Workers; delta updates

### Takeaway
The Tauri updater is a good fit for Cloudflare. It uses mandatory signatures (its own key pair) and either a static `latest.json` on R2 or a dynamic Worker endpoint returning 200 JSON or 204. It has no delta updates: full bundles only.

### Cited Findings
- Signing is mandatory and can't be disabled. `signer generate` creates the keys, the pubkey goes in `tauri.conf.json`, and the private key goes in the `TAURI_SIGNING_PRIVATE_KEY` env var (`.env` doesn't work). Losing the key blocks updates for existing users. — [Tauri updater](https://v2.tauri.app/plugin/updater/)
- Static JSON: `version`, plus `platforms.{OS-ARCH}.url` and `signature` (the `.sig` contents); optional `notes` and `pub_date`. The whole file is validated, so every platform entry must be complete. Dynamic endpoint: 200 with `url`/`version`/`signature`, or **204 No Content**. URL vars: `{{current_version}}`, `{{target}}`, `{{arch}}`. Rollback is possible with a version-comparator override. — [Tauri updater](https://v2.tauri.app/plugin/updater/)
- Update bundles: Linux AppImage plus `.sig`; macOS `.app.tar.gz` plus `.sig`; Windows NSIS/MSI plus `.sig`. Windows `installMode` is passive (default), basicUi or quiet, and the app exits during install. The docs don't mention delta updates. — [Tauri updater](https://v2.tauri.app/plugin/updater/)
- `tauri-latest-json` crate generates `latest.json` with a custom download base URL (works with an R2 domain). — [docs.rs tauri-latest-json](https://docs.rs/crate/tauri-latest-json/latest)
- R2: the updater must fetch through a public custom domain or r2.dev, because the S3 API endpoint requires auth. — [electron-builder R2Options](https://www.electron.build/docs/api/builder-util-runtime.Interface.R2Options)
- Electron: Squirrel.Windows uses RELEASES plus full and delta `.nupkg`. electron-updater (NSIS) uses `.blockmap` differential downloads, a separate mechanism. electron-builder has a built-in R2 publisher. — [update-electron-app](https://github.com/electron/update-electron-app); [Joplin write-up](https://discourse.joplinapp.org/t/weekly-update-3-beginning-of-automated-updates/38621); [electron-builder R2Options](https://www.electron.build/docs/api/builder-util-runtime.Interface.R2Options)
- Self-hosted Tauri release servers exist (Oasis: manifests, crash reports, feedback). — [skills.sh oasis](https://www.skills.sh/porkytheblack/coco/oasis-server-setup)

### Inferences
- Pipeline: CI uploads artifacts and `.sig` files to R2 under `releases/{version}/{target}-{arch}/…`. A Worker at `updates.lamina.app/{{target}}/{{arch}}/{{current_version}}?channel=` reads a channel manifest (KV or R2 JSON) and returns 204 or a JSON pointing at the R2 custom domain. This allows staged rollouts (hash the install ID to a percentage), channels and kill switches without republishing.
- [UNVERIFIED] Sparkle 2 (EdDSA, appcast, delta updates) is the macOS-native option if lamina ever ships a native mac shell. It doesn't matter for Tauri.
- Accept full-bundle updates (the Tauri bundle is tens of MB). Delta updates are only a concern with Electron-sized bundles.

### Gaps
- No found reference implementation of a Tauri updater Worker plus R2. No Tauri delta-update roadmap found.

## Q6. Code signing & notarization; Linux packaging; CI

### Takeaway
macOS: Developer ID plus notarization through an App Store Connect API key in CI. Windows: Azure **Artifact Signing** (renamed from Trusted Signing in early 2026) through Tauri's `signCommand`, but public-trust **individual** eligibility is US/Canada only. Otherwise an organization identity or a cloud-HSM OV/EV cert. Linux: AppImage (used by the updater) plus deb/rpm, with Flatpak optional.

### Cited Findings
- Tauri macOS notarization supports App Store Connect API key env vars (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`) or Apple ID, app-specific password and team ID. — [Tauri macOS signing](https://tauri.app/distribute/sign/macos); [Tauri env vars](https://v2.tauri.app/reference/environment-variables/)
- `bundle.windows.signCommand` runs a custom signing command with `%1` replaced by the file path. A third-party example uses `trusted-signing-cli` with endpoint, account and profile flags (not verified against official docs). — [skills.cat signing guide](https://skills.cat/skills/dchuk/claude-code-tauri-skills/signing-tauri-apps)
- Artifact Signing: "Individual developers must be located in the United States or Canada"; this limit doesn't apply to Private Trust. Organizations: US/Canada/EU/UK, and may need 3 years of verifiable tax history. Renamed from Trusted Signing in early 2026. — [MS Artifact Signing quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart); [devclass, 14 Jan 2026](https://devclass.com/2026/01/14/code-signing-windows-apps-may-be-easier-and-more-secure-with-new-azure-artifact-service/)
- Microsoft Q&A (AI-generated answer): an Irish sole trader doesn't qualify as an individual for public trust. — [MS Q&A](https://learn.microsoft.com/en-us/answers/questions/5915697/can-an-irish-sole-trader-(non-incorporated)-qualif)
- Tauri updater artifacts on Linux are AppImage only. — [Tauri updater](https://v2.tauri.app/plugin/updater/)

### Inferences
- [UNVERIFIED] Since 2023, CA/B Forum rules require OV/EV code-signing keys on HSMs, so a cloud HSM (Azure Key Vault, DigiCert KeyLocker, SSL.com eSigner) is needed in CI. EV no longer grants instant SmartScreen reputation (a 2024 Microsoft change).
- [UNVERIFIED] `tauri-apps/tauri-action` builds a matrix (macos-latest arm64/x64 or universal, windows-latest, ubuntu-22.04), signs and notarizes using the env vars above, and can emit `latest.json`. Then add a step that `wrangler r2 object put`s the artifacts to R2 instead of (or as well as) GitHub Releases.
- Flatpak apps can't use the Tauri self-updater meaningfully (Flathub handles updates), and sandbox portals affect file access.

### Gaps
- Not fetched: the tauri-action README and the current Flathub requirements for Tauri apps.

## Q7. Mobile future: Tauri 2 mobile vs UniFFI to Swift/Kotlin

### Takeaway
Tauri 2 supports iOS and Android (the updater plugin even lists them, and file associations work through `RunEvent::Opened`), so the same web UI could go mobile. UniFFI (0.32, Mozilla, used in Firefox mobile) is the mature path if mobile needs native SwiftUI/Compose UIs over the same Rust core.

### Cited Findings
- Tauri mobile file associations: `RunEvent::Opened` on iOS/Android. — [Tauri mobile file associations](https://v2.tauri.app/learn/mobile-file-associations/)
- UniFFI 0.32.0 (around Jul 2026): recursive enums, Kotlin/Swift `mutable_records`, and byte buffers passed as pointer + length (`ForeignBytes`) instead of being copied (helps page bitmaps). Bot-PR-sourced. — [SnapXRust PR](https://github.com/SnapXL/SnapXRust/pull/13); [uniffi changelog](https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md)
- UniFFI is "used extensively by Mozilla in Firefox mobile and desktop browsers". It fully supports Kotlin, Swift and Python, and Gobley adds Kotlin Multiplatform. — [docs.rs uniffi](https://docs.rs/crate/uniffi/latest)

### Inferences
- Design `core` with a UniFFI-friendly façade (owned types, byte buffers, a handle-based document API) now. The same façade then maps to Tauri commands, wasm-bindgen and UniFFI.

### Gaps
- No fetched data on Tauri mobile production maturity or iOS WKWebView memory limits with large PDFs.
