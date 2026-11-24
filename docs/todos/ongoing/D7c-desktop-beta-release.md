# D7c — Ship `desktop v0.1` on the beta channel for Windows and Linux and complete one auto-update cycle

| Field | Value |
| --- | --- |
| ID | `D7c` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — **M3 Desktop reader beta** (milestone exit for Windows and Linux; macOS follows in [D7g](../ready/D7g-macos-joins-the-desktop-beta.md)) |
| Depends on | [D4c](../done/D4c-native-printing.md), [D4d](../done/D4d-session-restore.md), [D4e](../done/D4e-tab-drag-between-windows.md), [D5c](../done/D5c-desktop-update-e2e.md), [D6d](../done/D6d-windows-signing.md), [D7a](../done/D7a-crash-reporting.md), [D7b](../done/D7b-settings-renderer-switch-and-download-page.md) |
| ADRs | Follows ADR 0014 (channels, rollout, promotion), ADR 0008, ADR 0011 (privacy page), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (SBOM/notices at first public release) |
| Branch | `chore/d7c-desktop-beta` |
| PR title | `chore(desktop): release desktop v0.1.0 beta for Windows and Linux` |
| Size | S–M (version bumps, release notes, checklists; the work is the release itself) |
| PR | [#791](https://github.com/laudebugs/laminapdf/pull/791) |

## Goal

Put the desktop reader in users' hands on Windows and Linux first: tag and publish `desktop v0.1.0` to the **beta** channel through the D6 pipeline, signed for Windows and Linux with the owner as publisher, linked from the download page, and then prove the update path in the field by shipping `v0.1.1` and watching beta installs auto-update on both. macOS follows in [D7g](../ready/D7g-macos-joins-the-desktop-beta.md), because Apple has not yet enabled notarisation on the owner's team ([D6e](../ready/D6e-macos-signed-release-checks.md)); the beta does not wait for it.

Owner decision 2026-11-24: this changes the plan's accepted-when from all three OSes to Windows and Linux for this todo, and moves the macOS half of milestone M3 ("Signed installers for three OSes, file associations, tabs, auto-update channels", [roadmap](../../plans/roadmap.md)) to D7g. M3 closes when D7g does.

## Context

- [Desktop plan §6 D7 accepted-when](../../plans/desktop-app.md#d7-crash-reporting-and-beta): beta installs and auto-updates for at least one release cycle on Windows and Linux (owner decision 2026-11-24); macOS is [D7g](../ready/D7g-macos-joins-the-desktop-beta.md).
- [D6d](../done/D6d-windows-signing.md) and [D6b](../done/D6b-release-workflow-and-linux-packages.md): the Windows and Linux signing paths this release uses. Real signed Windows release builds succeeded on 2026-11-16.
- [Desktop plan §4](../../plans/desktop-app.md#4-requirements): functional requirements, the release gates the beta must meet (the cold-start gates of ADR 0008 §4, installer < 16 MB; a miss blocks it) and the other non-functional targets it should meet (1 GB first page, idle memory).
- `docs/guidelines/desktop-release.md` (D6a/D6b): "Cutting a release", verification commands, SmartScreen notes, rollout/pause procedures.
- ADR 0014: rollout percentages, kill switch, bridges; default channel decision (plan open question 4).
- [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md): SBOMs and third-party notices required before the first public desktop release.
- Web plan [C7](../../plans/web-app.md#c7-accessibility-offline-and-launch): privacy page and launch conventions to mirror.

## Scope

**In scope**

- **Pre-release checklist** (in the PR, every item ticked with evidence):
  - [x] All D1–D7b todos are in `done/`, except D6e (macOS, see D7g).
    - *As built / evidence (2026-11-24):* every D1–D7b todo is in [`done/`](../done/) (D1a–D2g, D3a–c, D4a–f, D5a–c, D6a–d, D7a, D7b; there is no D2f), and so are D7d, D7e, D7f and D8a–c. `cargo xtask todos` lists D6e and D7g in `ready/` and D7c and D8d in `ongoing/`.
  - [ ] Release gates met on the reference machines, with numbers recorded: the cold-start gates on Windows and Linux: launch → window built and shown < 300 ms, and launch → first command within 75 ms of the floor build (owner decision 2026-11-11, [ADR 0008 §4](../../decisions/0008-desktop-shell.md#4-tiles-lossless-webp-over-the-lamina-scheme-512-px-single-threaded)), and every installer < 16 MB (owner decision 2026-11-15, [plan §4](../../plans/desktop-app.md#4-requirements)); a miss blocks the beta.
    - *As built / evidence, installer sizes (gate: each under 16,000,000 bytes, checked by `cargo xtask desktop-release sizes`):* the signed staging run [43854672452](https://github.com/laudebugs/laminapdf/actions/runs/43854672452) (2026-11-16, tag-mode Windows builds, version 0.0.178) made a setup `.exe` of 10,087,520 bytes and an MSI of 14,008,320 on x64, and 8,873,328 and 13,049,856 on Arm64, read from the run's artifacts. All four are under the gate. The Linux packages come from the scheduled run [45199278237](https://github.com/laudebugs/laminapdf/actions/runs/45199278237) (2026-11-23, nightly mode, so no `.rpm`): the `.deb` is 15,484,730 bytes on x64 and 14,873,424 on Arm64; the AppImages (92,629,496 and 89,999,880 bytes) are reported, not gated. The first tagged Linux build, with its `.rpm`, is the first measurement of that format. Owner/hardware: none for sizes; the tag run's `sizes` step fails the build above the gate.
    - *As built / evidence, cold start:* CI measures something else than the gates. The full lane's `Measure the cold start` step times five launches of the **debug e2e build** on a shared hosted runner, from the start of `run()` to the empty state on screen, and records them without asserting ([`cold-start.ts`](../../../apps/desktop/e2e/cold-start.ts)). The last green run on `main`, [45318193702](https://github.com/laudebugs/laminapdf/actions/runs/45318193702) (2026-11-24), recorded a median of 685 ms on Linux and 994 ms on Windows. Those figures don't bear on the gates: the gates (window built and shown under 300 ms, first command within 75 ms of the floor build) need a release build, a reference machine and the floor build run in turn, which [`tools/desktop/cold_start.py`](../../../tools/desktop/README.md) does. Owner/hardware: run it on the Windows and Linux reference machines and record the numbers here; a miss blocks the beta.
  - [ ] Pricing status stated: lamina will be priced (owner decision 2026-10-29) and the pricing page (U5a) comes after this beta, so the owner states what the beta is (for example a free preview) and the release notes and download page say lamina will be a paid product.
    - *As built / evidence:* the release notes page and the download page each carry the sentence "Lamina PDF will be a paid product, and its pricing page comes after this beta." followed by the marked placeholder "[OWNER TO CONFIRM: this beta is a free preview.]" (`WORDS.pricing` in [`downloads.ts`](../../../apps/site/src/data/downloads.ts), which [`desktop-releases.ts`](../../../apps/site/src/data/desktop-releases.ts) reuses). Owner/hardware: state what the beta is and replace the placeholder; the new website words need your approval.
  - [x] The release notes and download page say that macOS follows, and that the Mac App Store build ([D8d](../ongoing/D8d-app-store-listing-and-first-review.md)) is a separate install.
    - *As built / evidence:* the beta pane of `/download/` says "macOS is coming soon … The Mac App Store build is a separate install and doesn't come from this page", and holds macOS out of the beta and stable lists (`COMING_SOON`, `offered`), so no `.dmg` link or macOS checksum link can appear whatever the release workflow lists. The 0.1.0 notes page says the same in "Where it runs". Unit tests ([`downloads.test.ts`](../../../apps/site/test/downloads.test.ts), [`desktop-releases.test.ts`](../../../apps/site/test/desktop-releases.test.ts)) and the browser suite ([`download.e2e.test.ts`](../../../apps/site/e2e/download.e2e.test.ts), 14 tests in Chromium over its fixture) check it. Both pages' words wait for the owner's approval.
  - [ ] Other non-functional targets measured on reference machines and recorded (1 GB first page, idle memory) — misses listed as known issues with follow-up todos.
    - *As built / evidence:* the same CI step opens the generated 1 GB fixture and reports the time to its first page against the 1,500 ms target, never asserting it ([`large.e2e.ts`](../../../apps/desktop/e2e/specs/large.e2e.ts)). Run [45318193702](https://github.com/laudebugs/laminapdf/actions/runs/45318193702) reported 2,278 ms on Linux and 2,525 ms on Windows, on shared runners with the debug e2e build, so over the target but not a measurement of the release build. CI records no idle memory. Owner/hardware: measure the first page of a 1 GB PDF and the idle memory with one 10-page document (target 150 MB or less) on the reference machines, with the release build. List each miss under the release notes' known issues with a follow-up todo.
  - [ ] Supported-OS smoke on clean VMs: Windows 11 (+ Windows 10 22H2 best-effort), Ubuntu 22.04/24.04, Fedora current — install, open via double-click, tabs, print one page, settings, uninstall.
    - *As built / evidence:* the e2e scenarios run on a Windows and an Ubuntu runner on every desktop change ([`full.yml`](../../../.github/workflows/full.yml) `desktop`), and the release build installs and removes both Windows installers on a runner and checks "Open with" and the MSI's flavour ([`check-windows-installers.ps1`](../../../apps/desktop/scripts/check-windows-installers.ps1), runbook §3). No CI job installs the Linux packages on a clean Ubuntu or Fedora machine, or drives the signed installers as a person. Owner/hardware: run the checklist on clean Windows 11, Ubuntu 22.04 and 24.04, and Fedora virtual machines (and Windows 10 22H2 if wanted), with the SmartScreen prompt, the MSI's publisher in UAC and "Open with" ([runbook §6](../../guidelines/desktop-release.md#a-tagged-release)), and record each result here.
  - [ ] Security review pass: capabilities (no forbidden permissions), CSP, engine sandbox selftest on Windows and Linux, no e2e features in release binaries, PDF JavaScript never executed.
    - *Capabilities (verified):* [`capabilities/main.json`](../../../apps/desktop/src-tauri/capabilities/main.json) grants only the app's own `allow-*` commands and `core:event:allow-listen` and `allow-unlisten`; it grants no `core:default` and nothing from `shell:`, `fs:`, `http:`, `dialog:`, `process:`, `opener:`, `log:` or `updater:`. [`capabilities.rs`](../../../apps/desktop/src-tauri/tests/capabilities.rs) fails a PR that adds one, lists a file nobody enabled, or puts `e2e` in the base list; it runs in every native Rust leg of the fast lane.
    - *CSP (verified):* [`tauri.conf.json`](../../../apps/desktop/src-tauri/tauri.conf.json) sets `default-src 'self'; script-src 'self'` (no inline script, no `unsafe-eval`), `object-src 'none'`, `frame-src 'none'`, `base-uri 'none'`, `form-action 'none'` and `freezePrototype`; `style-src` allows inline styles only. `connect-src` names `ipc:` and the app's own `lamina:` scheme, with no network host. The guard above holds the e2e build's CSP to the base one plus `unsafe-eval`.
    - *Engine sandbox selftest (verified in CI, not in the e2e jobs):* [`sandbox_selftest.rs`](../../../crates/laminapdf-desktop-engine/tests/sandbox_selftest.rs) is a Rust integration test that locks the engine down as it does to serve, then tries to read, write, connect, spawn, fork and exceed its memory limit. It runs in the fast lane's native legs on Linux and Windows (x64 and Arm64), and in `check-windows` on the full lane.
    - *No e2e feature in a release binary (partly verified):* the release build ([`desktop-build`](../../../.github/actions/desktop-build/action.yml)) calls `tauri build` with `tauri.bundle.conf.json`, `tauri.release.conf.json` and the run's config, and never `--features e2e`; the `e2e` capability exists only in the e2e config. CI's string check for the e2e commands, the WebdriverIO plugin and the probe ([`full.yml`](../../../.github/workflows/full.yml), "Check the build has no end-to-end command, plugin or probe") reads the plain debug build, not the release binary. That is the same feature set, but a check on the release binary itself doesn't exist.
    - *PDF JavaScript (by design, no test):* the pinned PDFium is `chromium/7881` from `pdfium-binaries`, the build without V8 or XFA ([ADR 0008](../../decisions/0008-desktop-shell.md), [`pdfium.rs`](../../../tools/xtask/src/pdfium.rs)), and the webview shows rasters, so a document's script has no engine to run on. No test opens a PDF with a script and shows it didn't run. Worth a follow-up todo.
    - Owner/hardware: decide whether the two gaps (a check on the release binary, a JavaScript test) block the beta, and sign off the review.
  - [ ] Privacy page updated (update endpoint data, install ID and reset, crash reports opt-in and 30-day retention, local recent files).
    - *As built / evidence:* before this todo the page covered the web reader, plus one paragraph on the desktop's local recent files and Clear list; update checks, the install ID, its reset and crash reports were missing, though the app's Settings link to the page for them. This branch adds a "The desktop app" section ([`privacy.ts`](../../../apps/site/src/data/privacy.ts)): update checks (what a check sends, the anonymous ID, Reset anonymous ID, nothing kept) and crash reports (consent, what a report holds, 30 days), the second being the text issue [#640](https://github.com/laudebugs/laminapdf/issues/640) proposed. Each names the tests that prove it, and [`privacy.test.ts`](../../../apps/site/test/privacy.test.ts) holds them and the sentences that repeat the app's and the update service's own words to their sources. Owner/hardware: approve the words (issue #640 asks, and also asks whether they belong on `/privacy/`). The 30-day deletion rests on the crash bucket's lifecycle rule, which the owner set up by hand ([update service README, owner setup step 5](../../../apps/updates/README.md#owner-setup)); check it with `wrangler r2 bucket lifecycle list lamina-pdf-crash-reports`.
  - [x] Third-party notices bundled; SBOMs and attestations generated.
    - *As built / evidence:* every build writes the notices (`cargo xtask desktop-notices --bundle`) and fails when they are missing from the bundle (the `.deb`'s file list, the Windows build's copy next to the executable, the macOS app's resources), writes the CycloneDX SBOMs (`cargo xtask desktop-release sbom`), and the `attest` job attests every file and SBOM ([`desktop-release.yml`](../../../.github/workflows/desktop-release.yml)). All `Attest` jobs passed on the signed staging run [43854672452](https://github.com/laudebugs/laminapdf/actions/runs/43854672452) (Windows, tag mode) and on the scheduled run [45199278237](https://github.com/laudebugs/laminapdf/actions/runs/45199278237) (all six targets). `gh attestation verify` on the tagged files is an acceptance criterion, checked after the tag.
  - [ ] Owner confirms: domain, product name, bundle identifier, default channel for new installs (open question 4), default-handler behaviour (open question 5).
    - *As built / evidence:* the runbook records the product name `Lamina PDF` and the identifier `com.laminapdf.desktop` ([`tauri.conf.json`](../../../apps/desktop/src-tauri/tauri.conf.json), [runbook](../../guidelines/desktop-release.md#what-was-decided-on-2026-11-12)). [Plan §8](../../plans/desktop-app.md#8-open-questions) answers question 4 (an install follows its build's channel: beta for this release; stable once it exists) and question 5 (the installers register Lamina PDF as able to open PDFs and leave the default to you; the release build checks that it is never the default). Owner/hardware: confirm each, or change it before the tag.
- Version `0.1.0` in `apps/desktop/package.json` (single source; `tauri.conf.json` reads it), changelog `apps/desktop/CHANGELOG.md`, release notes page content at `/desktop/releases/<version>` on the website (`apps/site`; the updater's `notes` URL, [D5a](../done/D5a-update-worker-and-adr-0014.md)) (features, known issues, SmartScreen note if OV, Linux graphics note, how to report bugs, how to verify downloads).
- Release: tag `desktop-v0.1.0` → workflow publishes to `beta` at the rollout chosen per ADR 0014 (for a first beta, 100 % of a small audience is fine — record), downloads JSON updated, download page shows Beta for Windows and Linux.
- Announcement text (README badge/section, project site) — no external posting required by this todo.
- **Update cycle**: after ≥ 3 days, release `0.1.1` (real fixes from beta feedback, or a small change) through the same pipeline with staged rollout 10 % → 50 % → 100 % on `beta`, observing the Worker's 200/204 counts (aggregate only) and crash reports; confirm on Windows and Linux that a `0.1.0` install updated to `0.1.1` without user action beyond the restart prompt (owner/maintainer machines plus at least one external tester on each OS if available).
- Rehearse the kill switch once on `beta` during the 0.1.1 rollout (pause → poll the Worker every 5 s from at least two regions, with and without the install-ID header → every response is 204 within 60 s of the R2 write, expected ≤ 30 s given ADR 0014's single 30 s cache layer → resume), recorded with the measured times.
- Post-release: board updated, plan to-dos ticked, follow-up todos created for issues found (in `ready/` with IDs continuing the D series, e.g. `D8a…`, or under the editor plan if editor-related).

**Out of scope** (and which todo covers it)

- macOS in the beta channel, its clean-Mac checks, and its update cycle: [D7g](../ready/D7g-macos-joins-the-desktop-beta.md), which waits on [D6e](../ready/D6e-macos-signed-release-checks.md).
- Stable channel release (`1.0`/`stable`): a later todo once beta criteria are met (owner decides the bar).
- Store distribution: the Mac App Store has its own track ([D8d](../ongoing/D8d-app-store-listing-and-first-review.md)).
- Editor features: [full-editor plan](../../plans/full-editor.md).

## Tasks

- [x] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2`.
- [ ] Work through the pre-release checklist; collect evidence links (CI runs, VM notes).
- [x] Version bump, changelog, release notes. *Done on this branch; the bump is informational: a release build takes its version from the tag ([runbook §6](../../guidelines/desktop-release.md#6-cutting-a-release)). The notes and the new website words wait for the owner's approval.*
- [ ] Merge the PR (versions and notes) **before** tagging; tag from `main`; approve the `desktop-release` environment deployment (owner).
- [ ] Verify published artefacts with the runbook commands on each OS; verify the download page.
- [ ] After merge and the `desktop-v0.1.0` tag: run the 0.1.1 update cycle and kill-switch rehearsal (Post-merge checks) and record the results in a follow-up PR that ticks them in the `done/` file.
- [ ] Update plans and board; open follow-up todos.

## Acceptance criteria

- [ ] `desktop v0.1.0` is downloadable from `laminapdf.com/download` (the website) for Windows and Linux; each artefact passes the runbook's signature checks (Authenticode with the owner as publisher, GPG/debsig/rpm, AppImage) and `gh attestation verify`.
- [x] The 0.1.1 update-cycle and kill-switch rehearsal procedure (below) is written in the runbook and linked from the PR. *Written in [the runbook](../../guidelines/desktop-release.md#the-first-betas-update-cycle-and-kill-switch-rehearsal); link it from the PR.*
- [ ] Crash reporting: zero reports sent without consent (no reports at all is fine); any received reports triaged into follow-up todos.
- [x] The download page shows macOS as coming soon, or leaves it out until D7g, without a broken link. *Coming soon, with no macOS file or link on the beta list.*
- [ ] Known issues and target misses are listed in the release notes and have follow-up todos.

## Post-merge checks

- [ ] Tag `desktop-v0.1.0` from `main` after this PR merges and approve the `desktop-release` environment; the publish job and post-publish Worker check are green.
- [ ] At least one Windows install and one Linux install on the beta channel auto-updated from `0.1.0` to `0.1.1` (recorded: OS version, install type, time to update).
- [ ] Kill switch rehearsal: `paused` on `beta` → every poll returns 204 within 60 s of the manifest write (measured worst case recorded; anything over 30 s is investigated as a sign of a second cache layer) → resumed (recorded).
- [ ] Both are recorded in a follow-up PR that ticks these boxes in the `done/` file (this todo moves to `done/` with its work PR, per the DoD).

## Tests

No new automated tests required; the release relies on the CI suites from D2e–D7b and the clean-VM checklist. Any bug fixed for 0.1.1 comes with its regression test in that fix's PR.

## Docs to update

- `docs/plans/desktop-app.md`: tick "Ship `desktop v0.1` on the beta channel for Windows and Linux and complete one auto-update cycle" in the follow-up PR that records the cycle (Post-merge checks); in the work PR set plan status (e.g. "Beta shipped") — and answer open questions 4 and 5 with links.
- `docs/plans/roadmap.md` M3 status: **do not edit here** unless the roadmap owner agrees; note that Windows and Linux are in the beta, and that M3 closes with [D7g](../ready/D7g-macos-joins-the-desktop-beta.md), in the PR for the roadmap owner.
- `README.md` (project): desktop beta mention and download link; `apps/desktop/CHANGELOG.md`.

## Risks and open questions

- **SmartScreen warnings** (OV path) will deter some Windows beta users; the release notes and download page explain it.
- **Linux graphics** issues on NVIDIA/Wayland: the renderer switch and crash-loop guard should cover them; collect reports.
- **Update failures in the field** (permissions, AV interference on Windows, AppImage location not writable): monitor via user reports; the bridge-release mechanism in ADR 0014 handles bad releases.
- **A `desktop-v*` tag builds macOS too, which blocks the Windows and Linux beta** (found 2026-11-24 while preparing the PR). `plan` gives a tag every one of the six targets ([`plan.rs`](../../../tools/xtask/src/desktop_release/plan.rs), `every_target`), `SIGNED_PLATFORMS` names macOS, and the publish job needs every build and attestation. Apple's notarisation answers status 7000 for the owner's team, so the macOS legs fail and nothing publishes. `only_os` exists only on a staging dispatch. Before the tag the workflow needs a way to build and publish Windows and Linux alone (for example, a tag-mode `only_os` that leaves macOS out of the matrix until [D7g](../ready/D7g-macos-joins-the-desktop-beta.md)), and the manifest then lists no macOS platform, so the Worker offers macOS installs nothing. The download page already holds macOS back from the beta list, so it needs no change for that. This is a workflow change and a macOS-path change, so this PR doesn't make it; it needs the owner's go-ahead.
- **Timing**: the update cycle needs real elapsed time, so it is a Post-merge check recorded in a follow-up PR; the plan's "complete one auto-update cycle" box is ticked in that follow-up, not in the work PR.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
