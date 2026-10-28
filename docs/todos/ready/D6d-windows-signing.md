# D6d — Windows: Authenticode signing (Artifact Signing or OV in a cloud HSM), MSI and NSIS

| Field | Value |
| --- | --- |
| ID | `D6d` |
| Status | `ready` |
| Plan | [Desktop app §6 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D6a](./D6a-signing-runbook-and-owner-setup.md), [D6b](./D6b-release-workflow-and-linux-packages.md) |
| Blocks | [D7c](./D7c-desktop-beta-release.md) |
| ADRs | Implements ADR 0014 (Windows identity, path recorded by D6a) |
| Branch | `ci/d6d-windows-signing` |
| PR title | `ci(desktop): Authenticode-sign Windows MSI and NSIS builds with the owner's identity` |
| Size | M (200–600 lines) |
| PR | — |

## Goal

Every Windows binary lamina ships — `lamina.exe`, the engine sidecar, PDFium's DLL, the NSIS installer and uninstaller, and the MSI — carries a valid, timestamped Authenticode signature showing **the owner as publisher**, using whichever path D6a recorded: **Azure Artifact Signing** (if the owner is eligible as a US/Canada individual) or an **OV certificate whose key lives in a cloud HSM**. Both MSI (managed deployment) and NSIS (per-user install + updater) are produced. SmartScreen behaviour on a clean VM is recorded and handled as the plan specifies: no warning with Artifact Signing; a warning is acceptable for early releases with OV, and the release notes explain how reputation builds.

## Context

- [Desktop plan §6 D6 accepted-when](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline) (Authenticode valid, owner as publisher; SmartScreen expectations per path; neither OV nor EV gets instant reputation since 2024) and [§7 risks](../../plans/desktop-app.md#7-risks).
- `docs/guidelines/desktop-release.md` + ADR 0014 amendment (D6a): which path, tool, secret names.
- [Desktop notes Q6](../../research/architecture/notes/desktop-shell.md#q6-code-signing--notarization-linux-packaging-ci): `bundle.windows.signCommand` with `%1`; Artifact Signing eligibility; HSM requirement.
- Tauri [Windows signing](https://v2.tauri.app/distribute/sign/windows/): `bundle.windows.signCommand` (custom command; `%1` = file path; Tauri calls it for the app binary, sidecars, installers and the NSIS uninstaller), `certificateThumbprint`/`digestAlgorithm`/`timestampUrl` for local certificates; [Windows installer config](https://v2.tauri.app/distribute/windows-installer/): `bundle.windows.nsis` (`installMode: "currentUser" | "perMachine" | "both"`), `bundle.windows.wix`, `webviewInstallMode` (`downloadBootstrapper` / `embedBootstrapper` / `offlineInstaller`).
- Microsoft: [Artifact Signing quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart), SignTool with the Artifact Signing dlib (`/dlib Azure.CodeSigning.Dlib.dll /dmdf metadata.json`), `azure/login` with OIDC.

## Scope

**In scope**

- `tauri.conf.json` `bundle.windows`:
  - `signCommand` invoking a repo script `apps/desktop/scripts/sign-windows.{ps1,sh}` with `%1`, which dispatches on an env var `LAMINA_WIN_SIGN_PATH=artifact-signing|ov-cloud-hsm`:
    - **Artifact Signing**: `signtool sign /v /fd SHA256 /tr http://timestamp.acs.microsoft.com /td SHA256 /dlib <path>\Azure.CodeSigning.Dlib.dll /dmdf metadata.json %1` (metadata: endpoint, account, certificate profile) after `azure/login` via OIDC (no client secret); or `trusted-signing-cli` if preferred (licence check).
    - **OV in cloud HSM**: the CA's headless tool or `jsign` (`--storetype` per HSM: `AZUREKEYVAULT`, `DIGICERTONE`, `ESIGNER`, …) with RFC 3161 timestamping (`--tsaurl`), SHA-256.
  - `digestAlgorithm: "sha256"`, `timestampUrl` only if using local thumbprint signing (not used).
  - `nsis`: `installMode: "currentUser"` (no elevation, updater-friendly) — or `"both"` if the owner wants per-machine NSIS; `languages`, `displayLanguageSelector: false`; installer icon/header images.
  - `wix`: per-machine MSI for managed deployment (`upgradeCode` fixed forever — generate once, record in ADR 0014), `language: "en-US"`.
  - `webviewInstallMode: { type: "downloadBootstrapper" }` for NSIS (small) and `embedBootstrapper` for MSI (offline-friendlier) — decide with size budget.
  - File associations from D3a verified in both installers.
- Signing coverage check: a CI step listing every PE file in the build output and installers (extract MSI with `lessmsi` (MIT) or `msiexec /a`; NSIS with `7z`) and running `signtool verify /pa /v` on each; fails if any is unsigned or the signer subject isn't the owner's name.
- Lift the D6b guard for Windows.
- Clean-VM checks (Windows 11 current, fresh, SmartScreen on, Defender on): download each installer via Edge (Mark-of-the-Web set), run; record SmartScreen behaviour, UAC prompt publisher name (MSI), installed app's signature (`Get-AuthenticodeSignature`), file association registration, uninstall cleanliness; update from previous nightly via NSIS updater artefact.
- Windows 10 22H2: best-effort smoke only (no CI), per the plan's supported-OS note.
- Release-notes snippet (OV path) explaining SmartScreen "Windows protected your PC → More info → Run anyway" during the reputation period, added to the runbook template.

**Out of scope** (and which todo covers it)

- Microsoft Store/MSIX (non-goal).
- EV certificates (no reputation advantage since 2024; not planned).
- macOS: [D6c](D6c-macos-signing-and-notarisation.md).

## Tasks

- [ ] Fresh worktree; move this file to `ongoing/`. Load `tauri-v2`.
- [ ] Signing script(s) with strict error handling (non-zero exit on failure; retries on transient HSM/timestamp errors; no secret echo).
- [ ] `bundle.windows` config; generate and record the WiX `upgradeCode`.
- [ ] OIDC login step (Artifact Signing) or credential setup (OV) in the Windows matrix jobs; arm64 job signs the same way.
- [ ] Signing coverage check step.
- [ ] Clean-VM checklist with results in the PR (SmartScreen screenshots allowed; no secrets).
- [ ] Runbook updates (commands, troubleshooting, SmartScreen notes).

## Acceptance criteria

- [ ] Every PE file in both installers and the installed app has a valid SHA-256 Authenticode signature with an RFC 3161 timestamp, signer = the owner's legal name (CI coverage check green).
- [ ] On a clean Windows 11 VM: UAC (MSI) shows the owner as verified publisher; the installed app's `Get-AuthenticodeSignature` status is `Valid`.
- [ ] SmartScreen: with Artifact Signing, the installer runs **without** a SmartScreen warning; with OV, any warning is recorded and the release notes text is in place (both outcomes per the plan).
- [ ] MSI and NSIS both install, register `.pdf` "Open with", and uninstall cleanly (manual check recorded).
- [ ] An NSIS-installed signed nightly updates to the next signed nightly.

## Tests

- CI signing coverage check (above); `signtool verify` per file.
- Clean-VM checklist in the PR.

## Docs to update

- `docs/plans/desktop-app.md` §9 D6: tick "Windows signing via `signCommand` (Artifact Signing or the OV certificate); MSI and NSIS outputs" and complete "Verify installs on clean VMs (Gatekeeper, Windows signature and publisher, SmartScreen behaviour recorded, Linux signature checks)" once D6c's macOS part is also done.
- `docs/guidelines/desktop-release.md`: Windows commands, SmartScreen notes.
- ADR 0014: WiX upgrade code, install modes.

## Risks and open questions

- **Cloud HSM rate limits/latency**: Tauri signs many files; batch or cache sessions where the tool allows; retries.
- **Artifact Signing region/eligibility changes**: if eligibility lapses, switch to the OV path via `LAMINA_WIN_SIGN_PATH` (both paths kept working in the script).
- **Per-user NSIS vs per-machine MSI**: the updater installs the same flavour as installed; MSI users on managed machines may get updates through their management tool instead — document; D5b's policy key can disable updates.
- **SmartScreen reputation** with OV takes weeks/thousands of downloads; early beta users will see warnings — acceptable per plan.
- **arm64 signing** with the same tools: verify `signtool`/`jsign` handle ARM64 PE files (they do for Authenticode; confirm).

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
