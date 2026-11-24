# D7c — Ship `desktop v0.1` on the beta channel for Windows and Linux and complete one auto-update cycle

| Field | Value |
| --- | --- |
| ID | `D7c` |
| Status | `ready` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — **M3 Desktop reader beta** (milestone exit for Windows and Linux; macOS follows in [D7g](./D7g-macos-joins-the-desktop-beta.md)) |
| Depends on | [D4c](../done/D4c-native-printing.md), [D4d](../done/D4d-session-restore.md), [D4e](../done/D4e-tab-drag-between-windows.md), [D5c](../done/D5c-desktop-update-e2e.md), [D6c](../done/D6c-macos-signing-and-notarisation.md), [D6d](../done/D6d-windows-signing.md), [D7a](../done/D7a-crash-reporting.md), [D7b](../done/D7b-settings-renderer-switch-and-download-page.md) |
| ADRs | Follows ADR 0014 (channels, rollout, promotion), ADR 0008, ADR 0011 (privacy page), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (SBOM/notices at first public release) |
| Branch | `chore/d7c-desktop-beta` |
| PR title | `chore(desktop): release desktop v0.1.0 beta for Windows and Linux` |
| Size | S–M (version bumps, release notes, checklists; the work is the release itself) |
| PR | — |

## Goal

Put the desktop reader in users' hands on Windows and Linux first: tag and publish `desktop v0.1.0` to the **beta** channel through the D6 pipeline, signed for Windows and Linux with the owner as publisher, linked from the download page, and then prove the update path in the field by shipping `v0.1.1` and watching beta installs auto-update on both. macOS follows in [D7g](./D7g-macos-joins-the-desktop-beta.md), because Apple has not yet enabled notarisation on the owner's team ([D6e](./D6e-macos-signed-release-checks.md)); the beta does not wait for it.

*Proposed 2026-11-23; the owner confirmed it on 2026-11-24.* This changes the plan's accepted-when from all three OSes to Windows and Linux for this todo, and moves the macOS half of milestone M3 ("Signed installers for three OSes, file associations, tabs, auto-update channels", [roadmap](../../plans/roadmap.md)) to D7g. M3 closes when D7g does.

## Context

- [Desktop plan §6 D7 accepted-when](../../plans/desktop-app.md#d7-crash-reporting-and-beta): beta installs and auto-updates for at least one release cycle on Windows and Linux (owner decision 2026-11-24); macOS is [D7g](./D7g-macos-joins-the-desktop-beta.md).
- [D6d](../done/D6d-windows-signing.md) and [D6b](../done/D6b-release-workflow-and-linux-packages.md): the Windows and Linux signing paths this release uses. Real signed Windows release builds succeeded on 2026-11-16.
- [Desktop plan §4](../../plans/desktop-app.md#4-requirements): functional requirements, the release gates the beta must meet (the cold-start gates of ADR 0008 §4, installer < 15 MB; a miss blocks it) and the other non-functional targets it should meet (1 GB first page, idle memory).
- `docs/guidelines/desktop-release.md` (D6a/D6b): "Cutting a release", verification commands, SmartScreen notes, rollout/pause procedures.
- ADR 0014: rollout percentages, kill switch, bridges; default channel decision (plan open question 4).
- [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md): SBOMs and third-party notices required before the first public desktop release.
- Web plan [C7](../../plans/web-app.md#c7-accessibility-offline-and-launch): privacy page and launch conventions to mirror.

## Scope

**In scope**

- **Pre-release checklist** (in the PR, every item ticked with evidence):
  - [ ] All D1–D7b todos are in `done/`.
  - [ ] Release gates met on the reference machines, with numbers recorded: the cold-start gates on Windows and Linux: launch → window built and shown < 300 ms, and launch → first command within 75 ms of the floor build (owner decision 2026-11-11, [ADR 0008 §4](../../decisions/0008-desktop-shell.md#4-tiles-lossless-webp-over-the-lamina-scheme-512-px-single-threaded)), and every installer < 15 MB (owner decision 2026-10-29); a miss blocks the beta.
  - [ ] Pricing status stated: lamina will be priced (owner decision 2026-10-29) and the pricing page (U5a) comes after this beta, so the owner states what the beta is (for example a free preview) and the release notes and download page say lamina will be a paid product.
  - [ ] The release notes and download page say that macOS follows, and that the Mac App Store build ([D8d](../ongoing/D8d-app-store-listing-and-first-review.md)) is a separate install.
  - [ ] Other non-functional targets measured on reference machines and recorded (1 GB first page, idle memory) — misses listed as known issues with follow-up todos.
  - [ ] Supported-OS smoke on clean VMs: Windows 11 (+ Windows 10 22H2 best-effort), Ubuntu 22.04/24.04, Fedora current — install, open via double-click, tabs, print one page, settings, uninstall.
  - [ ] Security review pass: capabilities (no forbidden permissions), CSP, engine sandbox selftest on each OS, no e2e features in release binaries, PDF JavaScript never executed.
  - [ ] Privacy page updated (update endpoint data, install ID and reset, crash reports opt-in and 30-day retention, local recent files).
  - [ ] Third-party notices bundled; SBOMs and attestations generated.
  - [ ] Owner confirms: domain, product name, bundle identifier, default channel for new installs (open question 4), default-handler behaviour (open question 5).
- Version `0.1.0` in `apps/desktop/package.json` (single source; `tauri.conf.json` reads it), changelog `apps/desktop/CHANGELOG.md`, release notes page content at `/desktop/releases/<version>` on the website (`apps/site`; the updater's `notes` URL, [D5a](../done/D5a-update-worker-and-adr-0014.md)) (features, known issues, SmartScreen note if OV, Linux graphics note, how to report bugs, how to verify downloads).
- Release: tag `desktop-v0.1.0` → workflow publishes to `beta` at the rollout chosen per ADR 0014 (for a first beta, 100 % of a small audience is fine — record), downloads JSON updated, download page shows Beta for Windows and Linux.
- Announcement text (README badge/section, project site) — no external posting required by this todo.
- **Update cycle**: after ≥ 3 days, release `0.1.1` (real fixes from beta feedback, or a small change) through the same pipeline with staged rollout 10 % → 50 % → 100 % on `beta`, observing the Worker's 200/204 counts (aggregate only) and crash reports; confirm on Windows and Linux that a `0.1.0` install updated to `0.1.1` without user action beyond the restart prompt (owner/maintainer machines plus at least one external tester on each OS if available).
- Rehearse the kill switch once on `beta` during the 0.1.1 rollout (pause → poll the Worker every 5 s from at least two regions, with and without the install-ID header → every response is 204 within 60 s of the R2 write, expected ≤ 30 s given ADR 0014's single 30 s cache layer → resume), recorded with the measured times.
- Post-release: board updated, plan to-dos ticked, follow-up todos created for issues found (in `ready/` with IDs continuing the D series, e.g. `D8a…`, or under the editor plan if editor-related).

**Out of scope** (and which todo covers it)

- macOS in the beta channel, its clean-Mac checks, and its update cycle: [D7g](./D7g-macos-joins-the-desktop-beta.md), which waits on [D6e](./D6e-macos-signed-release-checks.md).
- Stable channel release (`1.0`/`stable`): a later todo once beta criteria are met (owner decides the bar).
- Store distribution: the Mac App Store has its own track ([D8d](../ongoing/D8d-app-store-listing-and-first-review.md)).
- Editor features: [full-editor plan](../../plans/full-editor.md).

## Tasks

- [ ] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2`.
- [ ] Work through the pre-release checklist; collect evidence links (CI runs, VM notes).
- [ ] Version bump, changelog, release notes.
- [ ] Merge the PR (versions and notes) **before** tagging; tag from `main`; approve the `desktop-release` environment deployment (owner).
- [ ] Verify published artefacts with the runbook commands on each OS; verify the download page.
- [ ] After merge and the `desktop-v0.1.0` tag: run the 0.1.1 update cycle and kill-switch rehearsal (Post-merge checks) and record the results in a follow-up PR that ticks them in the `done/` file.
- [ ] Update plans and board; open follow-up todos.

## Acceptance criteria

- [ ] `desktop v0.1.0` is downloadable from `laminapdf.com/download` (the website) for Windows and Linux; each artefact passes the runbook's signature checks (Authenticode with the owner as publisher, GPG/debsig/rpm, AppImage) and `gh attestation verify`.
- [ ] The 0.1.1 update-cycle and kill-switch rehearsal procedure (below) is written in the runbook and linked from the PR.
- [ ] Crash reporting: zero reports sent without consent (no reports at all is fine); any received reports triaged into follow-up todos.
- [ ] The download page shows macOS as coming soon, or leaves it out until D7g, without a broken link.
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
- `docs/plans/roadmap.md` M3 status: **do not edit here** unless the roadmap owner agrees; note that Windows and Linux are in the beta, and that M3 closes with [D7g](./D7g-macos-joins-the-desktop-beta.md), in the PR for the roadmap owner.
- `README.md` (project): desktop beta mention and download link; `apps/desktop/CHANGELOG.md`.

## Risks and open questions

- **SmartScreen warnings** (OV path) will deter some Windows beta users; the release notes and download page explain it.
- **Linux graphics** issues on NVIDIA/Wayland: the renderer switch and crash-loop guard should cover them; collect reports.
- **Update failures in the field** (permissions, AV interference on Windows, AppImage location not writable): monitor via user reports; the bridge-release mechanism in ADR 0014 handles bad releases.
- **Timing**: the update cycle needs real elapsed time, so it is a Post-merge check recorded in a follow-up PR; the plan's "complete one auto-update cycle" box is ticked in that follow-up, not in the work PR.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
