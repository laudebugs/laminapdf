# D5b — Updater in the app: settings, install-ID header, managed policy, staging e2e

| Field | Value |
| --- | --- |
| ID | `D5b` |
| Status | `ready` |
| Plan | [Desktop app §6 D5](../../plans/desktop-app.md#d5-update-service) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D4b](./D4b-native-menus.md), [D5a](./D5a-update-worker-and-adr-0014.md) |
| Blocks | [D6b](./D6b-release-workflow-and-linux-packages.md), [D7a](./D7a-crash-reporting.md), [H2](./H2-accounts-and-login.md) |
| ADRs | Implements ADR 0014 (client side). Follows ADR 0008 (security baseline), ADR 0011 (privacy) |
| Branch | `feat/d5b-desktop-updater` |
| PR title | `feat(desktop): wire the updater with channels, settings and the install-ID header` |
| Size | L (600–1,100 lines incl. staging workflow) |
| PR | — |

## Goal

Make the desktop app update itself safely from the D5a Worker: check at start and every 24 h on the user's channel, download in the background, install on quit or on "Restart to update", verify the Tauri updater signature (a tampered bundle must be rejected), and let users (and administrators) control it — channel, disable updates, check now, reset install ID. Requirement 8 of the [desktop plan](../../plans/desktop-app.md#functional). D5 is accepted when an app on `0.0.1-test` updates to `0.0.2-test` on each OS from a staging channel and a tampered bundle is rejected.

## Context

- ADR 0014 (D5a): endpoint shape, channels, header rules, managed policy keys, key custody.
- [Desktop plan §5](../../plans/desktop-app.md#5-design-summary): "send the ID in a custom request header (`Lamina-Install-Id`) set with the updater plugin's `UpdaterBuilder::header` in Rust" — never in the URL (the URL only substitutes `{{target}}`, `{{arch}}`, `{{current_version}}`).
- Tauri [updater plugin](https://v2.tauri.app/plugin/updater/): `tauri-plugin-updater`; config `plugins.updater.pubkey` and `endpoints`; `bundle.createUpdaterArtifacts: true` (v2 artefacts: `.app.tar.gz` + `.sig` on macOS, `.AppImage` + `.sig` on Linux, NSIS `-setup.exe`/`.msi` + `.sig` on Windows); `plugins.updater.windows.installMode` (`passive` default, `basicUi`, `quiet`); Windows exits the app during install; `Update::download` + `Update::install` split for install-on-quit; requires HTTPS endpoints in production (`dangerousInsecureTransportProtocol` only for local tests).
- `tauri-plugin-process` for relaunch (`app.restart()` is available in Rust without the plugin; prefer Rust, no webview permission).
- Install ID privacy rules (plan §5): UUID v4 on first run, stored only in app config, resettable, never linked to crash reports.

## Scope

**In scope**

- `tauri-plugin-updater` registered (Rust-side only; **no `updater:*` permission granted to the webview** — all calls go through app commands so the header and channel logic can't be bypassed).
- `tauri.conf.json`: `bundle.createUpdaterArtifacts: true`; `plugins.updater.pubkey` (the **public** key from D5a's key ceremony — owner provides); `plugins.updater.endpoints` left as a default stable URL, overridden at runtime by `app.updater_builder().endpoints(vec![channel_url])`; `plugins.updater.windows.installMode: "passive"`.
- `src-tauri/src/update/`:
  - `settings.rs`: `UpdateSettings { enabled, channel, last_check, install_id }` persisted in `app_config_dir()/settings.json` (shared settings store; atomic writes); install ID created lazily with `uuid::Uuid::new_v4()`.
  - `policy.rs`: managed policy — Windows `HKLM\Software\Policies\Lamina` (`UpdateChannel`, `DisableUpdates`), macOS managed preferences (`CFPreferencesCopyAppValue` for the bundle identifier domain, set by MDM configuration profiles), Linux `/etc/lamina/policy.json`; policy overrides user settings and the UI shows "Managed by your organisation".
  - `service.rs`: scheduler (on start after 10 s, then every 24 h; jitter ± 30 min), `check()` building the updater with `.header("Lamina-Install-Id", id)?` and the channel endpoint `https://updates.<domain>/{channel}/{{target}}/{{arch}}/{{current_version}}`, `.timeout(30s)`; background `download()` with progress events to the settings window; install on `RunEvent::ExitRequested`/explicit "Restart to update" (macOS/Linux: `install` then `app.restart()`; Windows: `install` exits the app — flush session first).
  - Offline/air-gapped behaviour: failures are silent (logged at `debug`), retried next interval; "Check now" reports errors to the user.
- Commands (manifest + capability, settings window only): `update_status`, `update_check_now`, `update_set_enabled`, `update_set_channel`, `update_reset_install_id`, `update_restart_to_install`. The main document windows get only `update_status` (for a subtle "update ready" indicator).
- Settings window (first real content of the D4b placeholder): "Updates" section — automatic updates toggle, channel (stable/beta/nightly; nightly behind an "advanced" disclosure), "Check now", status (up to date / downloading x % / ready — restart), "Reset anonymous ID" with an explanation linking to the privacy page.
- Menu: "Check for Updates…" (macOS app menu, Help menu elsewhere) enabled and wired.
- Staging e2e workflow `.github/workflows/desktop-update-e2e.yml` (`workflow_dispatch` + nightly): builds `0.0.1-test` and `0.0.2-test` for the three OSes with updater artefacts signed by a **test** updater key (separate from production; stored as a non-release secret), uploads 0.0.2-test to the staging R2 prefix and seeds the `staging` channel manifest, installs 0.0.1-test on the runner, launches it with a test-only flag (feature `e2e`) that forces the `staging` channel and an immediate check, waits for install + relaunch, and asserts the running version is `0.0.2-test`. A second run serves a tampered bundle (one byte flipped, original `.sig`) and asserts the updater rejects it and the app stays on `0.0.1-test`.

**Out of scope** (and which todo covers it)

- Production signing of installers (Apple/Windows/Linux) and the release workflow: [D6b](D6b-release-workflow-and-linux-packages.md)–[D6d](D6d-windows-signing.md). The staging e2e uses OS-unsigned (or ad-hoc signed) bundles; updater signatures are independent of OS code signing.
- Crash reporting settings: [D7a](D7a-crash-reporting.md). Renderer switch: [D7b](D7b-settings-renderer-switch-and-download-page.md).
- Mac App Store/Microsoft Store builds without the updater (non-goal).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2` and `rust-best-practices`.
- [ ] Add `tauri-plugin-updater` and `uuid` (licence checks); register the plugin; config keys above.
- [ ] Implement settings/policy/service modules with unit tests; ensure no lock is held across `await`.
- [ ] Commands + capabilities (separate `capabilities/settings.json` for the `settings` window; extend the forbidden-permissions guard from D2a to forbid `updater:` in any capability).
- [ ] Settings UI in `@lamina/ui` (desktop-only section rendered when `Platform.hasUpdater`).
- [ ] Menu wiring.
- [ ] Staging e2e workflow (above) with a test updater keypair (`pnpm tauri signer generate`) stored as `TAURI_TEST_SIGNING_PRIVATE_KEY`; document that this key is **not** the production key.
- [ ] Privacy: update the privacy page draft (owned by web C7/ADR 0011) with the update endpoint data and the reset option — via a PR note if the page doesn't exist yet.

## Acceptance criteria

- [ ] On macOS, Windows and Linux (AppImage), an installed `0.0.1-test` updates to `0.0.2-test` from the `staging` channel without user interaction beyond what `installMode: passive` shows (CI workflow green on all three OSes; run linked in the PR).
- [ ] A tampered bundle is rejected on each OS (signature error logged; app remains on 0.0.1-test) — same workflow.
- [ ] The request to the update endpoint carries `Lamina-Install-Id` as a header and not in the URL (unit test of the builder config + a Worker-side assertion in the staging run).
- [ ] Disabling updates stops all update network requests (unit test on the scheduler; manual check with a proxy).
- [ ] Managed policy overrides user settings on each OS (unit tests for the parsers; manual check on Windows registry and macOS profile).
- [ ] "Reset anonymous ID" generates a new UUID and the next check uses it (unit test).
- [ ] The webview has no `updater:*` permission (capability guard test).

## Tests

- Rust unit tests: settings persistence, policy parsing per OS (with fakes), scheduler timing (time abstraction), endpoint URL construction per channel, header presence.
- Vitest: settings UI states (up to date, downloading, ready, error, managed).
- CI: `desktop-update-e2e` workflow (above).

## Docs to update

- `docs/plans/desktop-app.md` §9 D5: tick "Wire the Tauri updater plugin with settings…" and "End-to-end update test on a staging channel on three OSes; tampered bundle rejected".
- ADR 0014: client-side details (scheduler, policy keys) if they changed during implementation.
- `apps/desktop/README.md`: testing updates locally against a local Worker (`wrangler dev`, `dangerousInsecureTransportProtocol` only in debug).

## Risks and open questions

- **macOS updates of unsigned/ad-hoc test bundles**: Gatekeeper/quarantine may interfere on runners; the staging workflow may need `xattr -dr com.apple.quarantine`; production uses D6c's signed bundles.
- **Linux deb/rpm installs can't self-update** (Tauri updates AppImage only): the app must detect a non-AppImage install (`APPIMAGE` env var absent) and show "Updates are managed by your package manager" instead of checking — implement here.
- **Windows per-machine MSI vs per-user NSIS** affects whether the updater can install without elevation; align with D6d's install-scope decision.
- **Install-on-quit** while documents are open on Windows (installer kills the app): flush session (D4a) before `install`.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
