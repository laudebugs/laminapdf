# D7c — Ship `desktop v0.1` on the beta channel and complete one auto-update cycle

| Field | Value |
| --- | --- |
| ID | `D7c` |
| Status | `ready` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — **M3 Desktop reader beta** (milestone exit) |
| Depends on | [D4c](./D4c-native-printing.md), [D6c](./D6c-macos-signing-and-notarisation.md), [D6d](./D6d-windows-signing.md), [D7a](./D7a-crash-reporting.md), [D7b](./D7b-settings-renderer-switch-and-download-page.md) |
| Blocks | [E1a](./E1a-editing-foundation-feature-plan.md) |
| ADRs | Follows ADR 0014 (channels, rollout, promotion), ADR 0008, ADR 0011 (privacy page), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (SBOM/notices at first public release) |
| Branch | `chore/d7c-desktop-beta` |
| PR title | `chore(desktop): release desktop v0.1.0 beta` |
| Size | S–M (version bumps, release notes, checklists; the work is the release itself) |
| PR | — |

## Goal

Put the desktop reader in users' hands: tag and publish `desktop v0.1.0` to the **beta** channel through the D6 pipeline, signed for all three OSes with the owner as publisher, linked from the download page, and then prove the update path in the field by shipping `v0.1.1` and watching beta installs auto-update on macOS, Windows and Linux. This closes Round D and milestone M3 ("Signed installers for three OSes, file associations, tabs, auto-update channels", [roadmap](../../plans/roadmap.md)).

## Context

- [Desktop plan §6 D7 accepted-when](../../plans/desktop-app.md#d7-crash-reporting-and-beta): "beta installs and auto-updates for at least one release cycle on all three OSes".
- [Desktop plan §4](../../plans/desktop-app.md#4-requirements): functional requirements and non-functional targets the beta should meet (cold start, 1 GB first page, idle memory, installer size).
- `docs/guidelines/desktop-release.md` (D6a/D6b): "Cutting a release", verification commands, SmartScreen notes, rollout/pause procedures.
- ADR 0014: rollout percentages, kill switch, bridges; default channel decision (plan open question 4).
- [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md): SBOMs and third-party notices required before the first public desktop release.
- Web plan [C7](../../plans/web-app.md#c7-accessibility-offline-and-launch): privacy page and launch conventions to mirror.

## Scope

**In scope**

- **Pre-release checklist** (in the PR, every item ticked with evidence):
  - [ ] All D1–D7b todos are in `done/`.
  - [ ] Non-functional targets measured on reference machines and recorded (cold start, 1 GB first page, idle memory, installer sizes) — misses listed as known issues with follow-up todos.
  - [ ] Supported-OS smoke on clean VMs: macOS 13 + latest, Windows 11 (+ Windows 10 22H2 best-effort), Ubuntu 22.04/24.04, Fedora current — install, open via double-click, tabs, print one page, settings, uninstall.
  - [ ] Security review pass: capabilities (no forbidden permissions), CSP, engine sandbox selftest on each OS, no e2e features in release binaries, PDF JavaScript never executed.
  - [ ] Privacy page updated (update endpoint data, install ID and reset, crash reports opt-in and 30-day retention, local recent files).
  - [ ] Third-party notices bundled; SBOMs and attestations generated.
  - [ ] Owner confirms: domain, product name, bundle identifier, default channel for new installs (open question 4), default-handler behaviour (open question 5).
- Version `0.1.0` in `apps/desktop/package.json` (single source; `tauri.conf.json` reads it), changelog `apps/desktop/CHANGELOG.md`, release notes page content (features, known issues, SmartScreen note if OV, Linux graphics note, how to report bugs, how to verify downloads).
- Release: tag `desktop-v0.1.0` → workflow publishes to `beta` at the rollout chosen per ADR 0014 (for a first beta, 100 % of a small audience is fine — record), downloads JSON updated, download page shows Beta.
- Announcement text (README badge/section, project site) — no external posting required by this todo.
- **Update cycle**: after ≥ 3 days, release `0.1.1` (real fixes from beta feedback, or a small change) through the same pipeline with staged rollout 10 % → 50 % → 100 % on `beta`, observing the Worker's 200/204 counts (aggregate only) and crash reports; confirm on each OS that a `0.1.0` install updated to `0.1.1` without user action beyond the restart prompt (owner/maintainer machines plus at least one external tester per OS if available).
- Rehearse the kill switch once on `beta` during the 0.1.1 rollout (pause → verify 204 within 60 s → resume), recorded.
- Post-release: board updated, plan to-dos ticked, follow-up todos created for issues found (in `ready/` with IDs continuing the D series, e.g. `D8a…`, or under the editor plan if editor-related).

**Out of scope** (and which todo covers it)

- Stable channel release (`1.0`/`stable`): a later todo once beta criteria are met (owner decides the bar).
- Store distribution (non-goals).
- Editor features: [full-editor plan](../../plans/full-editor.md).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2`.
- [ ] Work through the pre-release checklist; collect evidence links (CI runs, VM notes).
- [ ] Version bump, changelog, release notes.
- [ ] Merge the PR (versions and notes) **before** tagging; tag from `main`; approve the `release` environment deployment (owner).
- [ ] Verify published artefacts with the runbook commands on each OS; verify the download page.
- [ ] Run the 0.1.1 update cycle and kill-switch rehearsal; record results in a follow-up commit to this todo (the PR may stay open until the cycle completes, or a second small PR records it — keep this file in `ongoing/` until done).
- [ ] Update plans and board; open follow-up todos.

## Acceptance criteria

- [ ] `desktop v0.1.0` is downloadable from `<domain>/download` for macOS, Windows and Linux; each artefact passes the runbook's signature checks (Gatekeeper, Authenticode with the owner as publisher, GPG/debsig/rpm, AppImage) and `gh attestation verify`.
- [ ] At least one install per OS on the beta channel auto-updated from `0.1.0` to `0.1.1` (recorded: OS version, install type, time to update).
- [ ] Kill switch rehearsal: `paused` on `beta` → Worker returns 204 within 60 s → resumed (recorded).
- [ ] Crash reporting: zero reports sent without consent (no reports at all is fine); any received reports triaged into follow-up todos.
- [ ] Known issues and target misses are listed in the release notes and have follow-up todos.

## Tests

No new automated tests required; the release relies on the CI suites from D2e–D7b and the clean-VM checklist. Any bug fixed for 0.1.1 comes with its regression test in that fix's PR.

## Docs to update

- `docs/plans/desktop-app.md`: tick "Ship `desktop v0.1` on the beta channel and complete one auto-update cycle"; set plan status (e.g. "Beta shipped") — and answer open questions 4 and 5 with links.
- `docs/plans/roadmap.md` M3 status: **do not edit here** unless the roadmap owner agrees; note M3 completion in the PR for the roadmap owner.
- `README.md` (project): desktop beta mention and download link; `apps/desktop/CHANGELOG.md`.

## Risks and open questions

- **SmartScreen warnings** (OV path) will deter some Windows beta users; the release notes and download page explain it.
- **Linux graphics** issues on NVIDIA/Wayland: the renderer switch and crash-loop guard should cover them; collect reports.
- **Update failures in the field** (permissions, AV interference on Windows, AppImage location not writable): monitor via user reports; the bridge-release mechanism in ADR 0014 handles bad releases.
- **Timing**: the update cycle needs real elapsed time; keep this todo `ongoing/` until the cycle is recorded.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
