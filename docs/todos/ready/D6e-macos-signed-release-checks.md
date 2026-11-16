# D6e — macOS: prove a signed, notarised release on Apple's service and clean Macs

| Field | Value |
| --- | --- |
| ID | `D6e` |
| Status | `ready` |
| Plan | [Desktop app §6 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D6c](../done/D6c-macos-signing-and-notarisation.md) |
| ADRs | Verifies ADR 0014 §6 (macOS signing) |
| Branch | `ci/d6e-macos-signed-release-checks` |
| PR title | `ci(desktop): record the first signed and notarised macOS release` |
| Size | S (under 200 lines; mostly records, plus fixes the first signed run finds) |
| PR | — |

## Goal

The first `desktop-release` runs to the `staging` channel prove what [D6c](../done/D6c-macos-signing-and-notarisation.md) built: Apple notarises the app and the `.dmg`, every check in the build action's verification step passes, a signed build updates to the next signed build, and the `.dmg` opens on clean Macs with the owner shown as developer. D6c merged before any of this could run, because Apple's notary service didn't yet accept the owner's team.

## Context

- **Apple's block (this todo waits on it).** The first staging run (2026-11-15, [run 43765290088](https://github.com/laudebugs/laminapdf/actions/runs/43765290088)) imported the certificate and signed PDFium, the engine and the app with the Developer ID. Apple then answered submission `879e92c7-75af-423a-ae59-8f9ffa4c11b7` with status `Rejected`, statusCode 7000: "Team is not yet configured for notarization". Only Apple's Developer Programs Support can change that (topic Development and Technical → Other Development or Technical Questions); the owner asked them on 2026-11-15. Every way to notarise (`notarytool` with the API key or an Apple ID, Xcode) gets the same answer until then.
- **What D6c already proved.** Every macOS dry run draws a page with the bundled engine and PDFium; the signed path's keychain import, sidecar step and Tauri's signing ran in the run above.
- **Updater keys.** Both updater keys were replaced on 2026-11-15 (release `34929FE857511973`, nightly `46010ED395B59CAF`); the run above failed on Linux and Windows because the first release key's password didn't unlock it.
- Runbook: [desktop release §3 and §6](../../guidelines/desktop-release.md) (macOS checks, how a release is signed, troubleshooting).

## Scope

- In: two staging dispatches from `main`, fixes to what they find, the clean-Mac records.
- Out: Windows signing ([D6d](../ongoing/D6d-windows-signing.md)); the first tagged release ([D7c](./D7c-desktop-beta-release.md)).

## Tasks

- [ ] Fresh worktree; claim in the work PR. Confirm Apple has enabled notarisation (the owner forwards Apple's answer).
- [ ] Staging dispatch 1 (`gh workflow run desktop-release.yml -f channel=staging -f allow_unsigned_platforms=true`, the owner approves): both macOS legs green, the verification step included. Record the run.
- [ ] Note whether the runner's Gatekeeper gave `spctl`'s verdict or the step warned that assessments were off; if off, `spctl` on a clean Mac (below) stands in for it.
- [ ] Check the logs for the identity, the certificate password, the keychain password and the API key: masked or absent.
- [ ] Install build 1 from staging on a Mac, then run staging dispatch 2 and update to it through the updater.
- [ ] Clean Macs: macOS 13 and the latest macOS, without the owner's keychain. Download the `.dmg`, install, launch: only the standard first-launch prompt, naming the owner as developer. Open fixtures. Record OS versions and what each showed.
- [ ] Docs: tick the plan, record the runs in the runbook, move this file to `done/`.

## Acceptance criteria

- [ ] `spctl` accepts the `.app` as "Notarized Developer ID" and the `.dmg` passes its Gatekeeper check; `stapler validate` passes for both (CI step green, or `spctl` on a clean Mac where the runner had assessments off).
- [ ] On a clean macOS 13 machine and the latest macOS, the downloaded `.dmg` installs and launches with no Gatekeeper warning beyond the standard first-launch prompt showing the owner as developer (recorded with OS versions).
- [ ] The signed app opens fixtures, and its engine runs sandboxed: `Ready` reports the sandbox's layers (the shipped engine has no `--sandbox-selftest`; see D6c's notes).
- [ ] An installed signed build updates to the next signed build through the updater (two `desktop-release` staging dispatches).
- [ ] No signing secret is printed in the logs.

## Docs to update

- `docs/plans/desktop-app.md` §9 D6: the macOS part of "Verify installs on clean VMs".
- `docs/guidelines/desktop-release.md`: the first signed runs, and anything Apple's first answers teach the troubleshooting list.

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
