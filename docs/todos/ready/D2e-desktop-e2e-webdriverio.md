# D2e — Desktop end-to-end tests with WebdriverIO on three OSes

| Field | Value |
| --- | --- |
| ID | `D2e` |
| Status | `ready` |
| Plan | [Desktop app §6 D2](../../plans/desktop-app.md#d2-shell-and-native-core) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D2c](./D2c-engine-sandbox.md), [D2d](./D2d-tile-transport-and-first-document.md) |
| Blocks | [D3a](./D3a-open-from-os.md), [H1](./H1-auth-spike.md) |
| ADRs | Completes ADR 0008 §test strategy. Follows [0018](../../decisions/0018-test-strategy.md) (test lanes) |
| Branch | `test/d2e-desktop-e2e` |
| PR title | `test(desktop): run reader e2e scenarios with WebdriverIO on macOS, Windows and Linux` |
| Size | M–L (500–1,000 lines: config, page objects, scenarios, CI) |
| PR | — |

## Goal

Close Round D2's acceptance bar: the reader's end-to-end scenarios run against the **desktop build** on all three OSes in CI, including killing the engine and opening the 1 GB fixture. Playwright cannot drive Tauri webviews, so desktop uses WebdriverIO with `@wdio/tauri-service`: `tauri-driver` on Windows and Linux, and the service's embedded WebDriver server (`tauri-plugin-wdio-webdriver`, compiled into test builds only behind a Cargo feature) on macOS, where Apple provides no WKWebView driver. The web build's Playwright suite keeps running on all three OSes unchanged. After this PR every later D todo adds its scenarios here.

## Context

- [Desktop plan §6 D2 "Accepted when"](../../plans/desktop-app.md#d2-shell-and-native-core): the exact test strategy and the macOS fallback (web-build Playwright + recorded manual smoke checklist, gap recorded in ADR 0008).
- [Tauri WebDriver docs](https://v2.tauri.app/develop/tests/webdriver/): `tauri-driver` (`cargo install tauri-driver --locked`), Linux needs `WebKitWebDriver` (`webkit2gtk-driver` package) and a display (`xvfb-run`), Windows needs `msedgedriver` matching the installed WebView2 runtime (e.g. via `msedgedriver-tool`); macOS is unsupported by `tauri-driver`.
- `@wdio/tauri-service` docs (WebdriverIO): service options, `tauri:options.application` capability, embedded driver mode via `tauri-plugin-wdio-webdriver`.
- Web plan [C3](../../plans/web-app.md#c3-viewer-core)/[C4](../../plans/web-app.md#c4-app-shell-on-cloudflare) Playwright scenarios: the scenario list to port (open, scroll, zoom, password, errors).
- [ADR 0018](../../decisions/0018-test-strategy.md): which lane runs what (full lane on Linux/macOS/Windows).
- D2b's `debug_kill_engine` (feature `e2e`) and the 1 GB fixture generator.

## Scope

**In scope**

- `apps/desktop/e2e/` (TypeScript, ESM): `wdio.conf.ts` with the `tauri` service; capabilities per OS; page objects for the shell (toolbar, page indicator, zoom, password prompt, toasts); scenarios:
  1. App starts and shows the empty state within the cold-start budget (< 300 ms to an interactive window on every OS, owner decision 2026-10-29; logged as a metric; asserted with headroom on CI runners, e.g. ≤ 5 s).
  2. Open fixture (test-only command `e2e_open_path` behind the `e2e` feature, because native dialogs can't be automated) → first page painted (pixel probe: canvas not blank).
  3. Scroll to the last page of the 1,000-page fixture; page indicator correct.
  4. Zoom in/out with anchor preserved (page indicator and scroll offset stable).
  5. Encrypted fixture: password prompt, wrong then right password; then `debug_kill_engine` → the page re-renders with **no** second password prompt.
  6. Malformed fixture: repair notice.
  7. Kill engine via `debug_kill_engine` → toast → page re-renders.
  8. 1 GB generated fixture opens; first page painted within 1.5 s (measured in-app via `performance.now()` markers exposed to the test; assert with CI headroom, report raw).
- Cargo feature `e2e` on `lamina-desktop` enabling `tauri-plugin-wdio-webdriver` (macOS) and the test-only commands; a separate capability file `capabilities/e2e.json` included only when the feature is on (via `tauri.e2e.conf.json` merged with `--config`), so release builds have neither the plugin nor the permissions.
- CI job `desktop-e2e` (full lane, path-filtered) on `ubuntu-22.04` (xvfb), `windows-2022`, `macos-14`; runs `cargo xtask desktop-sidecar` (engine + PDFium) and builds `pnpm tauri build --debug --no-bundle --features e2e --config src-tauri/tauri.bundle.conf.json --config src-tauri/tauri.e2e.conf.json` (the bundle config carries the sidecar and PDFium, which D2b keeps out of the base config); installs `tauri-driver` (pinned version in `tools/xtask/src/doctor.rs`), `webkit2gtk-driver`, and the matching `msedgedriver`; uploads screenshots/logs on failure.
- A release-build guard: CI asserts the release binary does not contain the embedded WebDriver plugin or `e2e_` command names.
- macOS fallback documented: if the embedded driver is flaky (> 2 % failure over 50 runs in CI), the macOS job is reduced to the web Playwright run plus `apps/desktop/e2e/MANUAL-SMOKE.md` checklist, and ADR 0008 records the gap.
- ADR 0008 §test strategy finalised.

**Out of scope** (and which todo covers it)

- Open-from-OS tests (double-click, single-instance): [D3a](D3a-open-from-os.md) adds them.
- Update e2e: [D5b](D5b-updater-in-app.md). Installer tests on clean VMs: [D6b](D6b-release-workflow-and-linux-packages.md)–[D6d](D6d-windows-signing.md).
- Visual regression of renders: covered by C2's golden render tests.

## Tasks

- [ ] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2`.
- [ ] Add devDeps `@wdio/cli`, `@wdio/local-runner`, `@wdio/mocha-framework` (or Jasmine), `@wdio/spec-reporter`, `@wdio/tauri-service` (pinned via `catalog:`; licence check: MIT).
- [ ] Add `tauri-plugin-wdio-webdriver` as an optional dependency of `lamina-desktop` under feature `e2e` (licence check), registered only with `#[cfg(feature = "e2e")]`.
- [ ] Test-only commands under `e2e`: `e2e_open_path(path)` (path must be under the repo's `fixtures/` or `target/fixtures/` — checked), `e2e_metrics()` (first-paint timestamps), reuse `debug_kill_engine`.
- [ ] `tauri.e2e.conf.json` setting `app.security.capabilities` to the **full** base list plus `"e2e"` (today `["main", "e2e"]`; later `["main", "settings", "e2e"]` etc.). Tauri merges `--config` files as a JSON merge patch, which replaces arrays wholesale, so listing only `"e2e"` would silently drop `main`. D2a's capability guard test checks the e2e list equals the base list plus `"e2e"`, so a later todo that adds a capability must update both files. `capabilities/e2e.json` is scoped to `"windows": ["main", "doc-*"]` and grants only the `allow-e2e-*` / `allow-debug-kill-engine` permissions.
- [ ] Page objects and the eight scenarios; deterministic waits (no sleeps): wait for `data-state="painted"` attributes the viewer sets in test builds.
- [ ] CI job and doctor pins; cache `tauri-driver` install.
- [ ] Run the macOS job 50 times (workflow_dispatch loop) and record flakiness in the PR; apply the fallback rule if needed.
- [ ] Update ADR 0008 and the plan.

## Acceptance criteria

- [ ] `desktop-e2e` is green on Linux, Windows and macOS (or macOS uses the documented fallback with the gap in ADR 0008).
- [ ] All eight scenarios pass on each OS that runs WebdriverIO; the 1 GB scenario reports its first-page time.
- [ ] The web build's Playwright suite still runs on all three OSes (no change, verified in the PR).
- [ ] The release build contains no e2e plugin/commands (CI guard green).
- [ ] `pnpm --filter @lamina/desktop e2e` runs locally on Linux and Windows with documented prerequisites.

## Post-merge checks

None.

## Tests

This PR is tests: WebdriverIO scenarios under `apps/desktop/e2e/specs/`, fixtures from `fixtures/` plus the generated 1 GB file (generated in CI via `cargo xtask fixtures large-pdf`, cached by hash).

## Docs to update

- ADR 0008 §test strategy (final), including any macOS gap.
- `docs/plans/desktop-app.md` §9 D2: tick "Port the reader end-to-end scenarios to WebdriverIO…" and "Record the per-OS test strategy and any macOS gap in ADR 0008".
- `CLAUDE.md` commands (`pnpm --filter @lamina/desktop e2e`), `apps/desktop/README.md`, `tools/xtask` doctor pins.
- `docs/decisions/0018-test-strategy.md`: if the lanes change, a short amendment note (desktop e2e in the full lane).

## Risks and open questions

- **WebView2/msedgedriver version skew** on GitHub runners breaks Windows runs; pin via the runtime version at job start.
- **Embedded macOS driver maturity** — fallback rule above.
- **xvfb + WebKitGTK** may need `WEBKIT_DISABLE_DMABUF_RENDERER=1` on CI; document it.
- **CI time**: three debug Tauri builds + 1 GB fixture generation; cache aggressively and keep the job path-filtered.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
