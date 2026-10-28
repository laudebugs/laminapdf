# D6c — macOS: Developer ID signing, hardened runtime, notarisation and stapling

| Field | Value |
| --- | --- |
| ID | `D6c` |
| Status | `ready` |
| Plan | [Desktop app §6 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D6a](./D6a-signing-runbook-and-owner-setup.md), [D6b](./D6b-release-workflow-and-linux-packages.md) |
| Blocks | [D7c](./D7c-desktop-beta-release.md) |
| ADRs | Implements ADR 0014 (macOS identity); respects ADR 0008 / D2c (engine helper entitlements) |
| Branch | `ci/d6c-macos-signing` |
| PR title | `ci(desktop): sign, notarise and staple macOS builds with the owner's Developer ID` |
| Size | M (200–600 lines) |
| PR | — |

## Goal

Every macOS artefact lamina publishes — the `.app` inside the updater `.app.tar.gz` and the `.dmg` for download — is signed with the owner's **Developer ID Application** identity with the hardened runtime, notarised by Apple and stapled, so Gatekeeper opens it on a clean Mac without warnings and shows the owner as the developer. The sandboxed engine helper (D2c) and the bundled PDFium library are signed with the right entitlements so the sandbox keeps working in signed builds.

## Context

- [Desktop plan §6 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline): dmg (Developer ID + notarisation + stapling); Gatekeeper checks on clean VMs.
- `docs/guidelines/desktop-release.md` (D6a): secret names, verification commands.
- [Desktop notes Q6](../../research/architecture/notes/desktop-shell.md#q6-code-signing--notarization-linux-packaging-ci): Tauri notarisation via App Store Connect API key env vars (`APPLE_API_ISSUER`, `APPLE_API_KEY`, `APPLE_API_KEY_PATH`) or Apple ID + app-specific password + team ID.
- Tauri [macOS signing](https://v2.tauri.app/distribute/sign/macos/): `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`; `bundle.macOS.signingIdentity`, `bundle.macOS.entitlements`, `bundle.macOS.hardenedRuntime` (default true), `bundle.macOS.minimumSystemVersion`; Tauri signs nested `externalBin` sidecars and frameworks, notarises and staples the `.app`.
- Apple: [notarising](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution) (`xcrun notarytool submit --wait`, `xcrun stapler staple`), hardened runtime entitlements reference.
- D2c: `engine.entitlements` (`com.apple.security.app-sandbox`) for the helper.

## Scope

**In scope**

- `tauri.conf.json`: `bundle.macOS.minimumSystemVersion: "13.0"`, `hardenedRuntime: true`, `entitlements: "entitlements/app.entitlements"` (main app: no sandbox; only what WKWebView needs — expected none beyond defaults; document any added, e.g. `com.apple.security.cs.allow-jit` is **not** needed for WKWebView), signing identity from env.
- Engine helper signing: ensure the sidecar is signed with `engine.entitlements` (app sandbox) and hardened runtime — if Tauri signs sidecars with the app's entitlements, add a pre-sign step that signs the helper with its own entitlements first and verify Tauri preserves it (or sign in a post-build step and re-sign the outer app); PDFium `.dylib` signed with the Developer ID.
- Universal or per-arch (as decided in D6b): if universal, `lipo` the engine sidecar and PDFium.
- CI (in D6b's workflow, macOS jobs): import the certificate into a temporary keychain (tauri-action does this when `APPLE_CERTIFICATE` is set; otherwise `security create-keychain`/`import`/`set-key-partition-list`), write the API key `.p8` to `APPLE_API_KEY_PATH`, build → Tauri notarises and staples the `.app`; then **notarise and staple the `.dmg`** separately (`xcrun notarytool submit Lamina.dmg --key … --key-id … --issuer … --wait` and `xcrun stapler staple Lamina.dmg`), because the dmg container needs its own ticket for offline Gatekeeper checks.
- Updater artefact: the `.app.tar.gz` is created from the signed, stapled `.app` and signed with the updater key (order matters: verify Tauri's ordering).
- Verification steps in CI after build: `codesign --verify --deep --strict --verbose=2 Lamina.app`, `codesign -d --entitlements - <helper>` shows the sandbox entitlement, `spctl -a -vvv -t exec Lamina.app` → `accepted, source=Notarized Developer ID`, `spctl -a -vvv -t open --context context:primary-signature Lamina.dmg`, `xcrun stapler validate` on both.
- Lift the D6b guard for macOS (signed platforms may publish to beta/stable once Windows is also signed — D6d).
- Clean-VM checks: fresh macOS 13 and the latest macOS (VM or spare Mac): download the dmg via a browser (quarantine attribute set), open, drag to Applications, first launch shows only the standard "downloaded from the Internet" prompt naming the developer, no "unidentified developer"/"damaged" errors; engine sandbox selftest passes in the signed build; a signed build updates to the next signed build (nightlies are not OS-signed — D6b — so use two `workflow_dispatch` runs of the release workflow to the `staging` channel, each approved in the `release` environment).

**Out of scope** (and which todo covers it)

- Mac App Store packaging (non-goal).
- Windows signing: [D6d](D6d-windows-signing.md).

## Tasks

- [ ] Fresh worktree; this todo is already claimed via the claim PR (see DoD). Load `tauri-v2`.
- [ ] Entitlements files and `tauri.conf.json` macOS keys.
- [ ] Helper/PDFium signing order; verify with `codesign -dvvv` in CI.
- [ ] dmg notarisation + stapling step; retry/backoff around `notarytool` (Apple service hiccups) and a timeout.
- [ ] Verification step that fails the job on any check failure.
- [ ] Clean-VM checklist results in the PR.
- [ ] Update the runbook with exact commands and troubleshooting (common notarisation rejections: unsigned nested binaries, missing hardened runtime, secure timestamp).

## Acceptance criteria

- [ ] `spctl` accepts the `.app` as "Notarized Developer ID" and the `.dmg` passes its Gatekeeper check; `stapler validate` passes for both (CI step green).
- [ ] On a clean macOS 13 VM and the latest macOS, the downloaded dmg installs and launches with no Gatekeeper warning beyond the standard first-launch prompt showing the owner as developer (recorded with OS versions).
- [ ] The engine helper in the signed build runs sandboxed (`--sandbox-selftest` all blocked) and the app opens fixtures.
- [ ] An installed signed build updates to the next signed build via the updater (two `release`-environment `workflow_dispatch` runs to the `staging` channel; nightlies stay ad-hoc signed per D6b).
- [ ] No signing secrets are printed in logs (masking verified).

## Post-merge checks

- [ ] The first scheduled nightly after merge still builds macOS ad-hoc signed in the `nightly` environment (no Apple secrets referenced).

## Tests

- CI verification steps above (they are the tests).
- WebdriverIO smoke on the signed build is optional (the e2e job uses debug builds); the clean-VM checklist is required.

## Docs to update

- `docs/plans/desktop-app.md` §9 D6: tick "Apple Developer ID signing, notarisation and stapling" and the macOS part of "Verify installs on clean VMs".
- `docs/guidelines/desktop-release.md`: macOS commands and troubleshooting.
- ADR 0014: any deviation (e.g. per-arch vs universal).

## Risks and open questions

- **Sidecar entitlements** may be overwritten by Tauri's bundler signing; the pre-/post-sign approach must be verified each Tauri upgrade (add a CI assertion).
- **Notarisation latency** (minutes to hours) can stall releases; `--wait` with a generous timeout and a resumable "staple later" path.
- **Apple certificate expiry** (5 years) and Apple Developer Program renewal (yearly) — calendar in the runbook; an expired membership doesn't invalidate already-notarised builds but blocks new ones.
- **API key for individual accounts**: if unavailable, the Apple ID + app-specific password path is used (D6a records which).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
