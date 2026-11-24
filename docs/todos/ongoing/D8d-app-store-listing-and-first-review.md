# D8d — Mac App Store: the listing, the privacy label and the first review

| Field | Value |
| --- | --- |
| ID | `D8d` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D8](../../plans/desktop-app.md#d8-mac-app-store) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D8c](../done/D8c-app-store-build-and-upload.md) |
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
- In (agent, once the owner's secrets are set): the first real upload through `desktop-release.yml` (`app_store=upload`), which D8c built but couldn't run without the owner's certificates, profile and app record, and which also proves D8c's signed path (the certificates, `productbuild --sign`, `altool`) for the first time.
- Out: the build and the upload job themselves ([D8c](../done/D8c-app-store-build-and-upload.md)).

## Tasks

- [x] Fresh worktree; claim in the work PR
- [ ] Owner: certificates, profile and the app record (the certificates and profile go to D8c's secrets)
- [ ] The first upload: a `desktop-release.yml` dispatch with `only_os=macos`, `dry_run=false` and `app_store=upload`, approved by the owner; fix whatever the signed path finds; record the build number and App Store Connect's processing in the runbook's §8
- [x] The listing's text, screenshots and review notes, drafted for the owner ([`mac-app-store.md`](../../guidelines/mac-app-store.md); the screenshots are browser drafts to retake on a Mac)
- [ ] Owner: the privacy label and export compliance, then submit the first build. The build deliberately does not set `ITSAppUsesNonExemptEncryption`: the app implements the PDF standard's encryption, so whether it is exempt is the owner's answer to Apple in App Store Connect, not a claim the build makes. The guideline lays out the analysis and recommends an answer
- [ ] What happens with both installed (the store and the direct download share `com.laminapdf.desktop`): which one opens PDFs, whether the store updates a copy it didn't install, and that their settings stay apart; recorded, and said on the download page
- [ ] After approval: the download page's Mac App Store link; the runbook records the review and anything it asked for

## Acceptance criteria

- [ ] A release run uploads a package App Store Connect accepts and processes, and the build appears in TestFlight
- [ ] The app passes App Review and installs from the Mac App Store on a clean Mac, opening fixtures from Finder and the open panel
- [ ] The download page offers the store and the direct download, and says how they differ (the store updates the app; the direct download updates itself)

## Docs to update

- `docs/guidelines/mac-app-store.md` (new): its owner checklist moved into the runbook as §1.12
- `docs/guidelines/desktop-release.md`: the review, and how to submit each release
- The website's download page
- The website's support page (`apps/site/src/pages/support.astro`, new): the listing's Support URL

## Definition of done

- [ ] Work PR opened from a fresh worktree; its first commit moves this file `ready/` → `ongoing/` with `[skip ci]`; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## As built

Drafted by an agent on 2026-11-22, for the owner; nothing was run on a Mac, and nothing was uploaded or sent.

- **`docs/guidelines/mac-app-store.md`**, linked from the guidelines index. The owner's Apple checklist (App ID, the two certificates, the provisioning profile, the API key, the app record, the secrets) moved into the runbook's §1.12. The guideline holds the listing with character counts, the privacy label and export-compliance analysis, the App Review notes, the screenshot plan, the both-installed research and the per-release outline.
- **`tools/desktop/app-store-screenshots.mjs`** serves the built web reader and captures six scenes at 2880×1800 and 1440×900 into a folder outside the repository. Draft images exist; they show the web reader, and the final ones must be retaken from the real store build.
- **Code against the privacy story.** The store build sends nothing at run time, but `tauri-plugin-updater`, `reqwest` and `rustls` are linked in unconditionally, and the base `tauri.conf.json` still names the updater's key and endpoint; the "Check for updates…" menu items remain. Listed in §3.1 for the owner and D8c.
- **Export compliance.** The app decrypts and never encrypts. The build leaves `ITSAppUsesNonExemptEncryption` unset on purpose, so the owner answers Apple's questions in App Store Connect.
- **Still open, for a Mac:** which copy opens PDFs and whether the store replaces a direct copy (the steps are in §6), the copyright name, and the owner's phone number for App Review. The support page (`/support/`) is built; the owner chose the address `support@laminapdf.com`, and the mailbox may not exist yet, so the owner creates it before the page goes live.
- **Support page.** `apps/site` has a `/support/` page (words in `src/data/support.ts`, a footer link, a test that holds its crash-report sentence to the app's own text). It is the listing's Support URL. Its address and the footer line are new website copy for the owner to approve.
- **State on 2026-11-24.** The owner entered the listing in App Store Connect after an agent checked build 0.0.205 (105.2) on a Mac. Claims for items that the app greys out (Find, the sidebar and outline, themes, Reader mode, focus mode, crop and resize, reverse, back and forward) were removed from the description, the keywords and the review notes ([#793](https://github.com/laudebugs/laminapdf/issues/793)). Ten real-app screenshots (five scenes, light and dark, 2560×1600) were uploaded. The privacy label was answered "Data Not Collected" and export compliance "standard algorithms in addition to the OS's"; Apple let the build through to Ready to Submit. The owner removed China mainland (Apple lists a Security Assessment, an AI-content declaration and an ICP number as missing) and France (the French encryption declaration applies only there), so the app is available in 173 countries or regions and needs no encryption document now. Open: the owner's Publish of the label, Add for Review and Submit, and the owner's question about US export rules. The guideline's §8 has the details.
- **Document properties check, 2026-11-24.** Editing a title, Apply and File > Save wrote the new title to the file. Remove metadata cleared Author and Subject from the newest revision, but the save appends an incremental update, so the values stay in older revisions ([#795](https://github.com/laudebugs/laminapdf/issues/795)); the description therefore doesn't promise removal. Save a copy, Save as and Open With from the Dock are untried.
