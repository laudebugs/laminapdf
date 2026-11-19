# D8d — Mac App Store: the listing, the privacy label and the first review

| Field | Value |
| --- | --- |
| ID | `D8d` |
| Status | `ready` |
| Plan | [Desktop app §6 D8](../../plans/desktop-app.md#d8-mac-app-store) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D8c](./D8c-app-store-build-and-upload.md) |
| ADRs | Follows [0014 §9–§11](../../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build) |
| Branch | `docs/d8d-app-store-listing` |
| PR title | `docs(desktop): the Mac App Store listing and its first review` |
| Size | S (mostly the owner's work in App Store Connect, and records) |
| PR | — |

## Goal

Lamina PDF is on the Mac App Store. The owner sets up the store side in App Store Connect, the first build passes review, and the website's download page offers the store beside the direct download.

## Context

- ADR 0014 §9 (privacy) and §11; [Apple: App privacy details](https://developer.apple.com/app-store/app-privacy-details/); [App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)
- The download page (`apps/site`, D7b)

## Scope

- In (owner): the Apple Distribution and Mac Installer Distribution certificates; the App Store provisioning profile for `com.laminapdf.desktop`; the app record (name, subtitle, description, keywords, category Productivity, screenshots, support and privacy URLs, price free); the privacy label (no data collected: the store build sends nothing, ADR 0014 §11); export compliance; submitting the first build. In (agent): the listing's text and screenshots drafted from the website and the reader, the review notes, and the download page's store link.
- Out: the build and its upload ([D8c](D8c-app-store-build-and-upload.md)).

## Tasks

- [ ] Fresh worktree; claim in the work PR
- [ ] Owner: certificates, profile and the app record (the certificates and profile go to D8c's secrets)
- [ ] The listing's text, screenshots and review notes, drafted for the owner
- [ ] Owner: the privacy label and export compliance, then submit the first build
- [ ] What happens with both installed (the store and the direct download share `com.laminapdf.desktop`): which one opens PDFs, whether the store updates a copy it didn't install, and that their settings stay apart; recorded, and said on the download page
- [ ] After approval: the download page's Mac App Store link; the runbook records the review and anything it asked for

## Acceptance criteria

- [ ] The app passes App Review and installs from the Mac App Store on a clean Mac, opening fixtures from Finder and the open panel
- [ ] The download page offers the store and the direct download, and says how they differ (the store updates the app; the direct download updates itself)

## Docs to update

- `docs/guidelines/desktop-release.md`: the review, and how to submit each release
- The website's download page

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
