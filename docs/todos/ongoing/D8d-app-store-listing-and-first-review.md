# D8d — Mac App Store: the listing, the privacy label and the first review

| Field | Value |
| --- | --- |
| ID | `D8d` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D8](../../plans/desktop-app.md#d8-mac-app-store) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D8c](../ready/D8c-app-store-build-and-upload.md) |
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
- Out: the build and its upload ([D8c](../ready/D8c-app-store-build-and-upload.md)).

## Tasks

- [x] Fresh worktree; claim in the work PR
- [ ] Owner: certificates, profile and the app record (the certificates and profile go to D8c's secrets)
- [x] The listing's text, screenshots and review notes, drafted for the owner ([`mac-app-store.md`](../../guidelines/mac-app-store.md); the screenshots are browser drafts to retake on a Mac)
- [ ] Owner: the privacy label and export compliance, then submit the first build. The build deliberately does not set `ITSAppUsesNonExemptEncryption`: the app implements the PDF standard's encryption, so whether it is exempt is the owner's answer to Apple in App Store Connect, not a claim the build makes. The guideline lays out the analysis and recommends an answer
- [ ] The first upload through `desktop-release.yml` (`app_store=upload`), with its build number and App Store Connect's processing recorded in §8 of the runbook (moved here from D8c, which can't upload until the owner has the certificates, the profile and the app record)
- [ ] What happens with both installed (the store and the direct download share `com.laminapdf.desktop`): which one opens PDFs, whether the store updates a copy it didn't install, and that their settings stay apart; recorded, and said on the download page
- [ ] After approval: the download page's Mac App Store link; the runbook records the review and anything it asked for

## Acceptance criteria

- [ ] The first upload through `desktop-release.yml` (`app_store=upload`) is done, with its build number and App Store Connect's processing recorded in §8 of `docs/guidelines/desktop-release.md`
- [ ] The app passes App Review and installs from the Mac App Store on a clean Mac, opening fixtures from Finder and the open panel
- [ ] The download page offers the store and the direct download, and says how they differ (the store updates the app; the direct download updates itself)

## Docs to update

- `docs/guidelines/mac-app-store.md` (new): its owner checklist moves into the runbook as §1.12 once the D8c runbook change merges
- `docs/guidelines/desktop-release.md`: the review, and how to submit each release
- The website's download page

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

Drafted by an agent on 2026-11-22, for the owner; nothing was run on a Mac, and nothing was uploaded or sent.

- **`docs/guidelines/mac-app-store.md`**, linked from the guidelines index. Its §1.12 block is the owner's Apple checklist (App ID, the two certificates, the provisioning profile, the API key, the app record, the secrets), shaped to move into the runbook unchanged. The rest holds the listing with character counts, the privacy label and export-compliance analysis, the App Review notes, the screenshot plan, the both-installed research and the per-release outline.
- **`tools/desktop/app-store-screenshots.mjs`** serves the built web reader and captures six scenes at 2880×1800 and 1440×900 into a folder outside the repository. Draft images exist; they show the web reader, and the final ones must be retaken from the real store build.
- **Code against the privacy story.** The store build sends nothing at run time, but `tauri-plugin-updater`, `reqwest` and `rustls` are linked in unconditionally, and the base `tauri.conf.json` still names the updater's key and endpoint; the "Check for updates…" menu items remain. Listed in §3.1 for the owner and D8c.
- **Export compliance.** The app decrypts and never encrypts. The build leaves `ITSAppUsesNonExemptEncryption` unset on purpose, so the owner answers Apple's questions in App Store Connect.
- **Still open, for a Mac:** which copy opens PDFs and whether the store replaces a direct copy (the steps are in §6), the support page and address, the copyright name, and the owner's contact details.
- **Also fixed:** two todo links broken by the claim (this file's to D8c and D8c's to this file).
