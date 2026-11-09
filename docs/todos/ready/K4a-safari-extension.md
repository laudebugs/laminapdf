# K4a — The extension on Safari

| Field | Value |
| --- | --- |
| ID | `K4a` |
| Status | `ready` |
| Plan | [Browser extension §5 K4](../../plans/browser-extension.md#k4-safari) |
| Round / milestone | Round K (browser extension) — after M2b |
| Depends on | [D6a](./D6a-signing-runbook-and-owner-setup.md), [K2c](./K2c-chromium-release.md) |
| Dependency notes | D6a sets up the Apple Developer Program account that the Safari app wrapper is signed with. |
| ADRs | Implements 0033's Safari section |
| Branch | `feat/k4a-safari-extension` |
| PR title | `feat(extension): ship the extension on Safari for macOS, iOS and iPadOS` |
| Size | L (the app wrapper, macOS CI signing, the Safari build and its release) |
| PR | — |

## Goal

Bring the extension to Safari users, button first. Safari can't recognise a PDF by its response headers, so the toolbar button and the context menu are the main ways in. Links whose URL ends in `.pdf` open automatically where Safari's redirect works. A macOS runner builds the containing app with `xcrun safari-web-extension-packager` and Xcode, signs it with the Apple account, and submits it through App Store Connect for macOS first, then iOS and iPadOS.

## Context

- [Platform note §2 and §3](../../research/browser-extension/notes/platform-and-pdf-interception.md#3-firefox-built-in-pdfjs-and-safari-pdf-handling): what Safari can and can't intercept, the redirect bugs (WECG #381, WebKit 303800).
- [Distribution note](../../research/browser-extension/notes/distribution-and-tooling.md): the packager, the browser-based App Store Connect packager, TestFlight, Developer ID outside the store.
- [D6a](./D6a-signing-runbook-and-owner-setup.md) and [D6c](./D6c-macos-signing-and-notarisation.md): the desktop app's Apple signing, which this todo reuses.
- K1a's README: whether Safari's `.pdf` redirect worked in the spike.

## Scope

**In scope**

- WXT's Safari target and the Xcode project the packager writes, committed as ADR 0033 says.
- Toolbar button, context menu, file picker, and `.pdf` URL redirects where they work.
- A macOS job in the release workflow: build, sign, upload to App Store Connect; TestFlight first.
- iOS and iPadOS once macOS is released.

**Out of scope** (and which todo covers it)

- Distribution outside the App Store with Developer ID: not planned unless review fails.

## Tasks

- [ ] Generate the app wrapper; set bundle IDs and signing from D6a's account.
- [ ] Adapt the extension for Safari's API gaps (no side panel, no omnibox, per-site permission prompts).
- [ ] Add the macOS release job and the owner's App Store Connect steps to `RELEASE.md`.
- [ ] Test by hand on macOS Safari and on an iPhone or iPad through TestFlight.

## Acceptance criteria

- [ ] On macOS Safari, the toolbar button and the context menu open a PDF from a host without CORS in Lamina PDF.
- [ ] A TestFlight build installs and works on macOS and iOS.
- [ ] App Store review accepts the macOS app.

## Post-merge checks

- [ ] The macOS app is live in the App Store; the iOS app follows.

## Tests

No automated Safari extension testing exists; record the manual test steps in `apps/extension/README.md`.

## Docs to update

- `apps/extension/README.md`, `RELEASE.md`, and the plan (tick K4a).

## Risks and open questions

- Safari 26.0 had a redirect regression; check the current Safari before relying on `.pdf` redirects.
- App Store review may question an app whose only purpose is an extension; describe the reader clearly in the listing.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
