# D6a — Signing runbook and owner setup (certificates, accounts, secrets, Windows eligibility)

| Field | Value |
| --- | --- |
| ID | `D6a` |
| Status | `ongoing` |
| Plan | [Desktop app §6 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline) |
| Round / milestone | Round D (desktop) — M3 Desktop reader beta |
| Depends on | [D2a](../done/D2a-adr-0008-and-desktop-scaffold.md), [D5a](../ready/D5a-update-worker-and-adr-0014.md) |
| ADRs | Amends ADR 0014 (records the Windows path actually taken and the identities' details). Follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `docs/d6a-signing-runbook` |
| PR title | `docs(release): add the desktop signing runbook and record the owner's signing setup` |
| Size | S–M (docs only, ~300–500 lines) |
| PR | [#517](https://github.com/laudebugs/laminapdf/pull/517) |

## Goal

The project owner signs the desktop app **personally, as the publisher**, on Windows, macOS and Linux (decided 2026-10-28, [architecture §8 Q5](../../plans/architecture.md#8-open-questions-for-the-team)). None of the signing PRs (D6b–D6d) can merge until the owner holds the identities and CI holds the secrets. This todo produces the runbook that tells the owner exactly what to do, records **which Windows path applies** (Azure Artifact Signing is open to individuals only in the US and Canada; otherwise an OV certificate whose key lives in a cloud HSM), and inventories every secret by name and location so key custody and renewal are auditable. It merges once the owner has completed the checklist.

## Context

- [Desktop plan §6 D6](../../plans/desktop-app.md#d6-signing-packaging-and-release-pipeline) and [§7 risks](../../plans/desktop-app.md#7-risks) (Windows signing eligibility; losing the updater key).
- [Architecture §6 risks](../../plans/architecture.md#6-risks): signing identities and keys; [roadmap decisions](../../plans/roadmap.md): code signing by the owner (M3).
- [Desktop notes Q6](../../research/architecture/notes/desktop-shell.md#q6-code-signing--notarization-linux-packaging-ci): Tauri macOS notarisation env vars; `bundle.windows.signCommand`; Artifact Signing individual eligibility (US/Canada only, not for Private Trust); since 2023 OV/EV keys must be on HSMs; EV no longer grants instant SmartScreen reputation.
- ADR 0014 (D5a): identities, custody, rotation; the updater key ceremony (done in D5a).
- Primary docs to cite in the runbook: [Microsoft Artifact Signing quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart), [Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/), [Tauri Linux signing](https://v2.tauri.app/distribute/sign/linux/), [Apple notarisation](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution), [GitHub environments](https://docs.github.com/actions/deployment/targeting-different-environments/using-environments-for-deployment), [GitHub OIDC with Azure](https://docs.github.com/actions/security-for-github-actions/security-hardening-your-deployments/configuring-openid-connect-in-azure).

## Scope

**In scope**

- `docs/guidelines/desktop-release.md` (new; index in `docs/guidelines/README.md`): the release and signing runbook, with:
  1. **Owner setup checklist** (below), each item with cost, lead time, where the result is stored, and renewal date.
  2. **Secrets inventory**: name, purpose, where stored (secret manager entry name, GitHub environment `release` or `nightly`), who can read it, rotation/renewal cadence. Never values.
  3. **Verification commands** per OS (used in D6b–D6d and every release): `codesign --verify --deep --strict --verbose=2`, `spctl -a -vvv -t exec Lamina.app`, `spctl -a -vvv -t open --context context:primary-signature Lamina.dmg`, `xcrun stapler validate`; `signtool verify /pa /v`, `Get-AuthenticodeSignature`; `gpg --verify`, `debsig-verify`, `rpm -K`, AppImage signature validation; `gh attestation verify`.
  4. **Renewal and incident procedures**: Developer ID certificate expiry (5 years) and renewal; OV certificate yearly renewal; Artifact Signing identity revalidation; GPG subkey expiry (2 years) and extension; compromise of any key (revoke, pause channels per ADR 0014, re-sign, notify).
  5. **SmartScreen policy**: a warning is expected on the first downloads with either path (corrected 2026-11-12: Microsoft gives Artifact Signing no instant reputation either) and is acceptable for early releases, reputation builds with downloads; release notes template text explaining "More info → Run anyway" during the reputation period.
- ADR 0014 amendment: the Windows path taken (Artifact Signing vs OV + which CA/HSM and signing tool), the macOS team ID, the Linux GPG fingerprint, the publisher name as shown on each OS.

**Owner setup checklist (to complete before this PR merges)**

- [ ] **Owner — bundle identity confirmed**: product name "Lamina" and bundle identifier from D2a; the publisher name will be the owner's legal name on every OS.
- [ ] **Owner — Apple Developer Program (individual)**: enrol (annual fee; identity verification can take days); note the Team ID.
- [ ] **Owner — Developer ID Application certificate**: create via Xcode or a CSR from Keychain Access; export as `.p12` with a strong password; store in the secret manager; add GitHub `release`-environment secrets `APPLE_CERTIFICATE` (base64 of the `.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` (`Developer ID Application: <Name> (<TEAMID>)`), `KEYCHAIN_PASSWORD` (random, for the CI temporary keychain).
- [ ] **Owner — notarisation credentials**: App Store Connect API key (preferred; `.p8` file + Key ID + Issuer ID) → `APPLE_API_KEY` (Key ID), `APPLE_API_ISSUER`, `APPLE_API_KEY_P8` (contents, written to a file at `APPLE_API_KEY_PATH` in CI). If an individual account cannot create a team API key, use Apple ID + app-specific password + team ID (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`) and record why.
- [ ] **Owner — Windows eligibility check**: confirm whether the owner qualifies for Azure Artifact Signing **public trust as an individual** (must be located in the US or Canada; verify on the current quickstart page and record the date and outcome in ADR 0014).
  - [ ] **If eligible**: Azure subscription; create an Artifact Signing account (region close to CI runners); complete individual identity validation (Public Trust); create a Public Trust certificate profile; create an Entra app registration with a **federated credential for GitHub OIDC** limited to this repository's `release` environment; assign it the "Artifact Signing Certificate Profile Signer" role on the profile; record endpoint URL, account name, profile name (not secrets) and add `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_SUBSCRIPTION_ID` to the `release` environment.
  - [ ] **If not eligible**: buy an **OV code-signing certificate issued to the owner as an individual** from a CA that validates individuals and offers cloud-HSM key storage usable headlessly from CI (candidates to compare: SSL.com eSigner, DigiCert KeyLocker, Certum cloud signing, or Azure Key Vault Premium/Managed HSM with a CA that accepts Key Vault CSRs); complete validation; record CA, product, HSM, signing tool (e.g. `jsign` (Apache-2.0), `AzureSignTool`, `smctl`, `CodeSignTool`) and add its credentials (e.g. TOTP seed for eSigner, client cert for KeyLocker) to the `release` environment.
- [ ] **Owner — Linux GPG key**: create an offline primary key (ed25519) with a signing subkey (2-year expiry) for AppImage/deb/rpm; export the subkey to CI as `LINUX_SIGNING_KEY` (armored, passphrase-protected) + `LINUX_SIGNING_KEY_PASSPHRASE`; publish the public key at `https://laminapdf.com/keys/lamina-release.asc` (a file of the website, `apps/site/public/keys/`) and on keys.openpgp.org; record the fingerprint in ADR 0014 and the runbook.
- [ ] **Owner — updater key** (generated in D5a): confirm `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` exist in the `release` environment and the offline backups exist in two places.
- [ ] **Owner — GitHub `release` environment** (tagged releases and owner-run `workflow_dispatch`): required reviewer = owner; deployment branches/tags limited to `desktop-v*` tags, `main`, and — temporarily, so the signing PRs can prove their steps before merging — `ci/d6c-*` and `ci/d6d-*` (every run still waits for the owner's approval; D6d removes the two branch patterns when it merges); holds every production secret above; none at repository level.
- [ ] **Owner — GitHub `nightly` environment** (scheduled nightlies; **no required reviewer**, so the cron job isn't blocked waiting for approval): deployment branches limited to `main`; holds **only nightly-scoped secrets** — the nightly updater key from D5a (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, same names as in `release`, different values) and a Cloudflare API token scoped to the nightly R2 bucket only (`CLOUDFLARE_API_TOKEN`, `CLOUDFLARE_ACCOUNT_ID`). **No** Apple certificate, notarisation credentials, Windows signing credentials (and no Azure federated credential for this environment) or Linux GPG key: nightlies are not OS-signed (decided in D6b: macOS ad-hoc, Windows unsigned, Linux packages unsigned with `SHA256SUMS` only), so a compromised nightly run can't produce anything signed with the owner's identity or push an update to beta/stable installs.
- [ ] **Owner — Cloudflare** (from D5a): R2 buckets (`lamina-pdf-releases`, `lamina-pdf-nightly`), custom domains, two scoped API tokens — one for `lamina-pdf-releases` in the `release` environment, one for `lamina-pdf-nightly` in the `nightly` environment — each as `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID`.

**Out of scope** (and which todo covers it)

- Wiring the secrets into workflows: [D6b](../ready/D6b-release-workflow-and-linux-packages.md) (pipeline + Linux), [D6c](../ready/D6c-macos-signing-and-notarisation.md) (macOS), [D6d](../ready/D6d-windows-signing.md) (Windows).
- Mac App Store / Microsoft Store identities (non-goals).

## Tasks

- [ ] Fresh worktree; claim this todo via the claim PR (see DoD) as soon as the owner starts the checklist, so the board shows it in progress.
- [ ] Write `docs/guidelines/desktop-release.md` with sections 1–5 above; cite primary sources; mark recommendations as such.
- [ ] Owner works through the checklist; the PR description tracks progress (ticks only, no secret values or screenshots of secrets).
- [ ] Amend ADR 0014 with the outcomes.
- [ ] Add a calendar of renewals (dates) to the runbook.

## Acceptance criteria

- [ ] Every checklist item is ticked by the owner, or explicitly marked "not applicable" with the reason (e.g. Artifact Signing branch vs OV branch).
- [ ] ADR 0014 records: Windows eligibility outcome with date and source, the chosen Windows signing path, Apple Team ID, Linux GPG fingerprint, publisher name per OS.
- [ ] The secrets inventory lists every secret D6b–D6d will reference, by exact name, each scoped to the `release` or `nightly` environment (never the repository); the `nightly` environment contains no OS-signing secret and no production updater key.
- [ ] No secret values, private keys or certificate files appear anywhere in the repository (reviewer check + `gitleaks`-style scan if available in CI).

## Post-merge checks

None.

## Tests

None (documentation and account setup). The secrets are exercised by D6b–D6d's workflows.

## Docs to update

- `docs/guidelines/desktop-release.md` (new) and `docs/guidelines/README.md` index.
- ADR 0014 amendment.
- `docs/plans/desktop-app.md` §9 D6: tick "Owner confirms Windows Artifact Signing individual eligibility (US/Canada only); otherwise obtain an OV certificate in a cloud HSM".
- `docs/plans/architecture.md` §8 Q5 and `docs/plans/roadmap.md` decisions: **do not edit here** unless the outcome changes a recorded decision; if it does, note it for the plan owner in the PR.

## Risks and open questions

- **Lead times**: Apple individual enrolment and CA individual validation can take 1–3 weeks; start early (see Depends on).
- **Individual identity exposure**: the owner's legal name appears as publisher in Gatekeeper, Windows UAC/SmartScreen and package metadata — confirmed acceptable by the decision; re-confirm before purchase.
- **OV + cloud HSM CI automation** varies by CA (TOTP, client certs, rate limits); pick a CA whose tool works non-interactively on GitHub's Windows and Linux runners (signing Windows binaries from Linux with `jsign` is an option).
- **Artifact Signing eligibility rules may change**; the runbook cites the page and date checked.
- Future hosted-service company: if lamina later incorporates, identities move to the organisation — the runbook's rotation section covers re-signing and the updater key stays valid (it is independent of OS publisher identity).

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
