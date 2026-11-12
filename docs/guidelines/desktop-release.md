# Desktop release and signing runbook

This runbook tells the owner how to set up and look after the identities that sign Lamina PDF's desktop app, and tells the release workflows which secrets they may use. The owner signs every tagged release personally, as the publisher, on macOS, Windows and Linux ([architecture §8 Q5](../plans/architecture.md#8-open-questions-for-the-team), decided 2026-10-28). Sections 1 and 2 are the setup, section 3 checks each release, and sections 4 and 5 cover what happens later. Todo [D6a](../todos/ongoing/D6a-signing-runbook-and-owner-setup.md) wrote it; [D6b](../todos/ready/D6b-release-workflow-and-linux-packages.md), [D6c](../todos/ready/D6c-macos-signing-and-notarisation.md) and [D6d](../todos/ready/D6d-windows-signing.md) wire it into the workflows.

**Never write a secret value, private key, certificate file or recovery code into this repository**, a pull request, an issue or a screenshot. This page names secrets; it never holds them.

## What was decided on 2026-11-12

| Question | Answer |
| --- | --- |
| Product name and identifier | `Lamina PDF`, `com.laminapdf.desktop` (`apps/desktop/src-tauri/tauri.conf.json`) |
| Publisher shown on every OS | The owner's legal name, as on their government ID |
| Windows signing path | **Azure Artifact Signing**, Public Trust, individual developer. The owner lives in the US or Canada, the only places Microsoft accepts individuals. |
| Apple account | Apple Developer Program, **individual** membership, Team ID `35KBR7UHPZ` |
| Updater keys, R2 buckets, ADR 0014 | Wait on [D5a](../todos/ready/D5a-update-worker-and-adr-0014.md), which hasn't started |

D5a writes ADR 0014. Once it exists, these answers, the Apple Team ID and the GPG fingerprint go into it, before D6a merges.

The desktop app uses its own GitHub environments, `desktop-release`, `desktop-nightly` and `desktop-staging`. The repository's `release` environment belongs to the daily library and CLI releases, which publish with no approval step; adding a reviewer there would stop them, and their jobs must not read the desktop signing secrets.

## 1. Owner setup checklist

Start with the items that take longest: Apple enrolment and Azure identity validation both wait on someone else's checks. "Secret manager" below means the owner's password manager; the entry names are suggestions.

| Item | Cost | Lead time | Result kept in | Renew |
| --- | --- | --- | --- | --- |
| 1.1 Bundle identity | none | minutes | this page | never |
| 1.2 Apple Developer Program (individual) | US$99 a year | 1–3 days, sometimes weeks | Apple account; Team ID on this page | yearly membership |
| 1.3 Developer ID Application certificate | included | an hour | secret manager `Lamina PDF / Developer ID .p12`; `desktop-release` secrets | the date on the certificate (the first, issued 2026-11-12, expires 2027-12-12) |
| 1.4 Notarisation API key | included | minutes | secret manager `Lamina PDF / App Store Connect API key`; `desktop-release` secrets | when rotated |
| 1.5 Artifact Signing | Basic tier from US$9.99 a month, billed in full from the day the account is created; needs a paid subscription | 1–20 business days of identity validation | Azure subscription; non-secret names on this page; `desktop-release` secrets | identity validation before its expiry date (shown in the portal) |
| 1.6 Linux GPG key | none | an hour | offline primary key on two encrypted drives; subkey in secret manager and `desktop-release` secrets; public key on the website | subkey every 2 years |
| 1.7 Updater keys (D5a) | none | an hour | secret manager plus two offline backups; `desktop-release` and `desktop-nightly` secrets | only on compromise |
| 1.8 GitHub `desktop-release` environment | none | minutes | repository settings | review yearly |
| 1.9 GitHub `desktop-nightly` environment | none | minutes | repository settings | review yearly |
| 1.10 GitHub `desktop-staging` environment | none | minutes | repository settings | review yearly |
| 1.11 Cloudflare buckets and tokens (D5a) | R2 usage, a few US$ a month | minutes | Cloudflare account; `desktop-release`, `desktop-nightly` and `desktop-staging` secrets | yearly token rotation |

### 1.1 Bundle identity

- [ ] Confirm the product name `Lamina PDF` and identifier `com.laminapdf.desktop`. The identifier ends up in signed bundles and in every installed copy's settings path, so it must not change after the first release.
- [ ] Confirm the publisher is your legal name, spelled as on the ID you use for Apple and Azure. Both certificates must show the same name.

### 1.2 Apple Developer Program

- [x] Enrol as an individual at [developer.apple.com/programs/enroll](https://developer.apple.com/programs/enroll/) with the Apple Account you will keep for the project. Turn on two-factor authentication first.
- [x] Note the **Team ID** (Membership details) here: `35KBR7UHPZ` (recorded 2026-11-12).

### 1.3 Developer ID Application certificate

Gatekeeper accepts an app from outside the App Store only when a Developer ID Application certificate signed it and Apple notarised it ([Apple](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)).

- [x] In Xcode (Settings → Accounts → Manage Certificates → + → Developer ID Application), or from a certificate signing request made in Keychain Access, create the certificate. Only the Account Holder can create one.
- [x] In Keychain Access, export the certificate **with its private key** as a `.p12` protected by a long random password. Store the `.p12` and the password in the secret manager.
- [x] If Keychain Access says the certificate "is not trusted" and `security find-identity -v -p codesigning` finds 0 valid identities, install Apple's [Developer ID G2 intermediate](https://www.apple.com/certificateauthority/DeveloperIDG2CA.cer) (`security import DeveloperIDG2CA.cer -k ~/Library/Keychains/login.keychain-db`). CI's keychain needs it too.
- [x] Run `security find-identity -v -p codesigning` and copy the identity, `Developer ID Application: <Legal Name> (<TEAMID>)`.
- [x] Add the `desktop-release`-environment secrets `APPLE_CERTIFICATE` (`base64 -i cert.p12`), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` and `KEYCHAIN_PASSWORD` (a new random value for CI's temporary keychain).
- [x] Delete the exported `.p12` file from disk once it is in the secret manager.

**For D6c:** the build step gets `APPLE_CERTIFICATE` and `APPLE_CERTIFICATE_PASSWORD`, and Tauri signs with the identity in `APPLE_SIGNING_IDENTITY`. `KEYCHAIN_PASSWORD` is used only by the workflow's `security create-keychain` step ([Tauri macOS signing](https://v2.tauri.app/distribute/sign/macos/)).

### 1.4 Notarisation credentials

- [x] In App Store Connect → Users and Access → Integrations → App Store Connect API, request access if asked, then create a **Team key** with the Developer role. Download the `.p8` file: Apple lets you download it once.
- [x] Store the `.p8`, its Key ID and the Issuer ID in the secret manager.
- [x] Add the `desktop-release`-environment secrets `APPLE_API_KEY` (the Key ID), `APPLE_API_ISSUER` and `APPLE_API_KEY_P8` (the file's contents). The workflow writes the contents to a file and points `APPLE_API_KEY_PATH` at it, as Tauri expects.
- [ ] **Fallback** (not needed: the Team key worked), only if an individual account can't create a Team key: an app-specific password, with `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` as `desktop-release` secrets. Reason the API key wasn't possible: `<to fill in if used>`.

### 1.5 Azure Artifact Signing (Windows)

The steps follow Microsoft's [quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart). Identity validation for an individual takes its name and address from the Azure **billing account**, so fix those before you start.

- [ ] Create a pay-as-you-go Azure subscription (free, trial and sponsored ones can't use Artifact Signing) on a billing account whose Account Type is **Individual**. Set its legal name and sold-to address to match your government ID exactly.
- [ ] Register the `Microsoft.CodeSigning` resource provider on the subscription.
- [ ] Create an Artifact Signing account, Basic tier (billing starts now, for the whole month), in a US region (for example East US, endpoint `https://eus.codesigning.azure.net`). Give yourself the Artifact Signing Identity Verifier role.
- [ ] Under Identity validations, choose **Individual → New identity → Public**, then finish the ID check with AU10TIX and Microsoft Authenticator. The certificate shows your name, city, state or province, and country, never your street address.
- [ ] Create a **Public Trust** certificate profile using that validation. Leave "Include street address" and "Include postal code" unticked.
- [ ] In Microsoft Entra ID, register an app (`lamina-pdf-release-signing`). Add a **federated credential** for GitHub Actions: this repository, entity type *Environment*, environment `desktop-release`. Store no client secret ([GitHub OIDC with Azure](https://docs.github.com/actions/security-for-github-actions/security-hardening-your-deployments/configuring-openid-connect-in-azure)).
- [ ] Give the app the **Artifact Signing Certificate Profile Signer** role on the certificate profile only.
- [ ] Add the `desktop-release`-environment secrets `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` and `AZURE_SUBSCRIPTION_ID`. Record the non-secret names here:

| Name | Value |
| --- | --- |
| Endpoint | `<to fill in>` |
| Account | `<to fill in>` |
| Certificate profile | `<to fill in>` |
| Identity validation completed | `<date>` |

**For D6d (recommendation, not yet tested):** sign with `signtool` and Microsoft's Artifact Signing dlib in `bundle.windows.signCommand`, logged in through `azure/login` with the federated credential, so no client secret exists. Tauri's guide uses `artifact-signing-cli` instead, but that tool needs a client secret, `AZURE_CLIENT_SECRET` ([Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/); [artifact-signing-cli](https://github.com/Levminer/artifact-signing-cli)). Only if `signtool` fails does the owner add a client secret, with a 12-month expiry.

**OV certificate: not applicable**, because the owner qualifies for Artifact Signing. If Microsoft's rules change or validation fails, the fallback is the OV certificate in a cloud HSM that the D6a todo describes, and this section and ADR 0014 then record the CA and signing tool.

### 1.6 Linux GPG key

One key signs the AppImage, `.deb`, `.rpm`, the detached `.asc` files and `SHA256SUMS`.

- [x] On a trusted machine, preferably offline, create an ed25519 **primary key** for certification only: `gpg --quick-gen-key "<Legal Name> (Lamina PDF releases) <releases@laminapdf.com>" ed25519 cert 5y`. The email address is a suggestion.
- [x] Add a **signing subkey** that expires in two years: `gpg --quick-add-key <PRIMARY FPR> ed25519 sign 2y`.
- [ ] Back up the primary key (`gpg --export-secret-keys --armor`) and a revocation certificate to two encrypted drives kept in different places, then remove the primary secret key from the machine CI's key comes from.
- [x] Export **only the subkey** (`gpg --export-secret-subkeys --armor '<SUBKEY FPR>!'`, single-quoted so zsh leaves the `!` alone), protected by a passphrase. Add the `desktop-release` secrets `LINUX_SIGNING_KEY` (the armored export) and `LINUX_SIGNING_KEY_PASSPHRASE`.
- [x] Publish the public key at `https://laminapdf.com/keys/lamina-release.asc` (the file `apps/site/public/keys/lamina-release.asc`, added by D6a; live once the website deploys it).
- [x] Upload it to [keys.openpgp.org](https://keys.openpgp.org/upload) and click the link it sends to `releases@laminapdf.com`, so the key is found by its email address.
- [x] Record the fingerprints here (and in ADR 0014 once D5a writes it):
  - primary (certify only, expires 2031-11-11): `FA00 7EB8 6046 1729 9F59  6FC5 B9FC 925D 5E53 7171`
  - signing subkey (expires 2028-11-11): `DD4D 8CE5 7C00 8BF6 05D4  73B6 7C7B 196F 2CB0 87F5`

**For D6b:** the workflow maps the two secrets to what each packaging tool reads.

| Package | Variables | Source |
| --- | --- | --- |
| AppImage | `SIGN=1`, `SIGN_KEY`, `APPIMAGETOOL_SIGN_PASSPHRASE` | [Tauri Linux signing](https://v2.tauri.app/distribute/sign/linux/) |
| `.rpm` | `TAURI_SIGNING_RPM_KEY`, `TAURI_SIGNING_RPM_KEY_PASSPHRASE` | [Tauri RPM](https://v2.tauri.app/distribute/rpm/) |
| `.deb` | none: Tauri doesn't sign it, so D6b signs it with `debsigs` | — |

### 1.7 Updater keys (waits on D5a)

D5a's key ceremony makes two Tauri updater keys: one for releases and one for nightlies. Losing the release key strands every installed copy, because installed apps accept only updates that key signed.

- [ ] Confirm `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` exist in the `desktop-release` environment.
- [ ] Confirm the release key's offline backups exist in two places.
- [ ] Confirm the nightly key exists in the `desktop-nightly` environment under the same two names, with different values and one backup.

### 1.8 GitHub `desktop-release` environment

The `desktop-release` environment holds every production secret, and every job that uses it waits for the owner's approval ([GitHub environments](https://docs.github.com/actions/deployment/targeting-different-environments/using-environments-for-deployment)).

- [x] Create it with required reviewer = the owner, and "Prevent self-review" off so the owner can approve their own runs.
- [x] Limit deployment branches and tags to the tag pattern `desktop-v*`, the branch `main`, and, for now, the branches `ci/d6c-*` and `ci/d6d-*`, so D6c and D6d can test their signing steps before they merge. D6d removes the two `ci/` patterns when it merges.
- [ ] Add the secrets listed for `desktop-release` in §2, and none at repository level.

### 1.9 GitHub `desktop-nightly` environment

Scheduled nightlies run in `desktop-nightly`, which has **no required reviewer** so the schedule never waits for approval. To keep that safe, it holds nothing that can sign as the owner.

- [x] Create it with no reviewer, and limit deployment branches to `main`.
- [ ] Add only the nightly updater key and the Cloudflare token for the nightly bucket (§2). It gets no Apple, Azure or GPG secret and no Azure federated credential. Nightlies are ad-hoc signed on macOS, unsigned on Windows, and come with only `SHA256SUMS` on Linux (decided in D6b).

### 1.10 GitHub `desktop-staging` environment

D5b's updater tests run in `desktop-staging` on every push to `main` and every night, so it has no required reviewer either. It holds only a throwaway test updater key and a token for the staging bucket, so nothing in it can reach a user's install.

- [x] Create it with no reviewer, and limit deployment branches to `main`.
- [ ] Add only the two `desktop-staging` secrets in §2 (from D5a and D5b).

### 1.11 Cloudflare (waits on D5a)

- [ ] Confirm the R2 buckets `lamina-pdf-releases`, `lamina-pdf-nightly` and `lamina-pdf-staging` and their download domains exist (D5a). The staging bucket and the `desktop-staging` environment in §2 serve D5b's updater tests and D6b–D6d's trial runs; they hold only a throwaway updater key and that bucket's token, so nothing in them can reach a user's install.
- [ ] Confirm each token is scoped to one bucket only, stored as in §2. The staging bucket has two separate tokens: one in `desktop-release`, for D6b–D6d's signed trial runs, and one in `desktop-staging`, for D5b's tests.

## 2. Secrets inventory

Every secret is an **environment** secret; the repository level holds none. "Readers" means the jobs that can read it; the owner can always replace a value but not read one back. The source copy of each lives in the owner's secret manager.

| Secret | Environment | Purpose | Secret manager entry | Readers | Rotate or renew |
| --- | --- | --- | --- | --- | --- |
| `APPLE_CERTIFICATE` | `desktop-release` | base64 Developer ID `.p12` | `Lamina PDF / Developer ID .p12` | macOS signing jobs (D6c), after approval | with the certificate (§4) |
| `APPLE_CERTIFICATE_PASSWORD` | `desktop-release` | the `.p12` password | `Lamina PDF / Developer ID .p12` | same | with the certificate |
| `APPLE_SIGNING_IDENTITY` | `desktop-release` | `Developer ID Application: <Name> (<TEAMID>)` | `Lamina PDF / Developer ID .p12` | same | with the certificate |
| `KEYCHAIN_PASSWORD` | `desktop-release` | CI's temporary keychain | `Lamina PDF / CI keychain` | same | any time; no calendar entry |
| `APPLE_API_KEY` | `desktop-release` | App Store Connect Key ID | `Lamina PDF / App Store Connect API key` | notarisation step (D6c) | when the key is revoked |
| `APPLE_API_ISSUER` | `desktop-release` | App Store Connect Issuer ID | `Lamina PDF / App Store Connect API key` | same | never |
| `APPLE_API_KEY_P8` | `desktop-release` | the `.p8` contents | `Lamina PDF / App Store Connect API key` | same | when the key is revoked |
| `AZURE_CLIENT_ID` | `desktop-release` | Entra app for Artifact Signing (federated, no secret) | `Lamina PDF / Azure Artifact Signing` | Windows signing jobs (D6d), after approval | never |
| `AZURE_TENANT_ID` | `desktop-release` | Entra tenant | `Lamina PDF / Azure Artifact Signing` | same | never |
| `AZURE_SUBSCRIPTION_ID` | `desktop-release` | Azure subscription | `Lamina PDF / Azure Artifact Signing` | same | never |
| `LINUX_SIGNING_KEY` | `desktop-release` | armored GPG signing subkey | `Lamina PDF / GPG signing subkey` | Linux packaging jobs (D6b), after approval | subkey every 2 years |
| `LINUX_SIGNING_KEY_PASSPHRASE` | `desktop-release` | its passphrase | `Lamina PDF / GPG signing subkey` | same | with the subkey |
| `TAURI_SIGNING_PRIVATE_KEY` | `desktop-release` | release updater key (D5a) | `Lamina PDF / updater key (release)` | bundle jobs (D6b), after approval | only on compromise |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | `desktop-release` | its password | `Lamina PDF / updater key (release)` | same | with the key |
| `CLOUDFLARE_API_TOKEN` | `desktop-release` | write to `lamina-pdf-releases` only | `Lamina PDF / Cloudflare R2 release` | publish job (D6b) | yearly |
| `CLOUDFLARE_ACCOUNT_ID` | `desktop-release` | Cloudflare account | `Lamina PDF / Cloudflare R2 release` | same | never |
| `CLOUDFLARE_STAGING_R2_TOKEN` | `desktop-release` | write to `lamina-pdf-staging`, for signed staging dispatch runs | `Lamina PDF / Cloudflare R2 staging` | D6b–D6d staging runs | yearly |
| `TAURI_SIGNING_PRIVATE_KEY` | `desktop-nightly` | nightly-only updater key (D5a) | `Lamina PDF / updater key (nightly)` | nightly bundle jobs | only on compromise |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | `desktop-nightly` | its password | `Lamina PDF / updater key (nightly)` | same | with the key |
| `CLOUDFLARE_API_TOKEN` | `desktop-nightly` | write to `lamina-pdf-nightly` only | `Lamina PDF / Cloudflare R2 nightly` | nightly publish job | yearly |
| `CLOUDFLARE_ACCOUNT_ID` | `desktop-nightly` | Cloudflare account | `Lamina PDF / Cloudflare R2 nightly` | same | never |
| `CLOUDFLARE_STAGING_R2_TOKEN` | `desktop-staging` | write to `lamina-pdf-staging` only (D5b) | `Lamina PDF / Cloudflare R2 staging` | updater end-to-end tests on `main` | yearly |
| `TAURI_TEST_SIGNING_PRIVATE_KEY` | `desktop-staging` | throwaway test updater key (D5b), never a production key | none (throwaway) | same | any time |

`AZURE_CLIENT_SECRET` is added to `desktop-release` only if D6d's federated login fails (§1.5). D6b's workflow fails a build if any job in the `desktop-nightly` environment refers to an `APPLE_*`, `AZURE_*` or `LINUX_SIGNING_*` secret.

## 3. Verification commands

Run these on every release candidate and paste the output (not the files) into the release pull request. For the first release of each signing path, run them on a clean virtual machine.

**macOS** ([Apple](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)):

```sh
codesign --verify --deep --strict --verbose=2 "Lamina PDF.app"
spctl -a -vvv -t exec "Lamina PDF.app"          # expect: source=Notarized Developer ID
spctl -a -vvv -t open --context context:primary-signature Lamina-PDF.dmg
xcrun stapler validate Lamina-PDF.dmg
```

**Windows** (PowerShell):

```powershell
signtool verify /pa /v .\Lamina-PDF-setup.exe
Get-AuthenticodeSignature .\Lamina-PDF-setup.exe | Format-List   # Status Valid; SignerCertificate subject shows the legal name
```

**Linux:**

```sh
gpg --import lamina-release.asc
gpg --verify SHA256SUMS.asc SHA256SUMS && sha256sum --check --ignore-missing SHA256SUMS
gpg --verify lamina-pdf_<version>_amd64.deb.asc lamina-pdf_<version>_amd64.deb
debsig-verify lamina-pdf_<version>_amd64.deb   # needs the policy file and keyring D6b publishes
rpm -K lamina-pdf-<version>-1.x86_64.rpm         # after: rpm --import lamina-release.asc (Tauri's guide uses rpm -v --checksig)
./Lamina-PDF_<version>_amd64.AppImage --appimage-signature   # prints the embedded signature
./validate-x86_64.AppImage Lamina-PDF_<version>_amd64.AppImage   # expect: Validation successful
```

`validate-x86_64.AppImage` comes from [AppImageUpdate's continuous release](https://github.com/AppImageCommunity/AppImageUpdate/releases/tag/continuous). AppImage doesn't check its own signature when it runs, so users check it by hand against the key published on the website ([Tauri Linux signing](https://v2.tauri.app/distribute/sign/linux/)).

**Every artefact:**

```sh
gh attestation verify <file> --repo laudebugs/laminapdf
```

GitHub makes build-provenance attestations only for public repositories, so this check needs the repository to stay public.

## 4. Renewals and incidents

### Renewal calendar

Fill in each date when the item is created, and add a calendar reminder as far ahead as the Reminder column says.

| What | Expires | Reminder | How to renew |
| --- | --- | --- | --- |
| Apple Developer Program membership | `<enrolment date + 1 year>` | 60 days before | renew in the Apple Developer account; lapsing stops notarisation but existing notarised builds keep working |
| Developer ID Application certificate | 2027-12-12 (issued 2026-11-12; read the date from each new certificate: this one ran 13 months, not the 5 years older guidance gives) | 60 days before | create a new one (§1.3), replace `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD` and `APPLE_SIGNING_IDENTITY`, sign the next release with it |
| Artifact Signing identity validation | `<expiry date shown in the Azure portal>` | 60 days before (Microsoft also emails reminders from then) | create a new identity validation and attach it to the certificate profile; if it lapses, certificate renewal and signing stop ([FAQ](https://learn.microsoft.com/azure/artifact-signing/faq)) |
| Entra client secret (only if used) | `<creation + 12 months>` | 30 days before | create a new secret, update `AZURE_CLIENT_SECRET`, delete the old one |
| GPG signing subkey | 2028-11-11 | 60 days before | `gpg --quick-set-expire <PRIMARY FPR> 2y <SUBKEY FPR>` on the offline machine, re-export and republish the public key, replace `LINUX_SIGNING_KEY` |
| GPG primary key | 2031-11-11 | 90 days before | extend with `gpg --quick-set-expire <PRIMARY FPR> 5y`, republish |
| Cloudflare API tokens | `<creation + 1 year>` | 30 days before | create a new scoped token, replace the secret, revoke the old token |
| Environment review | every November | — | check reviewers, branch rules and that no secret sits at repository level |

### When a key or account is compromised

1. **Stop new updates.** Pause the affected channels in the update Worker's manifests (ADR 0014's kill switch, once D5a writes it; effective within 60 seconds). A paused channel still serves bridge releases.
2. **Revoke the identity.**
   - Apple: revoke the Developer ID certificate in the developer account, revoke the API key, and contact Apple Developer Support about the compromise so it can act on builds already signed.
   - Azure: remove the Signer role and the federated credential, then open an Azure support request to revoke the certificate profile's certificates (deleting the profile doesn't revoke them; [FAQ](https://learn.microsoft.com/azure/artifact-signing/faq)).
   - GPG: publish the revocation certificate for the subkey (or the primary key) to the website and keys.openpgp.org.
   - Updater key: follow ADR 0014's rotation procedure (once D5a writes it): a bridge release signed with the old key that carries the new public key.
   - Cloudflare or GitHub: revoke the token and delete the environment secret.
3. **Make the new identity** with this runbook and replace the secrets.
4. **Re-sign and republish** the current release with the new identity, verify it with §3, and lift the pause.
5. **Tell users** in the release notes, on the download page, and in a GitHub security advisory: what was exposed, which builds to distrust and what to install.

### If the project becomes a company

The publisher identities move to the company: a new Apple organisation membership, a new Artifact Signing organisation validation and a new GPG key. Releases signed after the move show the company's name. The updater key stays the same, because Tauri checks it independently of the OS publisher, so installed copies keep updating.

## 5. SmartScreen policy

A SmartScreen warning that names the publisher is acceptable for early releases; a prompt with no publisher name is a failure. Expect the warning on the first downloads: Microsoft's guidance is that a newly signed file shows "unrecognized" until it has download history, whatever the certificate: OV, EV and Artifact Signing alike, with the verified publisher name shown in the prompt. Reputation builds over several weeks and hundreds of clean installs, and later builds signed with the same identity can inherit some of it, so every release keeps the same signing identity ([SmartScreen reputation](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation)). D6d records what a clean Windows virtual machine shows for the first signed build.

Until releases install without a warning, the release notes carry this text:

> **Windows may show "Windows protected your PC".** Lamina PDF is new, so Microsoft SmartScreen hasn't seen many downloads of it yet. The installer is signed by <Legal Name>. To install it, select **More info**, check that the publisher is <Legal Name>, then select **Run anyway**. The warning goes away as more people install the app.

## Sources

Checked 2026-11-12:

- Microsoft: [Artifact Signing quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart) (updated 2026-10-08), [Artifact Signing FAQ](https://learn.microsoft.com/azure/artifact-signing/faq) (updated 2026-10-27), [SmartScreen reputation](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation) (updated 2026-08-17), [pricing](https://azure.microsoft.com/pricing/details/artifact-signing/)
- Tauri: [macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Windows signing](https://v2.tauri.app/distribute/sign/windows/), [Linux signing](https://v2.tauri.app/distribute/sign/linux/), [RPM](https://v2.tauri.app/distribute/rpm/)
- Apple: [notarising macOS software](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- GitHub: [environments](https://docs.github.com/actions/deployment/targeting-different-environments/using-environments-for-deployment), [OIDC with Azure](https://docs.github.com/actions/security-for-github-actions/security-hardening-your-deployments/configuring-openid-connect-in-azure)
