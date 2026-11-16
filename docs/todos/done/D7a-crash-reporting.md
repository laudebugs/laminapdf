# D7a — Opt-in crash and panic reporting with scrubbing

| Field | Value |
| --- | --- |
| ID | `D7a` |
| Status | `done` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D5a](./D5a-update-worker-and-adr-0014.md), [D5b](./D5b-updater-in-app.md) |
| Dependency notes | D6b's part, the symbol archive, moved to [D7d](../done/D7d-native-crash-minidumps-and-symbols.md), which depends on D6b instead. |
| ADRs | Records the crash-backend decision (plan open question 2) as an amendment to ADR 0014 or a short new ADR (next free number). Follows ADR 0011 (privacy/retention, web C2), ADR 0008 (engine process) |
| Branch | `feat/d7a-crash-reporting` |
| PR title | `feat(desktop): add opt-in, scrubbed crash and panic reporting` |
| Size | L (700–1,300 lines across app, engine and Worker) |
| PR | [#639](https://github.com/laudebugs/laminapdf/pull/639) |

## Goal

When lamina or its engine crashes or panics, we want to learn why — **only if the user agrees**, and **without ever receiving document content, file names or paths**. Reporting is off by default; after the first crash the app asks once; reports contain a stack (module + offset or symbolicated frames), OS, app version, renderer backend and process kind; they go to an endpoint on the update Worker and are stored in R2 for 30 days (the plan's recommendation), unless this PR's decision picks Sentry. Requirement 9 of the [desktop plan](../../plans/desktop-app.md#functional); D7 is accepted when "a forced crash produces a scrubbed report only after consent (test asserts no paths or content)".

## Context

- **Design (Paper):** [S07 Settings window](../../design/paper/desktop-shell.md#s07-settings-window), [S08 Crash recovered](../../design/paper/desktop-shell.md#s08-crash-recovered). Build to match these screens; record any deviation in the area document ([Paper design plan §5](../../plans/paper-design.md#5-how-the-designs-are-used)).
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
  - Webview: unhandled JS errors and rejections in the UI → `report_ui_error` (message + stack). The stack is parsed **in Rust** into structured frames `{ file, line, column, function? }`; `file` is kept only if, after stripping the app origin (`tauri://localhost/`, `http://tauri.localhost/`), it matches the bundle asset pattern `^assets/[A-Za-z0-9._-]+\.(js|mjs)$` (Vite's hashed output) — anything else becomes `"<external>"`. Frames are never sent as one free-text string.
- **Local processing and scrubbing**: minidumps processed locally with `minidump-processor` into `{ crash_reason, crashing_thread_frames: [{ module, offset, function? }], modules: [{ name, version, debug_id }] }`; the raw dump is deleted after processing. The report is built from an **allow-list** of fields: app version, channel, OS name/version, arch, process kind (main/engine/webview), renderer backend (PDFium version / Canvas path), sandbox level, uptime bucket, stack frames, module list (names only, no paths), a random per-report ID. Every **free-text** string field (crash reason, panic or JS message, function names) passes a scrubber that removes anything path-like (`/…/`, `C:\…`, `~`), `file://` URLs, e-mail addresses and long digit runs; no document metadata, no window titles, no file names. The structured `file` field of webview frames is the one exception: it is not scrubbed but must match the asset pattern above (otherwise `"<external>"`), so `assets/index-3f2a.js` survives and a real path cannot.
- **Consent UX**: setting "Send crash reports" (off by default) in the settings window; after the first crash (on next launch or after engine restart), a one-time dialog: "Lamina crashed. Send an anonymous report? [View report] [Don't send] [Send] ☐ Always send" — shown at most once per install unless the user changes the setting; "View report" shows the exact JSON that would be sent.
- **Upload**: `POST https://updates.laminapdf.com/crash` with JSON body (no install ID header — the updater's header is never attached to this client), size cap 64 KiB; offline queue with max 10 pending reports, oldest dropped; reports older than 30 days deleted unsent.
- **Worker** (`apps/updates`): `POST /crash` validates schema and size, rejects bodies whose free-text fields contain path-like strings and whose webview frame `file` values don't match the asset pattern or `"<external>"` (defence in depth; same pattern shared as a constant with the client), stores `crash-reports/{yyyy}/{mm}/{dd}/{report_id}.json` in R2, returns 202; rate limiting per IP via the Workers rate-limiting binding or a simple token bucket; no logging of bodies or IPs; R2 lifecycle rule deleting `crash-reports/` after 30 days (owner action or `wrangler r2 bucket lifecycle add`).
- **Symbols**: D6b's workflow archives debug symbols (dSYM, PDB, split DWARF via `objcopy --only-keep-debug`) to a **private** R2 bucket `lamina-pdf-symbols` keyed by debug ID; `cargo xtask crash symbolicate <report.json>` fetches symbols and prints a symbolicated stack for maintainers.
- Test-only commands (feature `e2e`): `e2e_crash_main_panic`, `e2e_crash_engine_segv`, `e2e_crash_engine_panic`.

**Out of scope** (and which todo covers it)

- The rest of the settings page (renderer switch): [D7b](../done/D7b-settings-renderer-switch-and-download-page.md).
- Usage analytics: none here; optional opt-in usage counts for both shells come with [U5c](../ready/U5c-opt-in-usage-telemetry.md), which reuses this todo's consent settings (owner decision 2026-10-29).
- Crash reporting for the web app: web plan.

## Tasks

- [x] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2`, `rust-best-practices`, `workers-best-practices`.
- [x] Backend decision write-up (ADR 0014 amendment or new short ADR) with the privacy rationale for local processing. *ADR 0014 §10.*
- [x] Capture: panic hooks (main, engine), ~~crash-handler/minidumper wiring between engine and main~~ (moved to [D7d](../done/D7d-native-crash-minidumps-and-symbols.md)), JS error capture.
- [x] ~~Local processing~~ (minidumps: D7d) + allow-list report builder + scrubber (`fn scrub(s: &str) -> Cow<'_, str>`).
- [x] Consent state machine (`ConsentState { NotAsked, Asked { send: bool }, Always }`) persisted in settings; dialog and settings toggle UI.
- [x] Upload queue with retry/backoff; the HTTP client is Rust-side (`reqwest` with rustls, or Tauri's http plugin **only** from Rust — no `http:` permission for the webview).
- [x] Worker endpoint, validation, rate limiting, R2 write, lifecycle rule; tests. *The lifecycle rule is the owner's step in apps/updates/README.md.*
- [ ] ~~D6b workflow addition: symbols to the private bucket; `cargo xtask crash symbolicate`.~~ Moved to [D7d](../done/D7d-native-crash-minidumps-and-symbols.md), which needs D6b's workflow on `main`.
- [x] Privacy page text (crash reports: what, where, retention 30 days, opt-in) — coordinate with the ADR 0011 privacy page owner. *The page's promises change only with the owner's sign-off: the text is in #640 for the owner.*

## Acceptance criteria

- [x] With reporting **off** (default), forcing a crash (main panic, engine segfault, engine panic) produces **no network request** (e2e with a request-recording proxy or a mocked endpoint) and only a local pending record that is deleted after 30 days. *The engine segfault in the e2e run; the main panic in `a_main_process_panic_should_leave_a_held_report_until_asked`.*
- [x] After consent, the forced crashes produce reports accepted by the Worker (202) and stored in R2 under `crash-reports/`. *The e2e run against a stand-in answering 202; the Worker's R2 layout in its workerd test.*
- [x] A test opens a fixture whose path contains a unique marker directory and file name, and whose content contains a unique marker string, forces each crash type, and asserts that **neither marker nor any path-like string** appears in the uploaded JSON (Rust integration test + e2e).
- [x] The report never contains the install ID (test asserts no UUID matching the stored install ID; the crash client never sets `Lamina-Install-Id`). *The e2e run checks the header; the Worker refuses a report carrying it.*
- [ ] ~~Raw minidumps are deleted after local processing (test).~~ Moved to [D7d](../done/D7d-native-crash-minidumps-and-symbols.md).
- [x] Worker tests: schema validation, size cap, path-like rejection in free-text fields, a webview report with `assets/…js` frames accepted, rate limiting, no body/IP logging (console spy), R2 key layout.
- [x] Engine crashes still trigger the D2b restart flow, unaffected by reporting.

## Post-merge checks

None.

## Tests

- Rust unit tests: scrubber (Unix/Windows paths, UNC, `file://`, home `~`, emails); JS stack parsing (a WebKit and a Chromium-format stack with `tauri://localhost/assets/index-abc.js:1:234` keep `assets/index-abc.js`; a `file:///Users/x/…` frame becomes `<external>`); report builder allow-list (unknown fields impossible by construction), consent state machine.
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

## As built

These differ from the text above:

- **Split.** Minidumps of native crashes, their local processing, the deletion test and the symbol archive went to [D7d](../done/D7d-native-crash-minidumps-and-symbols.md): they need D6b's release workflow, and capture across the sandbox needs testing on each OS. S08's tab choice went to [D7e](../ready/D7e-crash-recovered-tab-choice.md), with the e2e run pair that forces a main-process panic and relaunches. A main-process panic is tested here in Rust (`a_main_process_panic_should_leave_a_held_report_until_asked`).
- **Engine panics abort.** The engine's panic hook sends its summary, then aborts, so the supervisor sees a crash and restarts the engine. Before, a panic in the worker thread left an engine that beat but never answered.
- **Webview messages.** A page error keeps only its type, from a fixed list, and its frames: a script's message can quote document text, and the marker test showed that no scrubber can tell such text from code.
- **Renderer.** The report names the renderer (`pdfium`) without its version: each app version ships one pinned PDFium, so `app_version` says which.
- **Frames.** A panic's frames are function names with the offset within each function. A Rust function is linked into the panicking binary, which names its module; any other's module is `<unknown>`. A debug build names functions from its debug info, without their paths, so there every module is `<unknown>`. The engine sends no frames: naming them reads its own binary from disk, which its sandbox forbids (on Linux, seccomp ends the process for it). Its reports hold the panic's message and place; D7d's minidumps give native stacks.
- **Queues.** `held/` and `outbox/` each keep ten reports. Moving a report from one to the other keeps its age.
- **Consent off.** The e2e run checks only an engine segmentation fault with sending off. Every kind of crash goes through `Shelf::for_consent`, which alone decides whether a report waits or goes, and a Rust test checks it for each consent state.
- **Privacy page.** Its promises change only with the owner's sign-off, so the proposed text is in [#640](https://github.com/laudebugs/laminapdf/issues/640) rather than in this PR.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
