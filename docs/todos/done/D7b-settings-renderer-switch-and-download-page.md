# D7b — Complete settings (renderer switch) and the download page

| Field | Value |
| --- | --- |
| ID | `D7b` |
| Status | `done` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [C4b](./C4b-web-app-shell.md), [C4d](./C4d-deploy-previews-and-production.md), [W1b](./W1b-site-scaffold-and-deploys.md), [D6b](./D6b-release-workflow-and-linux-packages.md), [D7a](./D7a-crash-reporting.md) |
| Dependency notes | [D7a](../done/D7a-crash-reporting.md) (settings sections for crash reports), [D6b](../done/D6b-release-workflow-and-linux-packages.md) (artefacts and `SHA256SUMS` in R2). Cross-plan: the download page lives on the website, `apps/site`, from [W1b](../done/W1b-site-scaffold-and-deploys.md) ([website plan §4](../../plans/website.md#4-the-website)). |
| ADRs | Follows ADR 0008 (Linux fallback: renderer switch), ADR 0014 (downloads domain, channels), ADR 0011 (privacy page) |
| Branch | `feat/d7b-settings-and-download-page` |
| PR title | `feat(desktop): finish the settings page and add the download page` |
| Size | M–L (500–900 lines) |
| PR | [#647](https://github.com/laudebugs/laminapdf/pull/647) |

## Goal

Finish the desktop settings page so every user-facing control the beta needs is in one place — updates and channel (D5b), crash reports (D7a), the **renderer backend switch for Linux** (the plan's mitigation for WebKitGTK's blank windows and slow canvas), plus the general options earlier todos stored (recent files, session restore, reload on change, open in tabs vs windows, print path) — and publish a **download page** at `laminapdf.com/download` with per-OS links served from R2 so people can install the beta.

## Context

- **Design (Paper):** [S07 Settings window](../../design/paper/desktop-shell.md#s07-settings-window), [S09 Download page (web)](../../design/paper/desktop-shell.md#s09-download-page-web). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
- [Desktop plan §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) and [§7 risks](../../plans/desktop-app.md#7-risks) (renderer switch, documented env workarounds).
- [Desktop notes Q1](../../research/architecture/notes/desktop-shell.md#q1-shell-options-versions-size-memory-startup-webview-differences-licence-maturity-production-users) and [Tauri Linux graphics](https://v2.tauri.app/develop/debug/linux-graphics/): `WEBKIT_DISABLE_DMABUF_RENDERER=1`, `WEBKIT_DISABLE_COMPOSITING_MODE=1` (last resort), `__NV_DISABLE_EXPLICIT_SYNC=1`; these are **process environment variables** that must be set before WebKitGTK initialises, i.e. before the Tauri builder runs.
- D5b settings store and window; D3b/D4a/D4c settings flags.
- The website (`apps/site`, [W1b](../done/W1b-site-scaffold-and-deploys.md): static assets, `_headers`/CSP) and web plan C7 (launch, privacy page).
- ADR 0014: `downloads.laminapdf.com` custom domain, R2 layout, `SHA256SUMS`, GPG key location.

## Scope

**In scope**

- **Renderer switch (Linux)**: settings "Graphics" section with options *Automatic* (default), *Compatibility (disable DMABUF renderer)*, *Safe mode (disable compositing)*; stored in `settings.json`; applied at startup in `main.rs`/`run()` **before** `tauri::Builder` by setting the env vars for the current process (Rust 2024: `std::env::set_var` is `unsafe` — do it single-threaded at the very start with a `// SAFETY:` comment, or relaunch self with the env set). Changing it requires a restart (prompt with "Restart now", using `app.restart()`).
  - *Automatic*: detect NVIDIA proprietary driver (`/proc/driver/nvidia/version` exists) + Wayland (`WAYLAND_DISPLAY` set) and pre-apply the DMABUF workaround; log the decision.
  - Crash-loop guard: if the app exits abnormally twice within 10 s of start on Linux, next start uses *Compatibility* automatically and tells the user.
  - Command-line override `--safe-graphics` documented for users who can't reach settings.
- **Settings page completion** (`@laminapdf/ui` settings route for the `settings` window): sections General (open in tabs/windows, reopen last session, reload when file changes, remember recent files + clear), Updates (D5b), Privacy (crash reports from D7a, reset anonymous ID, link to privacy page), Graphics (Linux only; hidden elsewhere), Printing (system vs compatibility from D4c), About (version, channel, licences). Accessible (labels, keyboard, axe clean).
- Optional per plan open question 5: "Set Lamina as the default PDF app…" button that opens the OS's default-apps settings (`ms-settings:defaultapps` on Windows via the opener plugin from Rust; macOS: instructions — no API to set default without user action besides `LSSetDefaultRoleHandlerForContentType`, which we don't call silently; Linux: `xdg-mime default lamina.desktop application/pdf` only on explicit click) — only if D3a's decision says so.
- **Download page** on the website (`apps/site`): `/download`, a static Astro page, detecting the visitor's OS/arch with `navigator.userAgentData.getHighEntropyValues(['platform','architecture'])` where available, else UA parsing, offering the matching installer first and all others below: macOS dmg (arm64/x64 or universal), Windows NSIS `.exe` and `.msi` (x64/arm64), Linux AppImage, `.deb`, `.rpm` (x64/arm64); each with size, SHA-256 and links to `.asc`/`SHA256SUMS`, the GPG key, and attestation verification instructions (`gh attestation verify`); channel tabs (Stable when it exists, Beta). Data source: a small `downloads/{channel}.json` written by D6b's publish job to R2 and built into the page at deploy time, or fetched from `downloads.laminapdf.com` at runtime (decide). The website's CSP is `connect-src 'self'` ([ADR 0011 §4](../../decisions/0011-cloudflare-topology-and-privacy.md#4-security-headers-appswebpublic_headers-for-)), so a runtime fetch first needs an amendment naming that host, with CORS allowing `https://laminapdf.com`. Building the data in instead redeploys the website for each release.
- SmartScreen/Gatekeeper/Linux notes on the page (from the runbook), system requirements (macOS 13+, Windows 11, Ubuntu 22.04+/Fedora), and a link to the privacy page.

**Out of scope** (and which todo covers it)

- Shipping the beta and the update cycle: [D7c](../ongoing/D7c-desktop-beta-release.md).
- Store listings (non-goals).
- The privacy page itself (ADR 0011, web C7) — this PR adds desktop sections to it if it exists, else provides text for its owner.

## Tasks

- [x] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2` and `rust-best-practices`.
- [x] Renderer switch: early-startup env application, auto-detection, crash-loop guard, `--safe-graphics`; unit tests for the decision function (`fn graphics_env(settings, env_probe) -> Vec<(&str, &str)>`).
- [x] Settings UI sections + desktop `Platform` methods; capability for the `settings` window updated (only the commands it uses).
- [x] D6b publish job: write `downloads/{channel}.json` (version, files with size/sha256/url) to R2; R2 CORS rule for the web origin (owner action if done in the dashboard).
- [x] `apps/site` download page; Playwright test (the website's suite) with a mocked `downloads/beta.json`.
- [x] Manual check on Linux with NVIDIA + Wayland if hardware is available (else record the gap and ask beta testers). No such hardware: recorded as a gap below, for D7c to ask the beta testers.

## Acceptance criteria

- [x] On Linux, choosing *Compatibility* and restarting launches with `WEBKIT_DISABLE_DMABUF_RENDERER=1` in the process environment: the decision function's and `rerun`'s unit tests (`graphics.rs`).
- [ ] …and the manual check via `/proc/<pid>/environ`: not run, for lack of a Linux machine with a desktop session here (a gap, below).
- [x] Two abnormal exits within 10 s on Linux trigger the automatic fallback and a notice: the guard's unit tests (`Starts`), the notice once at start (`graphics_take_notice`) and in Settings › Graphics (`graphics.test.ts`).
- [x] Every setting stored by D3b, D4a, D4c, D5b and D7a is visible and changeable in the settings page (D4c stores none: it built no compatibility print path); axe reports zero serious violations in every section of Settings (the desktop `settings` scenario, on macOS, Windows and Linux in CI).
- [x] `/download` offers the right installer first on macOS arm64 (with client hints; without them, both Mac builds, Apple silicon first), Windows x64 and Linux x64 browsers, and lists every installer with the list's SHA-256 (`e2e/download.e2e.test.ts` against the fixture lists, under `wrangler dev` with the site's headers rather than the preview deploy). The publish job takes each SHA-256 from the build's `SHA256SUMS` and stops on a missing one (`downloads.test.ts`).
- [x] The download page works without JavaScript errors in Chromium, Firefox and WebKit (the same suite, one browser per `cargo xtask pdf-runtimes` run; checked by hand in Chromium at desktop and phone widths).

## Decisions and deviations

- **No `set_var`.** `std::env::set_var` is `unsafe` in Rust 2024, and rust.md allows `unsafe` only where a crate needs it. When the environment isn't the one the choice needs, `graphics::prepare` runs the app again in place with `exec`, before anything else starts. The process ID stays the same for the AppImage runtime and test drivers. The user's own variables are left alone. The app names its own in `LAMINA_GRAPHICS_ENV`, so going back to Automatic removes them at the next restart.
- **Automatic** sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` and `__NV_DISABLE_EXPLICIT_SYNC=1` on NVIDIA's own driver under Wayland, and nothing elsewhere.
- **The crash-loop guard** counts starts in `graphics-starts.json` until a start has run 10 s, quit or restarted. A second launch, which the single-instance plugin ends at once, takes its own count back. The guard switches only Automatic, to Compatibility. Safe mode and a saved Compatibility stay as they are, with no notice. End-to-end builds have no guard, since the tests stop the app within seconds.
- **`--safe-graphics` lasts until the app quits**, since `app.restart()` keeps the arguments.
- **Restart now refuses** while a window holds unsaved edits, and says why.
- **The download lists are built in, not fetched** (S09 drew a runtime fetch). The release workflow writes `downloads/<channel>.json` beside each release, and the website's production deploy reads them into `apps/site/src/data/downloads.json` every six hours and on each push. So there is no ADR 0011 amendment, no CORS rule on the buckets and no owner action, and a release shows on the page at the next deploy. The release runbook says how to deploy at once.
- **A Nightly tab** sits beside Beta and Stable, since only nightlies exist until D7c.
- **Settings' differences from S07** (Graphics as its own section, no Printing, Reset ID under Updates, a short About, Clear without Undo, no Default PDF app button) are in the area document's As built note.
- **The renderer switch's tests** are Rust unit tests. The Graphics section's view is a Vitest; whether it shows (Linux only) is the `settings` scenario's check on each OS. Opening Settings with `CmdOrCtrl+,` is the `menus` scenario's test.

## Gaps

- **NVIDIA under Wayland, by hand:** no such machine was available. D7c's beta testers are to check `/proc/<pid>/environ` with each choice.

## Post-merge checks

None.

## Tests

- Rust unit tests: graphics env decision, crash-loop guard state.
- Vitest: settings sections, conditional Graphics section, managed-policy states.
- Playwright (the website): download page OS detection, artefact list rendering, links.
- WebdriverIO (desktop): settings window opens from the menu (`CmdOrCtrl+,`), toggles persist across restart.

## Docs to update

- `docs/plans/desktop-app.md` §9 D7: tick "Settings page: updates, channel, crash reports, renderer backend switch" and "Download page on the web app with per-OS links from R2".
- `apps/desktop/README.md` (Linux graphics troubleshooting), `docs/guidelines/desktop-release.md` (downloads JSON).
- Web plan: **do not edit**; note the new `/download` route in the PR for the web plan owner.

## Risks and open questions

- **Env vars after start** have no effect on WebKitGTK — must be applied before the builder; a relaunch-based approach is safest if `set_var`'s unsafety is a concern in a multi-threaded start-up.
- **OS/arch detection** via client hints is Chromium-only; Safari/Firefox fall back to UA (arm64 Macs report Intel in UA) — show both macOS builds clearly if universal isn't used.
- **R2 CORS** for the downloads JSON must allow only the web origin.
- **Domain** settled: the download page is `https://laminapdf.com/download`, on the website ([ADR 0011 §1](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology)).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
