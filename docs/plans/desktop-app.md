# Plan: desktop PDF reader (`apps/desktop`)

- **Status:** Draft
- **Date:** 2026-10-28
- **Builds on:** [architecture plan](architecture.md) Round D (ADRs 0008, 0014; spike `desktop-ipc`) and the [architecture research report](../research/architecture/app-and-package-architecture.md) (Tauri and distribution sections, [desktop notes](../research/architecture/notes/desktop-shell.md))
- **Depends on:** [web-app.md](web-app.md) Rounds C2–C4 (`laminapdf-render`, `@laminapdf/viewer`, `@laminapdf/ui`, `Platform` adapter)
- **UI/UX:** [ui-ux.md](ui-ux.md) (layout, modes, command registry; native menus in D4b are generated from it)
- **Roadmap:** [roadmap.md](roadmap.md)

## 1. Goal

A native **PDF reader for macOS, Windows and Linux** built with Tauri 2. It reuses the web reader's UI packages, links the Rust core natively (no WASM), opens very large files via memory mapping, and updates itself from a Cloudflare Worker backed by R2. It has feature parity with the [web reader](web-app.md) plus what only a desktop app can do well: file associations, multi-window and tabs, native menus, in-place file watching, native printing and giant files. It is the second shell of the [editor](full-editor.md).

## 2. Scope and non-goals

**In scope (Rounds D1–D7)**

- Everything in the web reader's scope ([web-app §2](web-app.md#2-scope-and-non-goals)).
- File associations for `.pdf`, "Open with", drag onto the dock/taskbar icon, single-instance forwarding of paths.
- Recent files (OS recent documents plus an in-app list), reopen last session.
- Multiple windows and tabs, tab drag between windows.
- Native menus (macOS app menu, Windows/Linux menu bar) with the shared shortcut map.
- Large files (1 GB+) via positioned reads (no mmap in the main process: a file truncated by another app would crash it); parsing and rendering in a sandboxed child process.
- File watching: reload when the file changes on disk.
- Printing through a native path (decided by a print spike).
- Auto-update via the `apps/updates` Worker + R2 with stable/beta/nightly channels, staged rollout and kill switch.
- Signing and notarisation; packages: dmg (macOS, universal or per-arch), MSI and NSIS (Windows), AppImage and deb (Linux), rpm if cheap.
- Opt-in crash reporting.

**Non-goals (for now)**

- Mac App Store, Microsoft Store, Flathub, Snap (open question; MAS disables the self-updater).
- Editing and save (arrives with [full-editor.md](full-editor.md) rounds).
- Mobile (Tauri mobile vs UniFFI is an [architecture open question](architecture.md#8-open-questions-for-the-team)).
- Electron. It is the documented fallback for Linux if WebKitGTK fails ([architecture §6](architecture.md#6-risks)), not a plan.

## 3. Users and use cases

| User | Use case |
| --- | --- |
| Everyday users | Default PDF app that opens instantly from Finder/Explorer/Files |
| Power users | Many documents in tabs and windows, huge manuals, keyboard-driven |
| Offline and air-gapped users | A reader that never needs the network (updates can be disabled) |
| Organisations | Signed installers, MSI for managed deployment, update channels they can pin |

## 4. Requirements

### Functional

1. **Open** from file association, argv, drag-drop, menu, recent files; `single-instance` forwards to the running app; macOS `RunEvent::Opened` handled.
2. **Parity** with the web reader UI (shared `@laminapdf/ui`); differences go through `Platform` capability flags, never `if (isTauri)`.
3. **Native core**: `src-tauri` links `crates/*` by path. Parse and render in a child process; a crash shows "document process restarted" and reloads the document.
4. **Tiles** reach the webview over the transport chosen by the `desktop-ipc` spike (Channel with raw RGBA, `ipc::Response`, or `lamina://tile/...` custom scheme).
5. **Windows and tabs**: new window, new tab, move tab to window, restore session.
6. **Menus**: File (Open, Open Recent, Close, Print, Export copy), Edit (Copy, Find), View (zoom, sidebars, page layout), Window, Help; all with native accelerators.
7. **Printing**: native print dialog with page range, scaling, and duplex where the OS provides it.
8. **Updates**: check at start and every 24 h on the user's channel; download in background; install on quit or restart; signature verified; a setting disables updates; managed installs can pin a channel via config/registry/MDM.
9. **Crash reporting**: off by default; asks once after the first crash; reports contain stack, OS, version, renderer backend, never document content, file names or paths.

### Non-functional targets and release gates

**Release gates** (owner decision, 2026-10-29): cold start and installer size. A miss blocks the beta ([D7c](../todos/ready/D7c-desktop-beta-release.md)); the other rows are recommended targets.

| Metric | Target |
| --- | --- |
| Cold start to interactive window, mid-range machine | < 300 ms (macOS, Windows, Linux; owner decision 2026-10-29, from the [product research](../research/product/pdf-app-product-and-ux.md#a-phased-roadmap-that-fits-the-existing-rounds); was ≤ 1 s, ≤ 1.5 s on Linux) |
| First page of a 1 GB PDF | ≤ 1.5 s after open |
| Idle memory with one 10-page document | ≤ 150 MB |
| Installer size | < 15 MB per platform, PDFium native included (owner decision 2026-10-29; was ≤ 40 MB) |
| Supported OS | macOS 13+, Windows 11 (versions in Microsoft servicing), Ubuntu 22.04+ and supported Fedora releases (x64 and arm64 where Tauri supports it). Windows 10 22H2 is best-effort only: not tested in CI, no fixes for Windows 10-only bugs (end of support 14 October 2025, [Microsoft lifecycle](https://learn.microsoft.com/en-us/lifecycle/products/windows-10-home-and-pro)) |
| Update rollout | Configurable 1% → 10% → 50% → 100%, kill switch effective within 60 s |

### Security

- Webview CSP and Tauri capabilities restricted to the commands the UI needs; no `shell` or broad `fs` access from the webview.
- Child process with minimal privileges; OS sandboxing where available (macOS App Sandbox for the helper, Windows job objects/AppContainer, Linux seccomp/landlock) — scope decided in ADR 0008.
- Updater private key and signing identities held in a secret manager or HSM with a rotation runbook (ADR 0014).
- PDF JavaScript is never executed in the reader.

## 5. Design summary

Architecture details are in [architecture.md](architecture.md) and the report; the desktop app adds:

- **Layout**: `apps/desktop` is a pnpm package (frontend: `@laminapdf/ui` + desktop `Platform` adapter) with `src-tauri/` as a Cargo workspace member ([code organisation §1](../guidelines/code-organization.md#1-top-level-layout)).
- **Process model** (ADR 0008): Tauri main process (windows, menus, updater, file I/O only: positioned reads and file watching) → one sandboxed engine process per window or per N documents that does all parsing and rendering of untrusted files with the core crates + `laminapdf-render` (native PDFium via `pdfium-render`). Tiles of 256–512 px.
- **Platform adapter**: `canWriteInPlace: true`, `canWatchFiles: true`, `hasNativePrint: true`, `multiWindow: true`.
- **Distribution** (ADR 0014): GitHub Actions + tauri-action matrix → R2 `releases/{version}/{target}-{arch}/` with `.sig` files → channel manifests in R2/KV → `apps/updates` Worker at `updates.laminapdf.com/{channel}/{target}/{arch}/{current_version}` returning 204 or 200 JSON; downloads from `downloads.laminapdf.com` (R2 custom domain). Rollout percentage from a hash of an anonymous install ID. The updater URL only substitutes `{{target}}`, `{{arch}}` and `{{current_version}}`, so the app sends the ID in a custom request header (`Lamina-Install-Id`) set with the updater plugin's `UpdaterBuilder::header` in Rust or `check({ headers })` in JS ([Tauri updater plugin](https://v2.tauri.app/plugin/updater/)). The ID is a random UUID created on first run, stored only in the app's config, resettable in settings and never linked to crash reports; the Worker hashes it with the release version to pick a bucket and does not log it. A request without the header is served only releases at 100% rollout. ADR 0014 and the privacy page describe this.
- **Crash reports** (recommendation): an opt-in endpoint on the same Worker storing scrubbed minidumps/panic reports in R2 with 30-day retention, rather than a third-party SaaS. Alternative: self-hosted or SaaS Sentry. Decided in D7.

## 6. Rounds

Architecture Round D is delivered as rounds D1–D7, each split into todos (one PR each). D1 starts once C2c and C3b merge (the viewer core it measures); D2 needs C4a. The exact gates are in each todo's **Depends on**.

**Todos.** Each round is split into one or more todo files, one per pull request, on the [todo board](../todos/README.md) (`ready/` → `ongoing/` → `done/`). Each round below links its todos, and each todo lists its dependencies, including the cross-plan ones on web rounds C2–C4, C6 and C7. The links point into `ready/`; update them when a todo moves.

### D1. IPC and print spikes

**Todos:** [D1a](../todos/done/D1a-desktop-ipc-spike.md), [D1b](../todos/done/D1b-desktop-print-spike.md)

**Deliverable:** `spikes/desktop-ipc/` measuring Channel vs `ipc::Response` vs `lamina://` scheme for 256/512 px tiles on macOS, Windows and Linux, and whether WKWebView/WebView2/WebKitGTK honour COOP/COEP; `spikes/desktop-print/` comparing `window.print()` with native print paths per OS.
**Accepted when:** both READMEs report numbers per OS and recommend a tile transport and a print path.

**Result (`desktop-ipc`):** [spikes/desktop-ipc/README.md](../../spikes/desktop-ipc/README.md): lossless WebP tiles of 512 px over the `lamina://` scheme, raw RGBA on the same scheme as the fallback; cross-origin isolation only in WebView2, so the desktop app stays single-threaded; Linux without a GPU driver needs `WEBKIT_DISABLE_DMABUF_RENDERER=1`; Tauri's own dependencies need an MPL-2.0 exception (granted 2026-11-09, `deny.toml`).

**Result (`desktop-print`):** [spikes/desktop-print/README.md](../../spikes/desktop-print/README.md): D4c prints Lamina PDF's own rasters, rendered by the engine, through each OS's print API (AppKit, GDI, GTK) with the OS dialog; no webview print path ships, even as a fallback (WebView2 printed every page on US Letter, WebKitGTK split each page across two sheets, `window.print()` does nothing in WKWebView); PDFium's vector printing on Windows was the most faithful but needs a sandboxed print helper.

### D2. Shell and native core

**Todos:** [D2a](../todos/done/D2a-adr-0008-and-desktop-scaffold.md), [D2b](../todos/done/D2b-engine-process.md), [D2c](../todos/ongoing/D2c-engine-sandbox.md), [D2d](../todos/ready/D2d-tile-transport-and-first-document.md), [D2e](../todos/ready/D2e-desktop-e2e-webdriverio.md)

**Deliverable:** ADR 0008; `apps/desktop` with Tauri 2, desktop `Platform` adapter, engine child process, tile transport, single window opening one file.
**Accepted when:** the reader end-to-end scenarios run against the desktop build with WebdriverIO and `@wdio/tauri-service` on Windows and Linux (`tauri-driver`) and on macOS (the service's embedded WebDriver server via `tauri-plugin-wdio-webdriver`, compiled into test builds only behind a Cargo feature) ([Tauri WebDriver docs](https://v2.tauri.app/develop/tests/webdriver/)); the C-round Playwright suite keeps running against the web build on all three OSes (Playwright drives browsers, not Tauri webviews); killing the engine process restarts it and the document reloads; a 1 GB fixture opens within target. If the macOS embedded driver proves unreliable, macOS falls back to the web-build Playwright run plus a recorded manual smoke checklist, and ADR 0008 records the gap.

### D3. Files

**Todos:** [D3a](../todos/ready/D3a-open-from-os.md), [D3b](../todos/ready/D3b-recent-files-and-file-watching.md)

**Deliverable:** file associations, argv and `RunEvent::Opened`, single-instance forwarding, recent files (OS and in-app), file watching with reload, drag-drop onto window and icon.
**Accepted when:** double-clicking a PDF opens it in the running instance on each OS; editing the file externally reloads it keeping the page position.

### D4. Windows, tabs, menus and printing

**Todos:** [D4a](../todos/ready/D4a-windows-tabs-and-session-restore.md), [D4b](../todos/ready/D4b-native-menus.md), [D4c](../todos/ready/D4c-native-printing.md)

**Deliverable:** multi-window, tabs with drag between windows, session restore, native menus and accelerators, native printing per the D1 result.
**Accepted when:** menu items and shortcuts match the shared map (test); a 50-page document prints with page ranges on each OS (manual check recorded).

### D5. Update service

**Todos:** [D5a](../todos/ready/D5a-update-worker-and-adr-0014.md), [D5b](../todos/ready/D5b-updater-in-app.md)

**Deliverable:** ADR 0014; `apps/updates` Worker (channels, staged rollout, kill switch, 60 s cache) with tests; R2 layout and channel manifests; updater plugin wired in the app with settings.
**Accepted when:** Worker unit tests cover 204/200, rollout bucketing from the `Lamina-Install-Id` header (present, absent, malformed) and kill switch; a test asserts the install ID never reaches logs; an app on `0.0.1-test` updates to `0.0.2-test` on each OS from a staging channel; a tampered bundle is rejected.

### D6. Signing, packaging and release pipeline

**Todos:** [D6a](../todos/ready/D6a-signing-runbook-and-owner-setup.md), [D6b](../todos/ready/D6b-release-workflow-and-linux-packages.md), [D6c](../todos/ready/D6c-macos-signing-and-notarisation.md), [D6d](../todos/ready/D6d-windows-signing.md)

**Deliverable:** release workflow (tauri-action matrix) producing artefacts signed with the owner's identities as publisher: dmg (Developer ID + notarisation + stapling), MSI and NSIS (Azure Artifact Signing if the owner is eligible as an individual, otherwise an OV certificate in a cloud HSM), AppImage and deb (rpm optional); uploads to R2 with attestations; manifests updated per channel.
**Accepted when:** Gatekeeper and `debsig`/AppImage signature checks pass on clean VMs; on Windows the Authenticode signature is valid and the owner is shown as publisher. With Artifact Signing, the installer runs on a clean VM without a SmartScreen warning; with an OV certificate, a SmartScreen warning is acceptable for early releases (neither OV nor EV gets instant reputation since 2024), and the release notes document how reputation builds up; release artifacts carry GitHub attestations; a nightly channel build publishes automatically (nightlies are built in a separate `nightly` environment, are not OS-code-signed, and form their own install line: switching to beta or stable means reinstalling; see [D6b](../todos/ready/D6b-release-workflow-and-linux-packages.md)).

### D7. Crash reporting and beta

**Todos:** [D7a](../todos/ready/D7a-crash-reporting.md), [D7b](../todos/ready/D7b-settings-renderer-switch-and-download-page.md), [D7c](../todos/ready/D7c-desktop-beta-release.md)

**Deliverable:** opt-in crash and panic reporting to the chosen backend; settings page (updates, channel, crash reports, renderer backend switch for Linux); download page at `laminapdf.com/download`, on the website (`apps/site`, [website plan](website.md)); `desktop v0.1` beta on the beta channel.
**Accepted when:** a forced crash produces a scrubbed report only after consent (test asserts no paths or content); beta installs and auto-updates for at least one release cycle on all three OSes.

## 7. Risks

| Risk | Mitigation |
| --- | --- |
| Linux WebKitGTK blank windows, slow canvas, NVIDIA/Wayland issues | Canvas2D only, no GPU dependency; renderer switch; documented env workarounds; Electron-on-Linux fallback ([architecture §6](architecture.md#6-risks)) |
| IPC throughput too low for smooth zoom | D1 spike before D2; compressed tiles via custom scheme as alternative |
| Windows signing eligibility (Artifact Signing region rules) | The owner signs personally as the publisher (decided 2026-10-28, [architecture §8](architecture.md#8-open-questions-for-the-team) Q5). Artifact Signing's public-trust individual eligibility is US/Canada only: the owner checks eligibility before D6; otherwise an OV certificate in a cloud HSM |
| Losing the updater key strands users | HSM/secret-manager custody, offline backup, rotation runbook in ADR 0014 |
| Faithful printing needs per-OS code | Dedicated spike; ship `window.print()` raster path first if native lags |
| Sandbox restrictions block file access or IPC | Prototype sandbox in D2 per OS; document what is enforced |

## 8. Open questions

1. Mac App Store, Microsoft Store, Flathub: in scope, and when?
2. Crash backend: own Worker + R2, or Sentry (self-hosted or SaaS)?
3. One engine process per window, or a shared pool with per-document isolation? *Answered by D2b:* one per window; an engine uses 11–12 MiB with a small document ([ADR 0008 §2](../decisions/0008-desktop-shell.md#2-process-model-a-main-process-that-owns-files-and-a-sandboxed-engine-per-window)).
4. Default channel for new installs: stable only, or beta during 0.x?
5. Do we register as the default PDF handler on install (opt-in prompt) or leave it to the user?

## 9. To-dos

Each round's todos (one per PR, with dependencies) are linked from [§6](#6-rounds) and on the [todo board](../todos/README.md); this checklist is the summary.

Ticked by the PR that completes them.

### D1. IPC and print spikes

- [x] Build `spikes/desktop-ipc` with three tile transports and a scroll benchmark
- [x] Measure throughput and latency for 256 and 512 px tiles on macOS, Windows, Linux (x64)
- [x] Check `crossOriginIsolated` under COOP/COEP in each webview
- [x] Build `spikes/desktop-print` comparing `window.print()` and native print paths per OS
- [x] Write both results with recommendations for ADR 0008

### D2. Shell and native core

- [x] Write ADR 0008 (Tauri 2, process model, tile transport, Linux fallback)
- [x] Create `apps/desktop` and `src-tauri` as a workspace member depending on `crates/*` by path
- [x] Implement the desktop `Platform` adapter and capability flags
- [x] Implement the engine child process with restart on crash
- [x] Implement a positioned-read `ByteSource` (no mmap in the main process; see [D2b](../todos/done/D2b-engine-process.md) and ADR 0008 §2)
- [ ] Implement the chosen tile transport
- [ ] Port the reader end-to-end scenarios to WebdriverIO (`@wdio/tauri-service`): `tauri-driver` on Windows and Linux, embedded WebDriver plugin (test builds only) on macOS; run in CI on three OSes
- [ ] Record the per-OS test strategy and any macOS gap in ADR 0008

### D3. Files

- [ ] Register `.pdf` file associations in the bundle config for all three OSes
- [ ] Handle argv, `RunEvent::Opened` and single-instance forwarding
- [ ] Recent files: OS recent documents and an in-app list with "clear"
- [ ] Watch open files and reload preserving position
- [ ] Integration tests for open-from-OS on each platform

### D4. Windows, tabs, menus and printing

- [ ] Multi-window and tabs; drag a tab to a new window; session restore
- [ ] Native menus and accelerators generated from the shared shortcut map
- [ ] Implement printing per the D1 recommendation
- [ ] Record a manual print check per OS in the PR

### D5. Update service

- [ ] Write ADR 0014 (channels, rollout, signing identities with the owner as publisher, key custody and rotation)
- [ ] Create `apps/updates` Worker: `/{channel}/{target}/{arch}/{current_version}` → 204 or 200 JSON
- [ ] Implement rollout bucketing by install-ID hash (from the `Lamina-Install-Id` request header), kill switch, 60 s cache
- [ ] Define the R2 layout and channel manifest schema; tests for every branch
- [ ] Wire the Tauri updater plugin with settings (channel, disable, check now, reset install ID); send the install ID header via `UpdaterBuilder::header`
- [ ] End-to-end update test on a staging channel on three OSes; tampered bundle rejected

### D6. Signing, packaging and release pipeline

- [ ] Release workflow with tauri-action matrix (macOS arm64/x64, Windows x64/arm64, Linux x64/arm64)
- [ ] Apple Developer ID signing, notarisation and stapling
- [ ] Owner confirms Windows Artifact Signing individual eligibility (US/Canada only); otherwise obtain an OV certificate in a cloud HSM
- [ ] Windows signing via `signCommand` (Artifact Signing or the OV certificate); MSI and NSIS outputs
- [ ] Linux AppImage (updater) and deb; rpm if the matrix allows
- [ ] Upload to R2 with `.sig` files and GitHub attestations; update channel manifests
- [ ] Verify installs on clean VMs (Gatekeeper, Windows signature and publisher, SmartScreen behaviour recorded, Linux signature checks)

### D7. Crash reporting and beta

- [ ] Decide the crash backend (record in ADR 0014 or a short ADR)
- [ ] Implement opt-in crash/panic capture with scrubbing; test that no paths or content leak
- [ ] Settings page: updates, channel, crash reports, renderer backend switch
- [ ] Download page on the web app with per-OS links from R2
- [ ] Ship `desktop v0.1` on the beta channel and complete one auto-update cycle
