# D2a — ADR 0008 and the `apps/desktop` Tauri 2 scaffold

| Field | Value |
| --- | --- |
| ID | `D2a` |
| Status | `ready` |
| Plan | [Desktop app §6 D2](../../plans/desktop-app.md#d2-shell-and-native-core) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C2a](../done/C2a-render-adrs.md), [C4a](./C4a-ui-chrome-and-platform.md), [D1a](./D1a-desktop-ipc-spike.md), [D1b](./D1b-desktop-print-spike.md), [U6b](./U6b-self-hosted-fonts.md) |
| Blocks | [D2b](./D2b-engine-process.md), [D2d](./D2d-tile-transport-and-first-document.md), [D5a](./D5a-update-worker-and-adr-0014.md), [D6a](./D6a-signing-runbook-and-owner-setup.md) |
| Dependency notes | [D1a](D1a-desktop-ipc-spike.md), [D1b](D1b-desktop-print-spike.md); web plan **C4** (`@lamina/ui` with the `Platform` interface, [web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare)) and therefore ADR 0010 from web **C2** ([web-app §C2](../../plans/web-app.md#c2-render-backend-and-adrs)) |
| ADRs | **Writes ADR 0008** (desktop shell). Implements ADR 0010 (Platform adapter) for desktop. Follows [0001](../../decisions/0001-core-language-and-licence.md), [0015](../../decisions/0015-monorepo-layout-and-tooling.md), [0018](../../decisions/0018-test-strategy.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/d2a-desktop-scaffold` |
| PR title | `feat(desktop): add ADR 0008 and the Tauri 2 app scaffold` |
| Size | L (600–1,500 lines incl. ADR, config, generated-schema exclusions) |
| PR | — |

## Goal

Record the desktop shell decision (ADR 0008) from the D1 spike results and land a buildable, CI-checked `apps/desktop`: a Tauri 2 app whose frontend is `@lamina/ui` with a desktop `Platform` implementation and whose `src-tauri` crate is added to the Cargo workspace `members`. It opens one window showing the reader chrome with no document yet. Everything later in Round D (engine process, tile transport, file handling, menus, updater, signing) builds on the security baseline (CSP, capabilities, app-command permissions) fixed here, so it must be right from the first PR.

## Context

- [Desktop plan §4–§5](../../plans/desktop-app.md#5-design-summary): layout, process model, capability flags (`canWriteInPlace: true`, `canWatchFiles: true`, `hasNativePrint: true`, `multiWindow: true`), security requirements (no `shell` or broad `fs` from the webview, PDF JavaScript never executed).
- [Architecture §3](../../plans/architecture.md#3-decisions-to-record-adrs): ADR 0008 scope — Tauri 2, process model, tile transport, Linux fallback. [§6 risks](../../plans/architecture.md#6-risks): Linux WebKitGTK and the Electron fallback order.
- [Report: Tauri section](../../research/architecture/app-and-package-architecture.md#tauri-2-is-the-desktop-shell-with-electron-held-in-reserve-for-linux) and [desktop notes Q1–Q4](../../research/architecture/notes/desktop-shell.md): evidence for the decision and its alternatives.
- D1a and D1b READMEs (`spikes/desktop-ipc/`, `spikes/desktop-print/`): numbers ADR 0008 cites.
- [Code organisation §1](../../guidelines/code-organization.md#1-top-level-layout) and [ADR 0015](../../decisions/0015-monorepo-layout-and-tooling.md): `apps/desktop` is a pnpm package and its `src-tauri` crate is a Cargo workspace member; `publish = false`; layer `app`. The root `Cargo.toml` currently has `members = ["crates/*", "tools/xtask"]`, so **this PR adds `"apps/*/src-tauri"`** (the glob ADR 0015 describes).
- [Rust guidelines](../../guidelines/rust.md) and [TypeScript guidelines](../../guidelines/typescript.md): edition 2024, workspace lints, no `unwrap`/`expect` (the Tauri template's `.expect("error while running tauri application")` must be replaced), named exports, Biome.
- Web plan C4 `@lamina/ui` and ADR 0010 `Platform` interface; web plan [C3](../../plans/web-app.md#c3-viewer-core) `@lamina/viewer`.
- Tauri docs: [configuration reference](https://v2.tauri.app/reference/config/), [capabilities](https://v2.tauri.app/security/capabilities/), [permissions / app manifest](https://v2.tauri.app/security/permissions/), [CSP](https://v2.tauri.app/security/csp/), [HTTP headers](https://v2.tauri.app/security/http-headers/).

## Scope

**In scope**

- `docs/decisions/0008-desktop-shell.md` (status **Accepted** when this PR merges), covering:
  1. Tauri 2 (pinned minor), direct distribution (no MAS), alternatives considered (Electron, native UIs, Rust-native UI) with the research evidence.
  2. **Process model**: main process (windows, menus, dialogs, updater, file I/O: mmap and watching) + sandboxed **engine process** that parses and renders untrusted bytes; granularity (one engine per window vs a pool — the plan's open question 3; recommend one per window to start, revisit with memory numbers from D2b); restart policy; who owns file handles (main) and how bytes reach the engine (range requests over the engine pipe — confirmed or revised by D2b measurements).
  3. **Engine binary and protocol crate**: recommended as a separate bin crate `crates/lamina-desktop-engine` (layer `app`, `publish = false`) shipped as a Tauri sidecar via `bundle.externalBin` (Tauri handles per-target naming and macOS nested signing). The main↔engine **protocol types and framing live in their own crate `crates/lamina-desktop-protocol`** (layer `app`, `publish = false`; depends only on `serde`, `postcard`, `thiserror`, `zeroize` — no `lamina-render`, PDFium or parser dependency), used by both `src-tauri` and the engine, so the main process never links the render stack (D2b checks this with `cargo tree`). Alternative recorded: re-exec of the main binary with `--engine` (Chrome-style) — rejected because the engine would load the webview stack.
  4. **Tile transport**: the D1a recommendation (primary + fallback), tile size, compression, and COOP/COEP findings per OS.
  5. **Sandboxing scope per OS** (intent; D2c records what is enforced): macOS (App Sandbox entitlement on the helper or `sandbox_init` profile), Windows (restricted token + job object, AppContainer if feasible), Linux (seccomp-bpf + Landlock where the kernel supports it, else no-new-privs + namespaces).
  6. **Security baseline**: CSP, capability files, app-command allow-list via app manifest, no `shell`/`fs`/`http` plugins exposed to the webview, PDF JavaScript never executed, external links only via a confirmed `opener` call from Rust.
  7. **Linux fallback**: Canvas2D-only paint, renderer switch (env workarounds), Electron-on-Linux as the documented last resort; triggers for revisiting.
  8. **Test strategy per OS** (filled in by D2e): WebdriverIO + `@wdio/tauri-service`; `tauri-driver` on Windows/Linux; embedded WebDriver plugin on macOS behind a Cargo feature.
  9. Printing: link to D1b's recommendation (implemented in D4c).
- `apps/desktop/` pnpm package `@lamina/desktop` (`"private": true`): Vite 8 + React 19 frontend that mounts `@lamina/ui`'s shell with a **desktop `Platform` implementation** (`src/platform/desktop.ts`) reporting `canWriteInPlace: true, canWatchFiles: true, hasNativePrint: true, multiWindow: true` (the methods that need later todos return a typed "not yet supported" error, never `if (isTauri)` branches in shared UI).
- `apps/desktop/src-tauri/`: crate `lamina-desktop` (lib `lamina_desktop_lib`, bin `lamina-desktop`), edition 2024, `[lints] workspace = true`, `publish = false`, `[package.metadata.lamina] layer = "app"`; thin `main.rs`; `lib.rs` with `pub fn run() -> Result<(), DesktopError>` (no `expect`), `#[cfg_attr(mobile, tauri::mobile_entry_point)]` not needed (desktop only — state it).
- `tauri.conf.json` baseline (keys below), `capabilities/main.json`, `build.rs` with an **app manifest** so every app command needs an explicit `allow-*` permission.
- Plugins registered now: `tauri-plugin-dialog` (Rust-side use only; no `dialog:*` permission granted to the webview), `tauri-plugin-log` (file + stdout, no document paths at `info` level), `tauri-plugin-opener` (Rust-side only).
- One command: `app_info` → `{ version, os, arch, webview_version }` to prove the IPC + permission plumbing end to end.
- CI: a `desktop` job in the full lane building `apps/desktop` on `ubuntu-22.04`, `macos-14`, `windows-2022` (`pnpm --filter @lamina/desktop tauri build --debug --no-bundle`), plus clippy/test of `lamina-desktop` in `cargo xtask check rust` (Linux needs `libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev` installed in CI).
- `cargo xtask` updates: `check ts` covers `apps/desktop`; `doctor` lists the Linux system packages; `portability` keeps excluding app-layer crates.
- `cargo deny` passes with Tauri's dependency tree (add licence exceptions only with justification per ADR 0019).

**Out of scope** (and which todo covers it)

- Engine process and its protocol: [D2b](D2b-engine-process.md); OS sandboxing: [D2c](D2c-engine-sandbox.md).
- Tile transport and opening a document: [D2d](D2d-tile-transport-and-first-document.md).
- WebdriverIO e2e suite: [D2e](D2e-desktop-e2e-webdriverio.md) (this PR adds only a build job).
- File associations, single-instance: [D3a](D3a-open-from-os.md). Menus: [D4b](D4b-native-menus.md). Updater: [D5b](D5b-updater-in-app.md). Bundling/signing: [D6b](D6b-release-workflow-and-linux-packages.md)–[D6d](D6d-windows-signing.md).

## Tasks

- [ ] Bundle the self-hosted Paper fonts from `@lamina/fonts` ([U6b](./U6b-self-hosted-fonts.md)) in the desktop frontend (import `fonts.css`, preload Source Sans 3 Latin 400); nothing is fetched at runtime.
- [ ] Fresh worktree `feat-d2a-desktop-scaffold`; this todo is already claimed via the claim PR (see DoD), so do not move it here. Load `tauri-v2` and `rust-best-practices`.
- [ ] Write `docs/decisions/0008-desktop-shell.md` using the ADR template in `docs/decisions/README.md`; cite D1a/D1b numbers and research sources; add it to the decisions index and the architecture §3 table.
- [ ] `apps/desktop/package.json`: deps `@lamina/ui`, `@lamina/viewer` (workspace), `@tauri-apps/api@^2` (pinned via pnpm `catalog:`), devDeps `@tauri-apps/cli@^2`, `vite`, `@vitejs/plugin-react`; scripts `dev` (`vite`), `build` (`vite build`), `tauri` (`tauri`), `typecheck`, `test` (Vitest). Add `@tauri-apps/*` to the Dependabot group (ADR 0019).
- [ ] `apps/desktop/vite.config.ts`: fixed port (`strictPort: true`, e.g. 1420), `clearScreen: false`, `envPrefix: ['VITE_', 'TAURI_ENV_*']`, `build.target` per `TAURI_ENV_PLATFORM` (`safari15`/`chrome105`), no sourcemaps in release.
- [ ] `apps/desktop/src/platform/desktop.ts`: implement the ADR 0010 `Platform` interface; `isDesktop()` detection uses `isTauri()` from `@tauri-apps/api/core` once at startup, in the app entry only.
- [ ] Root `Cargo.toml`: add `"apps/*/src-tauri"` to `[workspace] members` (keep `exclude = ["spikes", "fuzz"]`); confirm `cargo metadata --no-deps` lists `lamina-desktop` and that `cargo xtask check rust` (fmt, clippy, test) now covers it.
- [ ] `apps/desktop/src-tauri/Cargo.toml`: `tauri = { workspace = true, features = [] }` (add `tauri`, `tauri-build`, plugins to `[workspace.dependencies]` with licence comments), `serde`, `serde_json`, `thiserror`, `tracing`; `[build-dependencies] tauri-build`.
- [ ] `apps/desktop/src-tauri/build.rs`: `tauri_build::try_build(tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&["app_info"])))` returning an error instead of panicking where possible.
- [ ] `apps/desktop/src-tauri/tauri.conf.json`:
  - `productName: "Lamina"`, `version` read from `../package.json`, `identifier` (reverse-DNS of the owner's domain, e.g. `app.lamina.desktop` — **owner confirms**; it can never change without breaking updates, file associations and macOS permissions).
  - `build`: `devUrl: "http://localhost:1420"`, `frontendDist: "../dist"`, `beforeDevCommand: "pnpm dev"`, `beforeBuildCommand: "pnpm build"`.
  - `app.windows`: `[]` with windows created in Rust (`WebviewWindowBuilder`, label `main`) so labels are under our control for multi-window (D4a); `app.withGlobalTauri: false`.
  - `app.security.csp`: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob: data: lamina: http://lamina.localhost; connect-src 'self' ipc: http://ipc.localhost lamina: http://lamina.localhost; worker-src 'self' blob:; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'` (adjust for D1a's transport; keep `freezePrototype: true`; `dangerousDisableAssetCspModification: false`).
  - `app.security.headers` with COOP/COEP only if D1a found them working and needed; otherwise omit and note in ADR 0008.
  - `app.security.capabilities: ["main"]` — an **explicit list**. Tauri v2 enables every file in `./capabilities/` only when this list is unset or empty; once it is set, only the listed identifiers are used ([capabilities](https://v2.tauri.app/security/capabilities/), [config reference](https://v2.tauri.app/reference/config/)). We keep it explicit so a test-only capability file (D2e's `e2e.json`) can live in `capabilities/` without being enabled in release builds. Rule for every later todo: **adding a capability file means adding its identifier to this list in the same PR** (e.g. `settings` in D4b) — enforced by the guard test below.
  - `bundle.active: true`, `targets: []` for now (D6b sets them), `icon` set (placeholder icons generated with `pnpm tauri icon`), `category: "Productivity"`, `copyright`, `licenseFile` pointing to the repo licence.
- [ ] `apps/desktop/src-tauri/capabilities/main.json`: `{"identifier":"main","windows":["main","doc-*"],"platforms":["macOS","windows","linux"],"permissions":["core:default","allow-app-info"]}` — `core:default` reviewed and trimmed if it grants anything the UI does not need (record in ADR 0008).
- [ ] `apps/desktop/src-tauri/src/lib.rs`: `DesktopError` (`thiserror`, `impl serde::Serialize` as a tagged `{ kind, message }` so the UI can map errors), builder with plugins and `invoke_handler(tauri::generate_handler![commands::app_info])`, `setup` creating the `main` window, `run()` returns `Result`; `main.rs` logs and exits non-zero on error.
- [ ] Generated `gen/schemas` committed or ignored consistently (Tauri regenerates them; ignore and document).
- [ ] CI job `desktop-build` (matrix) in `.github/workflows/ci.yml` gated by path filter `apps/desktop/**`, `packages/**`, `crates/**`; actions pinned by SHA (ADR 0019); cache Cargo with the existing setup.
- [ ] `apps/desktop/README.md`: how to run (`pnpm --filter @lamina/desktop tauri dev`), Linux prerequisites, layout.
- [ ] Update `CLAUDE.md` layout tree (`apps/desktop`) and commands; `docs/decisions/README.md` index.

## Acceptance criteria

- [ ] ADR 0008 exists, is **Accepted**, cites D1a and D1b results, and records every item listed in Scope.
- [ ] `pnpm --filter @lamina/desktop tauri dev` opens a window titled "Lamina" showing the `@lamina/ui` shell on macOS, Windows and Linux (screenshots in the PR).
- [ ] The `desktop-build` CI job is green on all three OSes; `cargo xtask check` passes including clippy `-D warnings` on `lamina-desktop`.
- [ ] Invoking any command not in the app manifest from the devtools console is rejected with a permission error (checked manually, noted in the PR); `invoke('plugin:dialog|open')` from the webview is rejected.
- [ ] The root `Cargo.toml` `members` includes `"apps/*/src-tauri"`; `cargo metadata --no-deps --format-version 1` lists `lamina-desktop`.
- [ ] The capability guard test passes and fails on purpose-built bad inputs (forbidden prefix, unlisted capability file, `e2e` in the base list, an e2e override that drops a base entry).
- [ ] No `unwrap`/`expect`/`panic!` in `src-tauri/src` (clippy lints from the workspace enforce it).
- [ ] `cargo deny check` passes; any new licence exception is justified in `deny.toml`.

## Post-merge checks

None.

## Tests

- Rust unit tests in `src-tauri/src/commands.rs` for `app_info` serialisation and `DesktopError` serialisation shape (`#[test] fn desktop_error_should_serialise_as_kind_and_message()`).
- Vitest unit tests for `platform/desktop.ts` capability flags (mock `@tauri-apps/api/core` with `mockIPC` from `@tauri-apps/api/mocks`).
- A capability guard test (a small Rust test or xtask check) that stays for all later todos. It parses `capabilities/*.json` and `tauri.conf.json` and fails if:
  - a forbidden permission prefix (`shell:`, `fs:`, `http:`, `dialog:`, `process:`) appears in any capability;
  - a capability file other than `e2e.json` is missing from `app.security.capabilities` (a file nobody enabled is a silent no-op), or the list names an identifier with no file;
  - `e2e` is listed in `tauri.conf.json` (it may only be enabled by `tauri.e2e.conf.json`, D2e);
  - `tauri.e2e.conf.json` exists and its `app.security.capabilities` is not exactly the base list plus `"e2e"` — `--config` files are merged as a JSON merge patch, which **replaces arrays** rather than appending, so the e2e override must repeat the full list.

## Docs to update

- `docs/decisions/0008-desktop-shell.md` (new), `docs/decisions/README.md` index, `docs/plans/architecture.md` §3 row (link + Accepted).
- `docs/plans/desktop-app.md` §9 D2: tick "Write ADR 0008", "Create `apps/desktop` and `src-tauri`…", "Implement the desktop `Platform` adapter…".
- `CLAUDE.md` layout and commands; `apps/desktop/README.md` (new).

## Risks and open questions

- **Bundle identifier and product name** are effectively permanent; the owner decides (ties to the domain question in web-app §8 Q1). Block merge until confirmed.
- **CI cost**: three-OS desktop builds are slow; keep them in the full lane with path filters, not the fast lane.
- **Linux build deps** on `ubuntu-22.04` (WebKitGTK 4.1); Ubuntu 22.04 is the oldest supported target, so build there for glibc compatibility.
- **CSP and the transport**: if D1a picked a custom scheme, the CSP must allow it on each OS's URL form; verify on Windows (`http://lamina.localhost`).
- ADR 0010's `Platform` interface may lack desktop-only methods (watching, print, multi-window); extend it in `@lamina/ui` in this PR with web stubs, coordinating with whoever owns C-round code.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
