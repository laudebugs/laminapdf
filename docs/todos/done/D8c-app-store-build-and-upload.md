# D8c — Mac App Store: the store build, its signing and the upload

| Field | Value |
| --- | --- |
| ID | `D8c` |
| Status | `done` |
| Plan | [Desktop app §6 D8](../../plans/desktop-app.md#d8-mac-app-store) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D8b](./D8b-app-sandbox-readiness.md) |
| ADRs | Implements [0014 §11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build) |
| Branch | `ci/d8c-app-store-build-and-upload` |
| PR title | `ci(desktop): build and upload the Mac App Store package` |
| Size | M (200–400 lines, mostly workflow and configuration) |
| PR | [#785](https://github.com/laudebugs/laminapdf/pull/785) |

## Goal

The release workflow builds the Mac App Store package beside the direct downloads: a universal app with the `app-store` feature and no updater. It signs the app with the Apple Distribution certificate, embeds the App Store provisioning profile, packages it with `productbuild` and the Mac Installer Distribution certificate, and uploads it to App Store Connect for the owner to submit.

## Context

- ADR 0014 §6 (the team and the API key) and §11; [Tauri: App Store](https://v2.tauri.app/distribute/app-store/); [Apple: upload builds](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds)
- The release runbook, `docs/guidelines/desktop-release.md`; D6c's macOS signing

## Scope

- In: the store's version and build number (up to three numbers, a build number that rises with each upload); the universal engine and PDFium; an API key role that can upload builds (Developer or App Manager); `tauri.appstore.conf.json` (no updater plugin, `bundle.macOS.entitlements`; the provisioning profile goes in with the signing); the universal build; the store certificates and profile as secrets of the `desktop-release` environment; `productbuild`; the upload with the App Store Connect API key (`xcrun altool --upload-app`); a check that the package holds no updater key or configuration (the updater's and the crash reports' code stays linked but off in the store build; compiling it out is a follow-up); a dry-run mode for pull requests.
- Out: the owner's Apple setup, the first real upload and the first review ([D8d](../ready/D8d-app-store-listing-and-first-review.md)): the upload needs the certificates, profile and app record that D8d's owner steps produce, so D8d records it. `ITSAppUsesNonExemptEncryption` is left unset: the app implements the PDF standard's encryption, so whether it is exempt is the owner's answer to Apple (runbook §8.5), not a claim the build makes.

## Tasks

- [x] Fresh worktree; claim in the work PR
- [x] The store configuration and the universal build
- [x] Signing, `productbuild` and the upload job, behind the `desktop-release` environment's approval
- [x] The package check, and a dry run on pull requests that builds and checks the package without uploading
- [x] The runbook's §8: the secrets, what runs when, what the check proves
- [ ] ~~One upload to App Store Connect, recorded in the runbook~~ moved to [D8d](../ready/D8d-app-store-listing-and-first-review.md), which holds the owner's Apple setup it needs

## Acceptance criteria

- [x] A pull request's dry run builds the universal app, signs it ad hoc with the store's entitlements, packages it and passes `check.sh`, with no secret and no upload
- [x] The package check fails unless the app has none of the updater's key or configuration, the entitlements are exactly the store's, and a signed package names the Apple Distribution and Mac Installer Distribution certificates
- [x] The upload job reads the store secrets only in `desktop-release`, after approval, and names the missing one (tested by the workflow checker)
- [x] The direct-download jobs are unchanged
- Moved to D8d: a release run uploads a package App Store Connect accepts and processes, and the build appears in TestFlight

## Docs to update

- `docs/guidelines/desktop-release.md`: the store build, its secrets and the upload
- `docs/agents/ci-and-release.md`: the store job

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
