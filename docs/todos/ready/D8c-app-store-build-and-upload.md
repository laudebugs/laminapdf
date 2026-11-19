# D8c — Mac App Store: the store build, its signing and the upload

| Field | Value |
| --- | --- |
| ID | `D8c` |
| Status | `ready` |
| Plan | [Desktop app §6 D8](../../plans/desktop-app.md#d8-mac-app-store) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D6e](./D6e-macos-signed-release-checks.md), [D8b](./D8b-app-sandbox-readiness.md) |
| ADRs | Implements [0014 §11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build) |
| Branch | `ci/d8c-app-store-build-and-upload` |
| PR title | `ci(desktop): build and upload the Mac App Store package` |
| Size | M (200–400 lines, mostly workflow and configuration) |
| PR | — |

## Goal

The release workflow builds the Mac App Store package beside the direct downloads: a universal app with the `app-store` feature and no updater. It signs the app with the Apple Distribution certificate, embeds the App Store provisioning profile, packages it with `productbuild` and the Mac Installer Distribution certificate, and uploads it to App Store Connect for the owner to submit.

## Context

- ADR 0014 §6 (the team and the API key) and §11; [Tauri: App Store](https://v2.tauri.app/distribute/app-store/); [Apple: upload builds](https://developer.apple.com/help/app-store-connect/manage-builds/upload-builds)
- The release runbook, `docs/guidelines/desktop-release.md`; D6c's macOS signing

## Scope

- In: the store's version and build number (up to three numbers, a build number that rises with each upload); the universal engine and PDFium; an API key role that can upload builds (Developer or App Manager); `tauri.appstore.conf.json` (no updater plugin, `bundle.macOS.entitlements`, the provisioning profile, `ITSAppUsesNonExemptEncryption`); the universal build; the store certificates and profile as secrets of the `desktop-release` environment; `productbuild`; the upload with the App Store Connect API key (`xcrun altool --upload-package` or the API); a check that the package contains no updater, no updater key and no `updates.laminapdf.com`; a dry-run mode for pull requests.
- Out: the owner's Apple setup and the first review ([D8d](D8d-app-store-listing-and-first-review.md)).

## Tasks

- [ ] Fresh worktree; claim in the work PR
- [ ] The store configuration and the universal build
- [ ] Signing, `productbuild` and the upload job, behind the `desktop-release` environment's approval
- [ ] The package check, and a dry run on pull requests that builds and checks the package without uploading
- [ ] One upload to App Store Connect, recorded in the runbook

## Acceptance criteria

- [ ] A release run uploads a package App Store Connect accepts and processes; the build appears in TestFlight
- [ ] The package's app has no updater and none of ADR 0014's updater endpoints or keys; `pkgutil --check-signature` names the Mac Installer Distribution certificate
- [ ] The direct-download jobs are unchanged

## Docs to update

- `docs/guidelines/desktop-release.md`: the store build, its secrets and the upload
- `docs/agents/ci-and-release.md`: the store job

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
