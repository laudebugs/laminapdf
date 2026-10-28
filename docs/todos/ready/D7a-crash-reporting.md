# D7a — Opt-in crash and panic reporting with scrubbing

| Field | Value |
| --- | --- |
| ID | `D7a` |
| Status | `ready` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D5a](./D5a-update-worker-and-adr-0014.md), [D5b](./D5b-updater-in-app.md), [D6b](./D6b-release-workflow-and-linux-packages.md) |
| Blocks | [D7b](./D7b-settings-renderer-switch-and-download-page.md), [D7c](./D7c-desktop-beta-release.md) |
| ADRs | Records the crash-backend decision (plan open question 2) as an amendment to ADR 0014 or a short new ADR (next free number). Follows ADR 0011 (privacy/retention, web C2), ADR 0008 (engine process) |
| Branch | `feat/d7a-crash-reporting` |
| PR title | `feat(desktop): add opt-in, scrubbed crash and panic reporting` |
| Size | L (700–1,300 lines across app, engine and Worker) |
| PR | — |

## Goal

When lamina or its engine crashes or panics, we want to learn why — **only if the user agrees**, and **without ever receiving document content, file names or paths**. Reporting is off by default; after the first crash the app asks once; reports contain a stack (module + offset or symbolicated frames), OS, app version, renderer backend and process kind; they go to an endpoint on the update Worker and are stored in R2 for 30 days (the plan's recommendation), unless this PR's decision picks Sentry. Requirement 9 of the [desktop plan](../../plans/desktop-app.md#functional); D7 is accepted when "a forced crash produces a scrubbed report only after consent (test asserts no paths or content)".

## Context

- [Desktop plan §4 req. 9, §5 crash reports, §8 Q2](../../plans/desktop-app.md#5-design-summary): own Worker + R2 with 30-day retention recommended; alternative self-hosted/SaaS Sentry; install ID **never** linked to crash reports.
- ADR 0011 (web C2): privacy and retention rules; the privacy page.
- ADR 0008 / D2b–D2c: engine process lifecycle; crashes of the sandboxed engine are observed by the supervisor.
- Crates (licence-check each): `crash-handler` + `minidumper` (Embark, MIT/Apache) for out-of-process minidump capture; `minidump`, `minidump-processor`, `minidump-unwind` (rust-minidump, MIT) to **process locally** into a stack trace; `std::panic::set_hook` for Rust panics; `sentry` crate (MIT) only if Sentry is chosen.
- **Privacy constraint**: a raw minidump contains stack and possibly heap memory, i.e. potentially document bytes and paths. Therefore raw minidumps must never be uploaded; only a locally extracted, allow-listed summary is.

## Scope

**In scope**

- Decision (recorded): backend = `apps/updates` Worker endpoint `POST /crash` + R2 `crash-reports/` prefix with a 30-day lifecycle rule, unless the comparison in this PR shows Sentry is clearly better; evaluation criteria: privacy (no raw dumps), cost, operational effort, symbolication.
- **Capture** (`src-tauri/src/crash/`):
  - Main process: `std::panic::set_hook` writing a panic record (message with paths redacted, location file:line from our crates only, backtrace frames as module + offset) to `app_data_dir()/crashes/pending/`; native crash capture via `crash-handler` in-process + `minidumper` server in a small monitor (or the main process monitoring the engine: the engine runs `crash-handler` and the main process acts as the `minidumper` server for it, which fits the sandbox because the dump is written by the unsandboxed side).
  - Engine: panic hook sends a final `Panic { summary }` frame before abort if possible; native crashes captured via the minidumper server in main; supervisor records exit codes/signals.
  - Webview: unhandled JS errors and rejections in the UI → `report_ui_error` (message + stack with URLs reduced to bundle-relative paths).
- **Local processing and scrubbing**: minidumps processed locally with `minidump-processor` into `{ crash_reason, crashing_thread_frames: [{ module, offset, function? }], modules: [{ name, version, debug_id }] }`; the raw dump is deleted after processing. The report is built from an **allow-list** of fields: app version, channel, OS name/version, arch, process kind (main/engine/webview), renderer backend (PDFium version / Canvas path), sandbox level, uptime bucket, stack frames, module list (names only, no paths), a random per-report ID. Every string field passes a scrubber that removes anything path-like (`/…/`, `C:\…`, `~`), `file://` URLs, e-mail addresses and long digit runs; no document metadata, no window titles, no file names.
- **Consent UX**: setting "Send crash reports" (off by default) in the settings window; after the first crash (on next launch or after engine restart), a one-time dialog: "Lamina crashed. Send an anonymous report? [View report] [Don't send] [Send] ☐ Always send" — shown at most once per install unless the user changes the setting; "View report" shows the exact JSON that would be sent.
- **Upload**: `POST https://updates.<domain>/crash` with JSON body (no install ID header — the updater's header is never attached to this client), size cap 64 KiB; offline queue with max 10 pending reports, oldest dropped; reports older than 30 days deleted unsent.
- **Worker** (`apps/updates`): `POST /crash` validates schema and size, rejects bodies containing path-like strings (defence in depth), stores `crash-reports/{yyyy}/{mm}/{dd}/{report_id}.json` in R2, returns 202; rate limiting per IP via the Workers rate-limiting binding or a simple token bucket; no logging of bodies or IPs; R2 lifecycle rule deleting `crash-reports/` after 30 days (owner action or `wrangler r2 bucket lifecycle add`).
- **Symbols**: D6b's workflow archives debug symbols (dSYM, PDB, split DWARF via `objcopy --only-keep-debug`) to a **private** R2 bucket `lamina-symbols` keyed by debug ID; `cargo xtask crash symbolicate <report.json>` fetches symbols and prints a symbolicated stack for maintainers.
- Test-only commands (feature `e2e`): `e2e_crash_main_panic`, `e2e_crash_engine_segv`, `e2e_crash_engine_panic`.

**Out of scope** (and which todo covers it)

- The rest of the settings page (renderer switch): [D7b](D7b-settings-renderer-switch-and-download-page.md).
- Telemetry/usage analytics: none (web plan §8 Q3 governs any future telemetry).
- Crash reporting for the web app: web plan.

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2`, `rust-best-practices`, `workers-best-practices`.
- [ ] Backend decision write-up (ADR 0014 amendment or new short ADR) with the privacy rationale for local processing.
- [ ] Capture: panic hooks (main, engine), crash-handler/minidumper wiring between engine and main, JS error capture.
- [ ] Local processing + allow-list report builder + scrubber (`fn scrub(s: &str) -> Cow<'_, str>`).
- [ ] Consent state machine (`ConsentState { NotAsked, Asked { send: bool }, Always }`) persisted in settings; dialog and settings toggle UI.
- [ ] Upload queue with retry/backoff; the HTTP client is Rust-side (`reqwest` with rustls, or Tauri's http plugin **only** from Rust — no `http:` permission for the webview).
- [ ] Worker endpoint, validation, rate limiting, R2 write, lifecycle rule; tests.
- [ ] D6b workflow addition: symbols to the private bucket; `cargo xtask crash symbolicate`.
- [ ] Privacy page text (crash reports: what, where, retention 30 days, opt-in) — coordinate with the ADR 0011 privacy page owner.

## Acceptance criteria

- [ ] With reporting **off** (default), forcing a crash (main panic, engine segfault, engine panic) produces **no network request** (e2e with a request-recording proxy or a mocked endpoint) and only a local pending record that is deleted after 30 days.
- [ ] After consent, the forced crashes produce reports accepted by the Worker (202) and stored in R2 under `crash-reports/`.
- [ ] A test opens a fixture whose path contains a unique marker directory and file name, and whose content contains a unique marker string, forces each crash type, and asserts that **neither marker nor any path-like string** appears in the uploaded JSON (Rust integration test + e2e).
- [ ] The report never contains the install ID (test asserts no UUID matching the stored install ID; the crash client never sets `Lamina-Install-Id`).
- [ ] Raw minidumps are deleted after local processing (test).
- [ ] Worker tests: schema validation, size cap, path-like rejection, rate limiting, no body/IP logging (console spy), R2 key layout.
- [ ] Engine crashes still trigger the D2b restart flow, unaffected by reporting.

## Tests

- Rust unit tests: scrubber (Unix/Windows paths, UNC, `file://`, home `~`, emails), report builder allow-list (unknown fields impossible by construction), consent state machine.
- Rust integration test: engine child crashes under the minidumper server → processed summary without markers.
- Vitest (Workers pool) for `POST /crash`.
- WebdriverIO: consent dialog flow and "View report".

## Docs to update

- `docs/plans/desktop-app.md` §9 D7: tick "Decide the crash backend…" and "Implement opt-in crash/panic capture with scrubbing; test that no paths or content leak"; answer §8 Q2 (link).
- ADR 0014 amendment or new ADR (+ decisions index); `apps/updates/README.md` (crash endpoint ops); `docs/guidelines/desktop-release.md` (symbols archive, symbolication).

## Risks and open questions

- **Stack frames can still leak data** via function arguments in symbolicated output? No — frames carry addresses only; but panic **messages** can include user data (e.g. `format!` with a path): the scrubber plus a rule "panic messages from lamina crates never include paths or content" (add a clippy/grep check if feasible).
- **minidumper IPC under the sandbox**: the engine's crash-handler must reach the main process's minidumper server; D2c's sandbox may block socket creation — prefer pre-connecting before lock-down; test on each OS.
- **Sentry vs own backend**: Sentry gives symbolication and grouping for free but sends raw minidumps unless configured carefully; our local-processing rule applies either way.
- **Rate limiting/abuse** of a public endpoint: cap sizes, rate limit, validate schema; R2 cost is negligible at expected volume.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
