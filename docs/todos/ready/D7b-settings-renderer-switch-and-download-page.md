# D7b — Complete settings (renderer switch) and the download page

| Field | Value |
| --- | --- |
| ID | `D7b` |
| Status | `ready` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C4b](./C4b-web-app-shell.md), [C4d](./C4d-deploy-previews-and-production.md), [D6b](./D6b-release-workflow-and-linux-packages.md), [D7a](./D7a-crash-reporting.md) |
| Blocks | [D7c](./D7c-desktop-beta-release.md) |
| Dependency notes | [D7a](D7a-crash-reporting.md) (settings sections for crash reports), [D6b](D6b-release-workflow-and-linux-packages.md) (artefacts and `SHA256SUMS` in R2). Cross-plan: the download page lives in `apps/web` from web **C4** ([web-app §C4](../../plans/web-app.md#c4-app-shell-on-cloudflare)). |
| ADRs | Follows ADR 0008 (Linux fallback: renderer switch), ADR 0014 (downloads domain, channels), ADR 0011 (privacy page) |
| Branch | `feat/d7b-settings-and-download-page` |
| PR title | `feat(desktop): finish the settings page and add the download page` |
| Size | M–L (500–900 lines) |
| PR | — |

## Goal

Finish the desktop settings page so every user-facing control the beta needs is in one place — updates and channel (D5b), crash reports (D7a), the **renderer backend switch for Linux** (the plan's mitigation for WebKitGTK's blank windows and slow canvas), plus the general options earlier todos stored (recent files, session restore, reload on change, open in tabs vs windows, print path) — and publish a **download page** at `<domain>/download` with per-OS links served from R2 so people can install the beta.

## Context

- [Desktop plan §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) and [§7 risks](../../plans/desktop-app.md#7-risks) (renderer switch, documented env workarounds).
- [Desktop notes Q1](../../research/architecture/notes/desktop-shell.md#q1-shell-options-versions-size-memory-startup-webview-differences-licence-maturity-production-users) and [Tauri Linux graphics](https://v2.tauri.app/develop/debug/linux-graphics/): `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1` (last resort), `__NV_DISABLE_EXPLICIT_SYNC=1`; these are **process environment variables** that must be set before WebKitGTK initialises, i.e. before the Tauri builder runs.
- D5b settings store and window; D3b/D4a/D4c settings flags.
- Web plan C4 (`apps/web`, static assets, `_headers`/CSP) and C7 (launch, privacy page).
- ADR 0014: `downloads.<domain>` custom domain, R2 layout, `SHA256SUMS`, GPG key location.

## Scope

**In scope**

- **Renderer switch (Linux)**: settings "Graphics" section with options *Automatic* (default), *Compatibility (disable DMABUF renderer)*, *Safe mode (disable compositing)*; stored in `settings.json`; applied at startup in `main.rs`/`run()` **before** `tauri::Builder` by setting the env vars for the current process (Rust 2024: `std::env::set_var` is `unsafe` — do it single-threaded at the very start with a `// SAFETY:` comment, or relaunch self with the env set). Changing it requires a restart (prompt with "Restart now", using `app.restart()`).
  - *Automatic*: detect NVIDIA proprietary driver (`/proc/driver/nvidia/version` exists) + Wayland (`WAYLAND_DISPLAY` set) and pre-apply the DMABUF workaround; log the decision.
  - Crash-loop guard: if the app exits abnormally twice within 10 s of start on Linux, next start uses *Compatibility* automatically and tells the user.
  - Command-line override `--safe-graphics` documented for users who can't reach settings.
- **Settings page completion** (`@lamina/ui` settings route for the `settings` window): sections General (open in tabs/windows, reopen last session, reload when file changes, remember recent files + clear), Updates (D5b), Privacy (crash reports from D7a, reset anonymous ID, link to privacy page), Graphics (Linux only; hidden elsewhere), Printing (system vs compatibility from D4c), About (version, channel, licences). Accessible (labels, keyboard, axe clean).
- Optional per plan open question 5: "Set Lamina as the default PDF app…" button that opens the OS's default-apps settings (`ms-settings:defaultapps` on Windows via the opener plugin from Rust; macOS: instructions — no API to set default without user action besides `LSSetDefaultRoleHandlerForContentType`, which we don't call silently; Linux: `xdg-mime default lamina.desktop application/pdf` only on explicit click) — only if D3a's decision says so.
- **Download page** in `apps/web`: route `/download` (static page, part of the SPA or a static HTML asset), detecting the visitor's OS/arch with `navigator.userAgentData.getHighEntropyValues(['platform','architecture'])` where available, else UA parsing, offering the matching installer first and all others below: macOS dmg (arm64/x64 or universal), Windows NSIS `.exe` and `.msi` (x64/arm64), Linux AppImage, `.deb`, `.rpm` (x64/arm64); each with size, SHA-256 and links to `.asc`/`SHA256SUMS`, the GPG key, and attestation verification instructions (`gh attestation verify`); channel tabs (Stable when it exists, Beta). Data source: a small `downloads/{channel}.json` written by D6b's publish job to R2 and fetched from `downloads.<domain>` at runtime (CORS allowing the web origin) — or built into the page at deploy time (decide; runtime fetch avoids redeploying the site per release).
- SmartScreen/Gatekeeper/Linux notes on the page (from the runbook), system requirements (macOS 13+, Windows 11, Ubuntu 22.04+/Fedora), and a link to the privacy page.

**Out of scope** (and which todo covers it)

- Shipping the beta and the update cycle: [D7c](D7c-desktop-beta-release.md).
- Store listings (non-goals).
- The privacy page itself (ADR 0011, web C7) — this PR adds desktop sections to it if it exists, else provides text for its owner.

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2` and `rust-best-practices`.
- [ ] Renderer switch: early-startup env application, auto-detection, crash-loop guard, `--safe-graphics`; unit tests for the decision function (`fn graphics_env(settings, env_probe) -> Vec<(&str, &str)>`).
- [ ] Settings UI sections + desktop `Platform` methods; capability for the `settings` window updated (only the commands it uses).
- [ ] D6b publish job: write `downloads/{channel}.json` (version, files with size/sha256/url) to R2; R2 CORS rule for the web origin (owner action if done in the dashboard).
- [ ] `apps/web` download route + components; Playwright test (web suite) with a mocked `downloads/beta.json`.
- [ ] Manual check on Linux with NVIDIA + Wayland if hardware is available (else record the gap and ask beta testers).

## Acceptance criteria

- [ ] On Linux, choosing *Compatibility* and restarting launches with `WEBKIT_DISABLE_DMABUF_RENDERER=1` in the process environment (unit test of the decision function + manual check via `/proc/<pid>/environ`).
- [ ] Two abnormal exits within 10 s on Linux trigger the automatic fallback and a notice (unit test of the guard).
- [ ] Every setting stored by D3b, D4a, D4c, D5b and D7a is visible and changeable in the settings page; axe reports zero serious violations on the settings route.
- [ ] `<domain>/download` (preview deploy) offers the right installer first on macOS arm64, Windows x64 and Linux x64 browsers (Playwright with UA/client-hint overrides) and lists all artefacts with SHA-256 values matching `SHA256SUMS` (test against the mocked JSON).
- [ ] The download page works without JavaScript errors in Chromium, Firefox and WebKit (web Playwright suite).

## Tests

- Rust unit tests: graphics env decision, crash-loop guard state.
- Vitest: settings sections, conditional Graphics section, managed-policy states.
- Playwright (web): download page OS detection, artefact list rendering, links.
- WebdriverIO (desktop): settings window opens from the menu (`CmdOrCtrl+,`), toggles persist across restart.

## Docs to update

- `docs/plans/desktop-app.md` §9 D7: tick "Settings page: updates, channel, crash reports, renderer backend switch" and "Download page on the web app with per-OS links from R2".
- `apps/desktop/README.md` (Linux graphics troubleshooting), `docs/guidelines/desktop-release.md` (downloads JSON).
- Web plan: **do not edit**; note the new `/download` route in the PR for the web plan owner.

## Risks and open questions

- **Env vars after start** have no effect on WebKitGTK — must be applied before the builder; a relaunch-based approach is safest if `set_var`'s unsafety is a concern in a multi-threaded start-up.
- **OS/arch detection** via client hints is Chromium-only; Safari/Firefox fall back to UA (arm64 Macs report Intel in UA) — show both macOS builds clearly if universal isn't used.
- **R2 CORS** for the downloads JSON must allow only the web origin.
- **Domain** still a placeholder until the owner decides (web plan §8 Q1); the download page needs it before D7c.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
