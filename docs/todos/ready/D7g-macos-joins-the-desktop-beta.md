# D7g — macOS joins the desktop beta

| Field | Value |
| --- | --- |
| ID | `D7g` |
| Status | `ready` |
| Plan | [Desktop app §6 D7](../../plans/desktop-app.md#d7-crash-reporting-and-beta) |
| Round / milestone | Round D (desktop) — **M3 Desktop reader beta** (milestone exit for macOS) |
| Depends on | [D6e](./D6e-macos-signed-release-checks.md), [D7c](../ongoing/D7c-desktop-beta-release.md) |
| ADRs | Follows ADR 0014 (channels, rollout, promotion; [§11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build) for the store build) and [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `chore/d7g-macos-beta` |
| PR title | `chore(desktop): add macOS to the desktop beta` |
| Size | S (release notes, download page entry and checklists; the work is the release itself) |
| PR | — |

## Goal

Add the signed and notarised macOS build to the **beta** channel once Apple enables notarisation on the owner's team, and prove on macOS the update cycle that [D7c](../ongoing/D7c-desktop-beta-release.md) proved on Windows and Linux. This closes milestone M3 for all three OSes.

Owner decision 2026-11-24: the beta ships on Windows and Linux first. Apple's notary service rejects every submission (status 7000, "Team is not yet configured for notarization") until Apple Developer Support enables the owner's team, and Apple has given no date. The plan's accepted-when ("all three OSes") now splits in two: D7c covers Windows and Linux, and this todo covers macOS.

## Context

- [D7c](../ongoing/D7c-desktop-beta-release.md): the Windows and Linux beta, its release notes, its download page and its update-cycle and kill-switch procedures, which this todo repeats for macOS.
- [D6e](./D6e-macos-signed-release-checks.md): the first signed and notarised macOS release, the clean-Mac checks and the Apple block this todo waits on.
- [D8d](../ongoing/D8d-app-store-listing-and-first-review.md): the Mac App Store listing, and the note on the download page about how the store build and the direct download differ. The store needs no notarisation, so the store track (D8c done, D8d under way) does not wait.
- `docs/guidelines/desktop-release.md`: "Cutting a release", the macOS verification commands, rollout and pause procedures.

## Scope

**In scope**

- **The macOS build in the beta channel**: after D6e, publish the direct download's signed and notarised `.dmg` (and the updater bundle) to `beta` through the D6 pipeline, as `desktop v0.1.x` where `x` is the first release that includes a macOS build. The rollout follows ADR 0014 and is recorded.
- **Release gates on macOS**, measured on the reference Mac with the numbers recorded, as D7c does for Windows and Linux: cold start (window shown < 300 ms, first command within 75 ms of the floor build, [ADR 0008 §4](../../decisions/0008-desktop-shell.md#4-tiles-lossless-webp-over-the-lamina-scheme-512-px-single-threaded)) and installer < 16 MB (owner decision 2026-11-15, [plan §4](../../plans/desktop-app.md#4-requirements)). A miss blocks this todo. The engine sandbox selftest also runs on macOS.
- **Notices and SBOM**: the macOS bundle carries the third-party notices, and its SBOM and attestation are generated ([ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md)). The privacy page D7c updated is not OS-specific and needs no second pass.
- **Download-page smoke test** on macOS 13 and the latest macOS, from `laminapdf.com/download`, run on the released beta artefacts:
  - install, then open a fixture by double-click;
  - use tabs and print one page;
  - open settings;
  - uninstall.

  [D6e](./D6e-macos-signed-release-checks.md) did the staging proof (Gatekeeper, notarisation staple, signed-to-signed update), so this todo does not repeat it. It verifies the released files with the runbook's macOS commands.
- **The update cycle on macOS**: install the first beta build that includes macOS, ship the next `0.1.x` through the same pipeline with staged rollout 10 % → 50 % → 100 %, and confirm the install updated without action beyond the restart prompt. Record the OS version, the install type and the time to update. The kill switch needs no second rehearsal: it works per channel, and D7c rehearsed it.
- **The download page's macOS entry** shows Beta, links the `.dmg`, and says how it differs from the Mac App Store build: the direct download updates itself and follows a channel; the store build is a separate install that updates through the store ([D8d](../ongoing/D8d-app-store-listing-and-first-review.md) records what happens with both installed). If D8d has not merged, the entry says only what is true then.
- **Release notes** at `/desktop/releases/<version>` say that macOS joined the beta, and drop the "macOS follows" line D7c added.
- **Known issues in the notes**: a line that lists what the desktop reader lacks and the web reader has, each linked to its todo until it ships: find ([D9c](./D9c-desktop-find-in-document.md)), the sidebar and outline ([D9d](../done/D9d-desktop-sidebar-and-outline.md)), reader mode ([D9e](./D9e-desktop-reader-mode.md)), Back and Forward ([D9f](../done/D9f-desktop-back-and-forward.md)), the network activity panel ([D9g](../done/D9g-desktop-network-activity-panel.md)) and page layouts ([D9h](./D9h-viewer-two-page-and-continuous-layouts.md)). The list loses an item as its todo merges. The published notes name the features in plain words and carry no todo IDs.

**Out of scope**

- The Mac App Store build, listing and review: [D8c](../done/D8c-app-store-build-and-upload.md) and [D8d](../ongoing/D8d-app-store-listing-and-first-review.md).
- Fixing Apple's notarisation block: the owner follows up with Apple, and D6e records the answer.
- Stable channel release: a later todo once beta criteria are met.

## Tasks

- [ ] Fresh worktree; claim in the work PR. Confirm that D6e is in `done/` and D7c has shipped.
- [ ] Release gates, notices and SBOM, and the download-page smoke test on macOS 13 and the latest macOS; collect evidence links.
- [ ] Release notes and the download page's macOS entry.
- [ ] Merge the PR before tagging; tag from `main`; the owner approves the `desktop-release` environment deployment.
- [ ] Verify the published macOS artefacts with the runbook commands, and `gh attestation verify`.
- [ ] After the tag, run the macOS update cycle (Post-merge checks) and record it in a follow-up PR that ticks it in the `done/` file.
- [ ] Update the plan; update the roadmap's M3 row only if the roadmap owner agrees.

## Acceptance criteria

- [ ] The signed and notarised macOS build is downloadable from `laminapdf.com/download` on the beta channel, and passes the runbook's macOS checks (Gatekeeper, notarisation staple) and `gh attestation verify`.
- [ ] Release gates met on macOS with numbers recorded (cold start, installer < 16 MB), and the engine sandbox selftest passes.
- [ ] The macOS bundle carries the third-party notices; its SBOM and attestation exist.
- [ ] On macOS 13 and the latest macOS the download from the page installs, launches and opens a fixture with no Gatekeeper warning beyond the standard first-launch prompt (recorded with OS versions).
- [ ] The download page's macOS entry shows Beta and says how the direct download differs from the Mac App Store build.
- [ ] No signing secret is printed in the release logs.

## Post-merge checks

- [ ] Tag the release from `main` after this PR merges and approve the `desktop-release` environment; the publish job and the post-publish Worker check are green.
- [ ] At least one macOS install on the beta channel auto-updated to the next `0.1.x` (recorded: OS version, install type, time to update).
- [ ] Both are recorded in a follow-up PR that ticks these boxes in the `done/` file.

## Tests

No new automated tests; the release relies on D6e's staging proof, the gate measurements and the download-page smoke test. A bug fixed for the update release comes with its regression test in that fix's PR.

## Docs to update

- `docs/plans/desktop-app.md`: tick the macOS line under D7 in the follow-up PR; M3 is complete.
- `docs/plans/roadmap.md`: the M3 row, once the roadmap owner agrees.
- `apps/desktop/CHANGELOG.md`, and the project `README.md` download mention.

## Risks and open questions

- **Apple's answer has no date.** This todo stays in `ready/` until D6e is done; the beta runs without macOS meanwhile.
- **Two installs on one Mac.** A person may have both the store build and the direct download ([D8d](../ongoing/D8d-app-store-listing-and-first-review.md)); the page copy must say which one updates how.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
