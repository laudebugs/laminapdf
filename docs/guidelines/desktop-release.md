# Desktop release and signing runbook

This runbook tells the owner how to set up and look after the identities that sign Lamina PDF's desktop app, and tells the release workflows which secrets they may use. The owner signs every tagged release personally, as the publisher, on macOS, Windows and Linux ([architecture §8 Q5](../plans/architecture.md#8-open-questions-for-the-team), decided 2026-10-28). Sections 1 and 2 are the setup, section 3 checks each release, sections 4 and 5 cover what happens later, section 6 cuts a release, and section 7 covers the crash symbols each release archives and how to read a report with them, and section 8 the Mac App Store build. Todo [D6a](../todos/done/D6a-signing-runbook-and-owner-setup.md) wrote it; [D6b](../todos/done/D6b-release-workflow-and-linux-packages.md), [D6c](../todos/done/D6c-macos-signing-and-notarisation.md) and [D6d](../todos/done/D6d-windows-signing.md) wire it into the workflows.

**Never write a secret value, private key, certificate file or recovery code into this repository**, a pull request, an issue or a screenshot. This page names secrets; it never holds them.

## What was decided on 2026-11-12

| Question | Answer |
| --- | --- |
| Product name and identifier | `Lamina PDF`, `com.laminapdf.desktop` (`apps/desktop/src-tauri/tauri.conf.json`) |
| Publisher shown on every OS | The owner's legal name, as on their government ID |
| Windows signing path | **Azure Artifact Signing**, Public Trust, individual developer. The owner lives in the US or Canada, the only places Microsoft accepts individuals. |
| Apple account | Apple Developer Program, **individual** membership, Team ID `35KBR7UHPZ` |
| Updater keys, R2 buckets, ADR 0014 | Buckets and tokens done 2026-11-12; keys (release `34929FE857511973`, nightly `46010ED395B59CAF`) replaced 2026-11-15, before any release or nightly published, because the first release key's password didn't unlock it; [ADR 0014](../decisions/0014-desktop-distribution.md) written by [D5a](../todos/done/D5a-update-worker-and-adr-0014.md) |

[ADR 0014 §6](../decisions/0014-desktop-distribution.md#6-signing-identities) records these answers, the Apple Team ID, the Azure account and profile, and the GPG fingerprints.

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
| 1.12 Mac App Store setup (D8c, D8d) | included | an hour | secret manager entries named in §2; `desktop-release` secrets | the certificates' dates and the profile's expiry (§4) |

### 1.1 Bundle identity

- [x] Confirm the product name `Lamina PDF` and identifier `com.laminapdf.desktop` (confirmed by the owner 2026-11-13). The identifier ends up in signed bundles and in every installed copy's settings path, so it must not change after the first release.
- [x] Confirm the publisher is your legal name, spelled as on the ID you use for Apple and Azure. Both certificates must show the same name.

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
- **Not applicable:** the Team key worked. **Fallback**, only if an individual account can't create a Team key: an app-specific password, with `APPLE_ID`, `APPLE_PASSWORD` and `APPLE_TEAM_ID` as `desktop-release` secrets. Reason the API key wasn't possible: `<to fill in if used>`.

### 1.5 Azure Artifact Signing (Windows)

The steps follow Microsoft's [quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart). Identity validation for an individual takes its name and address from the Azure **billing account**, so fix those before you start.

- [x] Create a pay-as-you-go Azure subscription (free, trial and sponsored ones can't use Artifact Signing) on a billing account whose Account Type is **Individual**. Set its legal name and sold-to address to match your government ID exactly.
- [x] Register the `Microsoft.CodeSigning` resource provider on the subscription.
- [x] Create an Artifact Signing account, Basic tier (billing starts now, for the whole month), in a US region (for example East US, endpoint `https://eus.codesigning.azure.net`). Give yourself the Artifact Signing Identity Verifier role.
- [x] Under Identity validations, choose **Individual → New identity → Public**, then finish the ID check with AU10TIX and Microsoft Authenticator. The certificate shows your name, city, state or province, and country, never your street address.
- [x] Create a **Public Trust** certificate profile using that validation. Leave "Include street address" and "Include postal code" unticked.
- [x] In Microsoft Entra ID, register an app (`lamina-pdf-release-signing`). Add a **federated credential** for GitHub Actions: this repository, entity type *Environment*, environment `desktop-release`. Store no client secret ([GitHub OIDC with Azure](https://docs.github.com/actions/security-for-github-actions/security-hardening-your-deployments/configuring-openid-connect-in-azure)).
- [x] Give the app the **Artifact Signing Certificate Profile Signer** role on the certificate profile only.
- [x] Add the `desktop-release`-environment secrets `AZURE_CLIENT_ID`, `AZURE_TENANT_ID` and `AZURE_SUBSCRIPTION_ID`. Record the non-secret names here:

| Name | Value |
| --- | --- |
| Endpoint | `https://eus.codesigning.azure.net` (East US) |
| Account | `LaurenceIninda`, resource group `Laurence_Ininda` |
| Certificate profile | `LaminaPdfPublicTrust` (Public Trust); subject `CN=Laurence Ininda, O=Laurence Ininda, L=Brooklyn, S=NY, C=US`; Microsoft reissues its certificate every few days |
| Identity validation completed | 2026-11-12 (ID `62b8833a-2cbf-46c0-8f7c-702606975b2a`) |
| Entra app (no client secret) | `lamina-pdf-release-signing`, federated credential `github-desktop-release` with the subject `repo:laudebugs@25782070/laminapdf@1437211634:environment:desktop-release`: this repository uses GitHub's immutable subject format, which names the owner and repository by ID (`gh api repos/laudebugs/laminapdf/actions/oidc/customization/sub`) |

**How the release build signs (D6d).** The Windows legs of `build-release` sign with SignTool and Microsoft's Artifact Signing dlib, logged in through `azure/login` with the federated credential, so no client secret exists. The dlib comes from the NuGet package `Microsoft.ArtifactSigning.Client`, pinned by version and SHA-256 in `.github/actions/desktop-build` and fetched only there; `apps/desktop/src-tauri/windows/artifact-signing.json` holds the endpoint, account and profile above. `apps/desktop/scripts/sign-windows.ps1` signs one file at a time; the Tauri CLI calls it for the app, the engine, both installers and the uninstaller, and the build calls it for PDFium's DLL. Tauri's guide uses `artifact-signing-cli` instead, but that tool needs a client secret, `AZURE_CLIENT_SECRET` ([Tauri Windows signing](https://v2.tauri.app/distribute/sign/windows/); [artifact-signing-cli](https://github.com/Levminer/artifact-signing-cli)). Add one, with a 12-month expiry, only if the federated login stops working.

**OV certificate: not applicable**, because the owner qualifies for Artifact Signing. If Microsoft's rules change or validation fails, the fallback is the OV certificate in a cloud HSM that the D6a todo describes, and this section and ADR 0014 then record the CA and signing tool.

### 1.6 Linux GPG key

One key signs the AppImage, `.deb`, `.rpm`, the detached `.asc` files and `SHA256SUMS`.

- [x] On a trusted machine, preferably offline, create an ed25519 **primary key** for certification only: `gpg --quick-gen-key "<Legal Name> (Lamina PDF releases) <releases@laminapdf.com>" ed25519 cert 5y`. The email address is a suggestion.
- [x] Add a **signing subkey** that expires in two years: `gpg --quick-add-key <PRIMARY FPR> ed25519 sign 2y`.
- [x] Back up the primary key (`gpg --export-secret-keys --armor`) and a revocation certificate to two encrypted drives kept in different places, then remove the primary secret key from the machine CI's key comes from.
- [x] Export **only the subkey** (`gpg --export-secret-subkeys --armor '<SUBKEY FPR>!'`, single-quoted so zsh leaves the `!` alone), protected by a passphrase. Add the `desktop-release` secrets `LINUX_SIGNING_KEY` (the armored export) and `LINUX_SIGNING_KEY_PASSPHRASE`.
- [x] Publish the public key at `https://laminapdf.com/keys/lamina-release.asc` (the file `apps/site/public/keys/lamina-release.asc`, added by D6a; live once the website deploys it).
- [x] Upload it to [keys.openpgp.org](https://keys.openpgp.org/upload) and click the link it sends to `releases@laminapdf.com`, so the key is found by its email address.
- [x] Record the fingerprints here (and in ADR 0014 §6):
  - primary (certify only, expires 2031-11-11): `FA00 7EB8 6046 1729 9F59  6FC5 B9FC 925D 5E53 7171`
  - signing subkey (expires 2028-11-11): `DD4D 8CE5 7C00 8BF6 05D4  73B6 7C7B 196F 2CB0 87F5`

**For D6b:** the workflow maps the two secrets to what each packaging tool reads.

| Package | Variables | Source |
| --- | --- | --- |
| AppImage | `SIGN=1`, `SIGN_KEY`, `APPIMAGETOOL_SIGN_PASSPHRASE` | [Tauri Linux signing](https://v2.tauri.app/distribute/sign/linux/) |
| `.rpm` | none: D6b signs it with `rpmsign` through gpg, since Tauri's own rpm signer (`TAURI_SIGNING_RPM_KEY`) may not read a subkey-only export | [Tauri RPM](https://v2.tauri.app/distribute/rpm/) |
| `.deb` | none: Tauri doesn't sign it, so D6b signs it with `debsigs` | — |

### 1.7 Updater keys

D5a's key ceremony makes two Tauri updater keys: one for releases and one for nightlies. Losing the release key strands every installed copy, because installed apps accept only updates that key signed.

- [x] Confirm `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` exist in the `desktop-release` environment.
- [x] Confirm the release key's offline backups exist in two places (owner, 2026-11-15: both updater keys, as replaced that day).
- [x] Confirm the nightly key exists in the `desktop-nightly` environment under the same two names, with different values and one backup.

### 1.8 GitHub `desktop-release` environment

The `desktop-release` environment holds every production secret, and every job that uses it waits for the owner's approval ([GitHub environments](https://docs.github.com/actions/deployment/targeting-different-environments/using-environments-for-deployment)).

- [x] Create it with required reviewer = the owner, and "Prevent self-review" off so the owner can approve their own runs.
- [x] Limit deployment branches and tags to the tag pattern `desktop-v*`, the branch `main`, and, for now, the branches `ci/d6c-*` and `ci/d6d-*`, so D6c and D6d can test their signing steps before they merge. Remove each `ci/` pattern once its todo has merged.
- [x] Add the secrets listed for `desktop-release` in §2, and none at repository level.

### 1.9 GitHub `desktop-nightly` environment

Scheduled nightlies run in `desktop-nightly`, which has **no required reviewer** so the schedule never waits for approval. To keep that safe, it holds nothing that can sign as the owner.

- [x] Create it with no reviewer, and limit deployment branches to `main`.
- [x] Add only the nightly updater key and the Cloudflare token for the nightly bucket (§2). It gets no Apple, Azure or GPG secret and no Azure federated credential. Nightlies are ad-hoc signed on macOS, unsigned on Windows, and come with only `SHA256SUMS` on Linux (decided in D6b).

### 1.10 GitHub `desktop-staging` environment

D5b's updater tests run in `desktop-staging` on every push to `main` and every night, so it has no required reviewer either. It holds only a throwaway test updater key and a token for the staging bucket, so nothing in it can reach a user's install.

- [x] Create it with no reviewer, and limit deployment branches to `main`.
- [x] Add only the two `desktop-staging` secrets in §2 (from D5a and D5b).

### 1.11 Cloudflare

- [x] Confirm the R2 buckets `lamina-pdf-releases`, `lamina-pdf-nightly` and `lamina-pdf-staging` and their download domains exist (D5a). The staging bucket and the `desktop-staging` environment in §2 serve D5b's updater tests and D6b–D6d's trial runs; they hold only a throwaway updater key and that bucket's token, so nothing in them can reach a user's install.
- [ ] Create the private symbols bucket `lamina-pdf-symbols` and its R2 token (§7.3).
- [x] Confirm each token is scoped to one bucket only, stored as in §2. The staging bucket has two separate tokens: one in `desktop-release`, for D6b–D6d's signed trial runs, and one in `desktop-staging`, for D5b's tests.

### 1.12 Mac App Store setup

This is the checklist for the store side. Do the steps in order; each needs the one before it. Everything here is Apple Developer account work, and only the Account Holder or an Admin can do most of it. The team is individual, Team ID `35KBR7UHPZ` (§1.2). After step 6, the `desktop-release.yml` workflow can build and upload the store package (`app_store=upload`, §8).

| Step | Where | Result kept | Goes to |
| --- | --- | --- | --- |
| 1 App ID | developer.apple.com › Identifiers | the App ID | nothing secret |
| 2 Two certificates | Keychain Access, then Certificates | two `.p12` files and their passwords | `desktop-release` secrets |
| 3 Provisioning profile | developer.apple.com › Profiles | one `.provisionprofile` | `desktop-release` secret |
| 4 API key | App Store Connect › Users and Access | `.p8`, Key ID, Issuer ID | `desktop-release` secrets |
| 5 App record | App Store Connect › Apps | the record | the listing |
| 6 Secrets | GitHub › `desktop-release` environment | nine secret values | the workflow |

#### Step 1: The App ID

An explicit App ID ties the bundle identifier to the team, so the store build's application identifier is `35KBR7UHPZ.com.laminapdf.desktop` ([Register an App ID](https://developer.apple.com/help/account/identifiers/register-an-app-id)).

- [ ] Sign in at developer.apple.com and open Certificates, Identifiers & Profiles › **Identifiers**.
- [ ] Select the add button (+), choose **App IDs**, then **Continue**, and confirm the type is **App**.
- [ ] Enter the description `Lamina PDF`.
- [ ] Choose **Explicit** and enter the Bundle ID `com.laminapdf.desktop`. The identifier is permanent in every installed copy (§1.1), so check each character.
- [ ] If the form asks for a platform, choose **macOS**. Apple's page doesn't show a platform selector (a single App ID can build for iOS, macOS, tvOS and watchOS since Xcode 11.4), so check Apple's current page.
- [ ] Leave the capabilities as they are: the build's entitlements (App Sandbox, files, printing, outgoing connections) need none that the portal gates. Select **Continue**, check the summary, then **Register**.
- [ ] Check that the list shows the identifier as `com.laminapdf.desktop` with the prefix `35KBR7UHPZ`.

If an App ID with this identifier exists already (Xcode creates `XC` ones for its own projects), use it, and say so in the todo.

#### Step 2: Two distribution certificates

The Apple Distribution certificate signs the app, and the Mac Installer Distribution certificate signs the `.pkg` that `productbuild` makes. Both belong to the team, and only one of each type is allowed per team ([Certificates overview](https://developer.apple.com/help/account/certificates/certificates-overview)). Only an Account Holder or Admin can create them. Make each from its own request, so each has its own private key.

For each certificate, **Apple Distribution** first, then **Mac Installer Distribution**:

- [ ] Open **Keychain Access**, then Keychain Access › Certificate Assistant › **Request a Certificate from a Certificate Authority…**. Enter your email address, a Common Name that says which one it is (`Lamina PDF Apple Distribution`, then `Lamina PDF Installer Distribution`), leave **CA Email Address** empty, choose **Saved to disk** and **Continue** ([Create a certificate signing request](https://developer.apple.com/help/account/certificates/create-a-certificate-signing-request)). The private key stays in your keychain.
- [ ] On developer.apple.com open **Certificates**, press **+**, choose the type under Software, **Continue**, upload the request, **Continue**, then **Download**. Check Apple's current page for the screen's labels.
- [ ] Double-click the downloaded `.cer` to put it in the login keychain beside its key. In Keychain Access › **My Certificates**, check that an arrow expands the certificate to show a private key. A certificate with no key can't sign.
- [ ] Select the certificate (not the key), choose File › **Export Items…**, pick the **.p12** format and set a long random password. Save the `.p12` and its password in your password manager.
- [ ] Encode it for GitHub: `base64 -i file.p12 | pbcopy` puts the file's base64 text on the Mac's clipboard, ready to paste into GitHub's secret box.

If `security find-identity -v -p codesigning` finds no valid identity after the import, or Keychain Access says the certificate "is not trusted", install Apple's current intermediate certificate from [apple.com/certificateauthority](https://www.apple.com/certificateauthority/) (the Developer ID setup needed the same, §1.3). The store job pins Apple's two intermediate certificates itself (§8.1).

The certificate's validity period isn't on Apple's overview page: read its expiry date from the certificate and add it to the renewal calendar in the runbook ([§4](desktop-release.md#4-renewals-and-incidents)). Apple says that after expiry or revocation you can't upload new builds, and an uploaded but unsubmitted build signed with a revoked certificate may turn invalid.

#### Step 3: The Mac App Store provisioning profile

The profile joins the App ID to one Apple Distribution certificate, and it sits in the bundle as `embedded.provisionprofile` ([Create an App Store provisioning profile](https://developer.apple.com/help/account/provisioning-profiles/create-an-app-store-provisioning-profile)). Only an Account Holder or Admin can create it.

- [ ] Open **Profiles** and press **+**.
- [ ] Under **Distribution**, choose **Mac App Store Connect**, then **Continue**.
- [ ] Choose the App ID `com.laminapdf.desktop` from step 1, then **Continue**.
- [ ] Choose the **Apple Distribution** certificate from step 2. A store profile holds exactly one distribution certificate.
- [ ] Name the profile `Lamina PDF Mac App Store`, then **Generate** and **Download**. The file ends in `.provisionprofile` (Apple's page doesn't say so; the extension is Xcode's convention, check the file you get).
- [ ] Encode the profile as in step 2, with the profile's own file name.

A profile names its certificate. When you renew or replace the Apple Distribution certificate, regenerate the profile against the new one and replace `APPLE_APP_STORE_PROFILE` too. Apple's page doesn't say how long a profile lasts: it is about a year, so check the expiry date shown in the developer portal (or open the file with `security cms -D -i file.provisionprofile` in Terminal) and calendar it beside the certificates.

#### Step 4: The App Store Connect API key

The release workflow uploads each build with an API key. The key that notarises (§1.4) has the Developer role, which Apple's role table lets upload builds, so it may serve for both ([Creating API keys](https://developer.apple.com/documentation/appstoreconnectapi/creating-api-keys-for-app-store-connect-api); the role table in that answer came from general knowledge, so check Apple's current page).

- [ ] Open App Store Connect › **Users and Access** › **Integrations** › **App Store Connect API** › **Team Keys**. The Account Holder asks for API access first if it was never requested ([App Store Connect API](https://developer.apple.com/help/app-store-connect/get-started/app-store-connect-api)).
- [ ] Open the notarisation key's row. If its Access column says **Developer** or **App Manager**, reuse it. Otherwise generate a new key with one of those roles, since Apple doesn't let you change a key's role afterwards.
- [ ] For a new key: **Generate API Key** (or **+**), name it `Lamina PDF GitHub Actions`, choose the role, **Generate**, then **Download API Key** at once. Apple allows the `.p8` download only once.
- [ ] Copy the **Key ID** from the table and the **Issuer ID** from the top of the page. Keep all three in the password manager.
- [ ] If you made a new key, replace the three secrets `APPLE_API_KEY`, `APPLE_API_ISSUER` and `APPLE_API_KEY_P8`, and revoke the old key only after a notarisation run has used the new one.

#### Step 5: The app record

- [ ] The Account Holder accepts any waiting agreement in App Store Connect › **Business**. A free app needs the free-app agreement only (check Apple's current page).
- [ ] In **Apps** press **+** › **New App** ([Add a new app](https://developer.apple.com/help/app-store-connect/create-an-app-record/add-a-new-app)).
- [ ] Choose the platform **macOS**.
- [ ] Name: `Lamina PDF` (the name is unique per localisation: if it is taken, see the claim steps on Apple's page, and tell the agent so the listing can change).
- [ ] Primary language: English (U.S.).
- [ ] Bundle ID: choose `Lamina PDF - com.laminapdf.desktop` from the list (it appears only after step 1).
- [ ] SKU: any unique string you like, such as `laminapdf-desktop-mac`. Apple's page doesn't say whether it can change later, so treat it as permanent.
- [ ] User Access: **Full Access**.
- [ ] **Create**, then paste the listing from §2 of [the Mac App Store guideline](mac-app-store.md) into the version page and the App Information page.

#### Step 6: The secrets

Add each to the **`desktop-release`** environment (repository Settings › Environments › `desktop-release` › Environment secrets, §1.8). Encode each file as in step 2. **No secret value is ever pasted into a chat, an issue, a pull request or a commit**, and the agent never needs to see one. Delete the exported `.p12` and `.p8` files from disk once they are in the password manager.

Five secrets are new: `APPLE_DISTRIBUTION_CERTIFICATE` and its password, `APPLE_INSTALLER_CERTIFICATE` and its password, and `APPLE_APP_STORE_PROFILE`. §8.1 says what the job needs from each, and §2 lists all nine with their readers and renewals. The other four, `KEYCHAIN_PASSWORD` and the three `APPLE_API_*` secrets, are the direct download's: reuse them, or replace the three `APPLE_API_*` ones with a new key from step 4. Add the two certificates' expiry dates and the profile's to §4's renewal calendar.

The five store secrets reach only the steps that import the certificates and upload the package (§8.2). The build number is `<run>.<attempt>`; §8.2 says why an upload by hand with a larger first number would make every later build look older, so don't upload by hand.

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
| `APPLE_DISTRIBUTION_CERTIFICATE` | `desktop-release` | base64 Apple Distribution `.p12` (the Mac App Store build, §8) | `Lamina PDF / Apple Distribution .p12` | the store upload job (D8c), after approval | with the certificate (§4) |
| `APPLE_DISTRIBUTION_CERTIFICATE_PASSWORD` | `desktop-release` | the `.p12` password | `Lamina PDF / Apple Distribution .p12` | same | with the certificate |
| `APPLE_INSTALLER_CERTIFICATE` | `desktop-release` | base64 Mac Installer Distribution `.p12` | `Lamina PDF / Mac Installer Distribution .p12` | same | with the certificate |
| `APPLE_INSTALLER_CERTIFICATE_PASSWORD` | `desktop-release` | the `.p12` password | `Lamina PDF / Mac Installer Distribution .p12` | same | with the certificate |
| `APPLE_APP_STORE_PROFILE` | `desktop-release` | base64 Mac App Store provisioning profile for `com.laminapdf.desktop` | `Lamina PDF / Mac App Store profile` | same | with the certificate, and when Apple expires it (a year) |
| `AZURE_CLIENT_ID` | `desktop-release` | Entra app for Artifact Signing (federated, no secret) | `Lamina PDF / Azure Artifact Signing` | Windows signing jobs (D6d), after approval | never |
| `AZURE_TENANT_ID` | `desktop-release` | Entra tenant | `Lamina PDF / Azure Artifact Signing` | same | never |
| `AZURE_SUBSCRIPTION_ID` | `desktop-release` | Azure subscription | `Lamina PDF / Azure Artifact Signing` | same | never |
| `LINUX_SIGNING_KEY` | `desktop-release` | armored GPG signing subkey | `Lamina PDF / GPG signing subkey` | Linux packaging jobs (D6b), after approval | subkey every 2 years |
| `LINUX_SIGNING_KEY_PASSPHRASE` | `desktop-release` | its passphrase | `Lamina PDF / GPG signing subkey` | same | with the subkey |
| `TAURI_SIGNING_PRIVATE_KEY` | `desktop-release` | release updater key (D5a) | `Lamina PDF / updater key (release)` | bundle jobs (D6b), after approval | only on compromise |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | `desktop-release` | its password | `Lamina PDF / updater key (release)` | same | with the key |
| `CLOUDFLARE_API_TOKEN` | `desktop-release` | write to `lamina-pdf-releases` only | `Lamina PDF / Cloudflare R2 release` | publish job (D6b) | yearly |
| `R2_ACCESS_KEY_ID` | `desktop-release` | that token's S3 Access Key ID (R2's S3 API; the secret is the token's SHA-256) | `Lamina PDF / Cloudflare R2 release` | same | with the token |
| `CLOUDFLARE_ACCOUNT_ID` | `desktop-release` | Cloudflare account | `Lamina PDF / Cloudflare R2 release` | same | never |
| `SYMBOLS_R2_TOKEN` | `desktop-release` | write to the private `lamina-pdf-symbols` only (§7.3) | `Lamina PDF / Cloudflare R2 symbols` | `archive-symbols` job (D7d), tag pushes only | yearly |
| `SYMBOLS_R2_ACCESS_KEY_ID` | `desktop-release` | that token's S3 Access Key ID | `Lamina PDF / Cloudflare R2 symbols` | same | with the token |
| `CLOUDFLARE_STAGING_R2_TOKEN` | `desktop-release` | write to `lamina-pdf-staging`, for signed staging dispatch runs | `Lamina PDF / Cloudflare R2 staging` | D6b–D6d staging runs | yearly |
| `STAGING_R2_ACCESS_KEY_ID` | `desktop-release` | that token's S3 Access Key ID | `Lamina PDF / Cloudflare R2 staging` | same | with the token |
| `TAURI_SIGNING_PRIVATE_KEY` | `desktop-nightly` | nightly-only updater key (D5a) | `Lamina PDF / updater key (nightly)` | nightly bundle jobs | only on compromise |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | `desktop-nightly` | its password | `Lamina PDF / updater key (nightly)` | same | with the key |
| `CLOUDFLARE_API_TOKEN` | `desktop-nightly` | write to `lamina-pdf-nightly` only | `Lamina PDF / Cloudflare R2 nightly` | nightly publish job | yearly |
| `R2_ACCESS_KEY_ID` | `desktop-nightly` | that token's S3 Access Key ID | `Lamina PDF / Cloudflare R2 nightly` | same | with the token |
| `CLOUDFLARE_ACCOUNT_ID` | `desktop-nightly` | Cloudflare account | `Lamina PDF / Cloudflare R2 nightly` | same | never |
| `CLOUDFLARE_STAGING_R2_TOKEN` | `desktop-staging` | write to `lamina-pdf-staging` only (D5b) | `Lamina PDF / Cloudflare R2 staging` | updater end-to-end tests on `main` | yearly |
| `STAGING_R2_ACCESS_KEY_ID` | `desktop-staging` | that token's S3 Access Key ID | `Lamina PDF / Cloudflare R2 staging` | same | with the token |
| `TAURI_TEST_SIGNING_PRIVATE_KEY` | `desktop-staging` | throwaway test updater key (D5b), never a production key | none (throwaway) | same | any time |

`AZURE_CLIENT_SECRET` is added to `desktop-release` only if D6d's federated login fails (§1.5). D6b's workflow fails a build if any job in the `desktop-nightly` environment refers to an `APPLE_*`, `AZURE_*` or `LINUX_SIGNING_*` secret.

## 3. Verification commands

Run these on every release candidate and paste the output (not the files) into the release pull request. For the first release of each signing path, run them on a clean virtual machine.

**macOS** ([Apple](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)):

```sh
codesign --verify --deep --strict --verbose=2 "Lamina PDF.app"
spctl -a -vvv -t exec "Lamina PDF.app"          # expect: source=Notarized Developer ID
xcrun stapler validate "Lamina PDF.app"
spctl -a -vvv -t open --context context:primary-signature Lamina-PDF.dmg   # expect: source=Notarized Developer ID
xcrun stapler validate Lamina-PDF.dmg
codesign -dvv "Lamina PDF.app/Contents/MacOS/laminapdf-desktop-engine"        # expect: flags=…runtime, TeamIdentifier=35KBR7UHPZ
codesign -dvv "Lamina PDF.app/Contents/Resources/pdfium/libpdfium.dylib"    # expect: TeamIdentifier=35KBR7UHPZ
```

The release workflow runs these checks on every macOS build it signs, and fails the build when one fails. It also checks that every other piece of code in the app carries the Team ID, that neither the app nor the engine has an entitlement, and that the updater's `.app.tar.gz` holds the stapled app. A runner whose image turns Gatekeeper's assessments off can't give `spctl`'s verdict: the build then warns and relies on the staple checks, so run `spctl` on a clean machine before the first release. On every macOS build, signed or not, the workflow has the bundled engine draw a page with the bundled PDFium.

**Windows** (PowerShell):

```powershell
signtool verify /pa /v .\Lamina-PDF-setup.exe
Get-AuthenticodeSignature .\Lamina-PDF-setup.exe | Format-List   # Status Valid; SignerCertificate subject shows the legal name
# Every file inside both installers, as the release build checks them (signer, SHA-256, RFC 3161 timestamp):
$env:SIGNTOOL = (Get-Command signtool).Source
.\apps\desktop\scripts\verify-windows-signatures.ps1 -Dir <folder with the installers> -Publisher 'Laurence Ininda'
```

The release build runs the last script on what it built, and installs and removes each installer on the runner (`apps/desktop/scripts/check-windows-installers.ps1`): Lamina PDF in `.pdf`'s "Open with" list and never its default, the MSI's install flavour, nothing left after the uninstall. NSIS's plugin DLLs, which the NSIS installer unpacks while it runs, are checked too: the Tauri CLI signs its copies of them.

**Linux:**

```sh
gpg --import lamina-release.asc
gpg --verify SHA256SUMS.asc SHA256SUMS && sha256sum --check --ignore-missing SHA256SUMS
gpg --verify lamina-pdf_<version>_amd64.deb.asc lamina-pdf_<version>_amd64.deb
debsig-verify lamina-pdf_<version>_amd64.deb   # needs the policy and keyring below
rpm -K lamina-pdf-<version>-1.x86_64.rpm         # after: rpm --import lamina-release.asc (Tauri's guide uses rpm -v --checksig)
./Lamina-PDF_<version>_amd64.AppImage --appimage-signature   # prints the embedded signature
./validate-x86_64.AppImage Lamina-PDF_<version>_amd64.AppImage   # expect: Validation successful
```

`debsig-verify` checks a `.deb` only against a policy and a keyring filed under the signing subkey's ID, `7C7B196F2CB087F5`. Install both once on each machine that checks:

```sh
id=7C7B196F2CB087F5
sudo mkdir -p /usr/share/debsig/keyrings/$id /etc/debsig/policies/$id
gpg --dearmor < lamina-release.asc | sudo tee /usr/share/debsig/keyrings/$id/debsig.gpg > /dev/null
sudo tee /etc/debsig/policies/$id/lamina-pdf.pol > /dev/null <<'POLICY'
<?xml version="1.0"?>
<!DOCTYPE Policy SYSTEM "https://www.debian.org/debsig/1.0/policy.dtd">
<Policy xmlns="https://www.debian.org/debsig/1.0/">
  <Origin Name="Lamina PDF" id="7C7B196F2CB087F5" Description="Lamina PDF releases"/>
  <Selection>
    <Required Type="origin" File="debsig.gpg" id="7C7B196F2CB087F5"/>
  </Selection>
  <Verification MinOptional="0">
    <Required Type="origin" File="debsig.gpg" id="7C7B196F2CB087F5"/>
  </Verification>
</Policy>
POLICY
```

A new signing subkey has a new ID, so its renewal (§4) adds a policy and keyring under the new ID.

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
| Artifact Signing identity validation | `<expiry date shown in the Azure portal; validated 2026-11-12>` | 60 days before (Microsoft also emails reminders from then) | create a new identity validation and attach it to the certificate profile; if it lapses, certificate renewal and signing stop ([FAQ](https://learn.microsoft.com/azure/artifact-signing/faq)) |
| Entra client secret (only if used) | `<creation + 12 months>` | 30 days before | create a new secret, update `AZURE_CLIENT_SECRET`, delete the old one |
| GPG signing subkey | 2028-11-11 | 60 days before | `gpg --quick-set-expire <PRIMARY FPR> 2y <SUBKEY FPR>` on the offline machine, re-export and republish the public key, replace `LINUX_SIGNING_KEY` |
| GPG primary key | 2031-11-11 | 90 days before | extend with `gpg --quick-set-expire <PRIMARY FPR> 5y`, republish |
| Cloudflare API tokens | `<creation + 1 year>` | 30 days before | create a new scoped token, replace the secret, revoke the old token |
| Environment review | every November | — | check reviewers, branch rules and that no secret sits at repository level |

### When a key or account is compromised

1. **Stop new updates.** Pause the affected channels in the update Worker's manifests ([ADR 0014 §4](../decisions/0014-desktop-distribution.md#4-the-kill-switch-and-bridges)'s kill switch, effective within 60 seconds). A paused channel still serves bridge releases.
2. **Revoke the identity.**
   - Apple: revoke the Developer ID certificate in the developer account, revoke the API key, and contact Apple Developer Support about the compromise so it can act on builds already signed.
   - Azure: remove the Signer role and the federated credential, then open an Azure support request to revoke the certificate profile's certificates (deleting the profile doesn't revoke them; [FAQ](https://learn.microsoft.com/azure/artifact-signing/faq)).
   - GPG: publish the revocation certificate for the subkey (or the primary key) to the website and keys.openpgp.org.
   - Updater key: follow [ADR 0014 §7](../decisions/0014-desktop-distribution.md#7-the-updater-keys-custody-and-rotation)'s rotation procedure: a bridge release signed with the old key that carries the new public key.
   - Cloudflare or GitHub: revoke the token and delete the environment secret.
3. **Make the new identity** with this runbook and replace the secrets.
4. **Re-sign and republish** the current release with the new identity, verify it with §3, and lift the pause.
5. **Tell users** in the release notes, on the download page, and in a GitHub security advisory: what was exposed, which builds to distrust and what to install.

### When Windows signing fails

The Windows legs stop at the first file that can't be signed, after two retries. The log names the file and SignTool's message.

- **`azure/login` fails with AADSTS70021 or AADSTS700213.** No federated credential matches the run. The error quotes the subject GitHub sent; the credential must name exactly that, `repo:laudebugs@25782070/laminapdf@1437211634:environment:desktop-release` (§1.5), and the job must run in `desktop-release`. A name-based subject (`repo:laudebugs/laminapdf:…`) never matches here, because the repository uses GitHub's immutable subject format.
- **SignTool reports 403 Forbidden.** The Entra app lacks the Artifact Signing Certificate Profile Signer role on `LaminaPdfPublicTrust`, or the identity validation lapsed (the renewal calendar above).
- **SignTool can't load the dlib.** The dlib is x64 only, so the build uses the x64 SignTool on both runners; an Arm64 SignTool can't load it.
- **The timestamp server or the signing service times out.** The script tries again twice, 15 and 30 seconds apart. If it fails after those retries, re-run the failed job.
- **The signature check fails on a file nobody signed.** A new PE file reached an installer by a path the signing misses, for example a new resource DLL. Sign it in the build's `beforeBundleCommand`, as PDFium's DLL is.

### If the project becomes a company

The publisher identities move to the company: a new Apple organisation membership, a new Artifact Signing organisation validation and a new GPG key. Releases signed after the move show the company's name. The updater key stays the same, because Tauri checks it independently of the OS publisher, so installed copies keep updating.

## 5. SmartScreen policy

A SmartScreen warning that names the publisher is acceptable for early releases; a prompt with no publisher name is a failure. Expect the warning on the first downloads: Microsoft's guidance is that a newly signed file shows "unrecognized" until it has download history, whatever the certificate: OV, EV and Artifact Signing alike, with the verified publisher name shown in the prompt. Reputation builds over several weeks and hundreds of clean installs, and later builds signed with the same identity can inherit some of it, so every release keeps the same signing identity ([SmartScreen reputation](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation)). D6d records what a clean Windows virtual machine shows for the first signed build.

Until releases install without a warning, the release notes carry this text:

> **Windows may show "Windows protected your PC".** Lamina PDF is new, so Microsoft SmartScreen hasn't seen many downloads of it yet. The installer is signed by <Legal Name>. To install it, select **More info**, check that the publisher is <Legal Name>, then select **Run anyway**. The warning goes away as more people install the app.

## 6. Cutting a release

The desktop release workflow, `.github/workflows/desktop-release.yml`, builds and publishes every release. Its first job, `plan`, decides from what started the run which version it builds, which channel it publishes to and which GitHub environment holds the secrets:

| Started by | Version | Channel and bucket | Environment |
| --- | --- | --- | --- |
| The schedule, 04:23 UTC every day | `X.Y.Z-nightly.YYYYMMDD.N`: the patch after the latest `desktop-v*` tag (`0.1.0` before the first), the date and the run number | `nightly`, `lamina-pdf-nightly` | `desktop-nightly` |
| A dispatch (`gh workflow run desktop-release.yml`) | `0.0.(100 + run number)` | `staging`, `lamina-pdf-staging` | `desktop-release` |
| A `desktop-vX.Y.Z` tag | `X.Y.Z`, plain: no `-rc.1` or other suffix | `beta`, `lamina-pdf-releases` | `desktop-release` |
| A pull request that changes the release files (the workflow, its actions, the Tauri configs, the plan code or the manifest tool's publish code) | a nightly version; Windows x64 also builds once in tag mode, to check the MSI | nothing is published | none |

macOS and Windows builds from `desktop-release` are signed ([D6c](../todos/done/D6c-macos-signing-and-notarisation.md), [D6d](../todos/done/D6d-windows-signing.md)), so `plan` publishes tags, and a staging dispatch needs no `allow_unsigned_platforms`. The workflow's `SIGNED_PLATFORMS` reads `macos,windows`; were a platform left out, `plan` would refuse a tag and a staging dispatch would publish only with that input.

Each run builds macOS (arm64 and x64), Windows (x64 and arm64) and Linux (x64 and arm64) on runners of their own OS and CPU. Every build makes the `.dmg`, the NSIS installer, the AppImage, the `.deb`, the updater's bundles and their `.sig` files, `SHA256SUMS` and the SBOMs. Tag and staging builds also make the MSI and the `.rpm`; nightlies can't, because neither format accepts a `-nightly` version. It fails when an installer reaches 16 MB (15 MB until 2026-11-15, [desktop plan §4](../plans/desktop-app.md#non-functional-targets-and-release-gates)). The AppImage, which carries GTK and WebKitGTK (about 90 MB), is reported but not gated ([desktop plan §4](../plans/desktop-app.md#non-functional-targets-and-release-gates)). Linux packages from `desktop-release` are signed with the GPG key (§1.6), each with a detached `.asc`, and `SHA256SUMS.asc`; Windows files from it are Authenticode-signed (§1.5). After the builds, `attest` attests every file, and the publish job uploads them to `releases/<version>/<platform>/` in the channel's bucket, makes them the channel's release, and asks the update Worker for each platform.

### The download page's lists

Once the update Worker serves a release, the publish job writes `downloads/<channel>.json` beside it in the channel's bucket (`manifest downloads`, `apps/updates/scripts/downloads.ts`). The file lists each installer (`.dmg`, setup `.exe`, `.msi`, AppImage, `.deb`, `.rpm`) with its size, its SHA-256 from the build's `SHA256SUMS`, its address and its `.asc`. An installer missing from `SHA256SUMS` stops the publish. The website's production deploy reads the lists from `downloads.laminapdf.com` and `nightly-downloads.laminapdf.com` (`pnpm --filter @laminapdf/site run downloads`), so a release shows on `laminapdf.com/download` at the next deploy: every six hours, or at once by running the **Website** workflow on `main`.

### How a macOS release is signed

Builds in `desktop-release` sign macOS with the Developer ID (§1.3) and notarise with the App Store Connect key (§1.4), in this order:

1. The build imports the certificate into a keychain of its own, with Apple's Developer ID G2 intermediate, which it checks against a pinned SHA-256.
2. `cargo xtask desktop-sidecar`, which Tauri runs before it bundles, signs PDFium's library with the Developer ID. Tauri signs the app and the engine but never the files in `Resources/`, and the hardened runtime's library validation would refuse a library without the engine's own Team ID.
3. Tauri signs the engine and the app with the Developer ID and the hardened runtime (`tauri.developer-id.conf.json`), then notarises and staples the app. The `.dmg` and the updater's `.app.tar.gz` both hold the stapled app.
4. The workflow notarises and staples the `.dmg`, which needs a ticket of its own for Gatekeeper's check while the Mac is offline. It tries a failed submission up to three times, waits on a submission Apple accepted rather than sending it again, and fails at once on a rejection, printing Apple's log.
5. It runs §3's checks, then deletes the keychain and the key from the runner.

Nightlies and dry runs are signed ad hoc and without the hardened runtime (`tauri.release.conf.json`). An ad-hoc signature has no Team ID, so the hardened runtime would make the engine refuse PDFium and draw nothing.

When notarisation fails, `xcrun notarytool log <submission id>` (in the build's log) names each file Apple refused. The usual reasons:

- **A file isn't signed with the Developer ID,** often a library in `Resources/`. Sign it in `cargo xtask desktop-sidecar`, as PDFium is.
- **An executable lacks the hardened runtime.** Check that the build used `tauri.developer-id.conf.json`.
- **A signature has no secure timestamp.** `codesign` adds one for a Developer ID; the sidecar step asks for it with `--timestamp`.
- **The key was refused (401).** The App Store Connect key was revoked or its secrets are wrong (§1.4).

### Nightlies

Nightlies need nothing from the owner. A night on which nothing the app is built from changed publishes nothing. The run's summary lists the version, files and sizes. A nightly installs on macOS with a Gatekeeper warning (ad-hoc signed) and on Windows with a SmartScreen warning (unsigned); the download page says so.

### A staging trial

1. Run `gh workflow run desktop-release.yml -f channel=staging`. Add `-f only_os=windows` (or `macos`, `linux`) to build and publish that OS alone, for example while another platform's signing waits on its vendor; the staging channel then serves only that OS. The publish job resumes the staging channel, which D5c's update test leaves paused after each of its runs.
2. Approve the `desktop-release` deployment each time GitHub asks: for the builds, then for the publish job.
3. Check the files with §3's commands. Staging builds never reach a user's install: the app offers no staging channel.

Add `-f dry_run=true` to build every target with no environment and publish nothing.

### A tagged release

A tag publishes to every beta install, so before the first one, run a staging trial and the person-only checks on clean virtual machines against its files: on Windows the SmartScreen prompt, the MSI's publisher in UAC, "Open with" and the MSI's "Managed by your organisation" ([D6d](../todos/done/D6d-windows-signing.md) lists them). Gatekeeper on macOS runs the same way before the first tag that includes a macOS build, which is [D7g](../todos/ready/D7g-macos-joins-the-desktop-beta.md)'s ([D6e](../todos/ready/D6e-macos-signed-release-checks.md) lists the checks).

1. Merge everything the release needs into `main`, and wait for the nightly to pass on it.
2. Tag the commit: `git tag desktop-v0.1.0 <commit> && git push origin desktop-v0.1.0`. The version must be plain `X.Y.Z`, with X and Y at most 255 and Z at most 65,535, the limits an MSI allows.
3. Approve the `desktop-release` deployment, twice: the builds start once you approve, and the publish job asks again when the builds have passed.
4. Run §3's checks on a clean virtual machine for each platform, and paste their output into the release's pull request or issue.

Every beta install now offers the release.

### Promote, roll out, pause

Stable releases come from beta. Promote a beta release to stable at 1 %, then raise the rollout, and pause at once if something goes wrong. The update service's README has the commands and the rules for each step: [Operations](../../apps/updates/README.md#operations).

## 7. Crash symbols

A native crash report names its frames by module and offset only, so each tagged release archives the debug symbols that turn those offsets back into function names and source lines. The app processes a crash's minidump on the user's computer and uploads only the frames and the modules' debug IDs ([D7d](../todos/done/D7d-native-crash-minidumps-and-symbols.md)). The symbols hold no user data, but only maintainers need them, so they stay in a private bucket.

### 7.1 What each release archives

Every build leg writes the symbols of the app's two binaries, the main process (`laminapdf-desktop`) and the engine sidecar (`laminapdf-desktop-engine`), with `.github/actions/desktop-build/symbols.sh`. PDFium has none: it is a prebuilt third-party library that ships without symbols, so its frames print as module and offset.

The release build keeps line tables (`CARGO_PROFILE_RELEASE_DEBUG=line-tables-only`): function names, source lines and inlined calls, at a fraction of full debug information's size. The shipped binaries still carry none of it, because each OS moves it out:

| OS | Debug information | How it leaves the binary |
| --- | --- | --- |
| macOS | a dSYM bundle | `split-debuginfo = packed`: rustc runs `dsymutil`, then strips the binary |
| Windows | a PDB | the linker writes it beside the executable |
| Linux | DWARF in the binary | `symbols.sh`, which the Tauri CLI runs before it bundles: `objcopy --only-keep-debug` into a `.debug` file, then `objcopy --strip-all` with a debug link |

Mozilla's `dump_syms` (MIT OR Apache-2.0, pinned with its checksums in `.github/actions/dump-syms`, version in `tools/xtask/src/doctor.rs`) turns each into a Breakpad `.sym` file. Each leg uploads them as the workflow artifact `symbols-desktop-<platform>-<mode>`, apart from the files that are attested and published. Dry runs make them too, so a pull request that changes the build shows whether the symbols still come out.

### 7.2 The bucket's layout and retention

The `archive-symbols` job uploads a tagged release's symbols to `lamina-pdf-symbols` in Breakpad's layout, which rust-minidump's symbol suppliers read:

```text
<debug file>/<debug ID>/<debug file without .pdb>.sym
<debug file>/<debug ID>/<raw debug file>
```

The debug file is the binary's name on macOS and Linux (`laminapdf-desktop-engine`) and the PDB's on Windows (`laminapdf_desktop_engine.pdb`). Beside each `.sym` file sits the raw debug file it came from: `<binary>.dSYM.tar.gz`, the `.pdb`, or `<binary>.debug`. They let a maintainer run a newer `dump_syms` later, or open a debugger on a crash they reproduce.

Only a `desktop-v*` tag push archives anything. Nightlies would add a set every day, and staging trials never reach a user's install. Until macOS and Windows are signed, `plan` refuses a tag (§6), so the first archive comes with the first tagged release. Keep every release's symbols for as long as any install of it can still send a report, so the bucket gets no lifecycle rule. The job's summary lists each file it uploads with its size.

### 7.3 Owner setup

The job needs one bucket and one R2 token, stored as two secrets. Without them it uploads nothing, leaves a notice in the run, and the release still publishes.

- [ ] In Cloudflare R2, create the bucket `lamina-pdf-symbols`. Leave it private: no public access, no custom domain, no `r2.dev` address.
- [ ] Create an R2 API token with Object Read & Write on `lamina-pdf-symbols` only. Store its value as `SYMBOLS_R2_TOKEN` and its Access Key ID as `SYMBOLS_R2_ACCESS_KEY_ID` in the `desktop-release` environment (§2). The job reads the account from the environment's `CLOUDFLARE_ACCOUNT_ID`.
- [ ] Give each maintainer who reads crash reports a Cloudflare account on Lamina PDF's account with read access to R2. Wrangler's `r2 object` commands call Cloudflare's REST API, which a token scoped to one bucket can't use. A maintainer who prefers a token creates an account API token with the Workers R2 Storage Read permission and sets it as `CLOUDFLARE_API_TOKEN` in their own shell.

### 7.4 Symbolicating a crash report

Save the report's JSON, as the crash reports bucket holds it, and run:

```sh
cargo xtask crash symbolicate report.json --fetch
```

The command reads the report's `native` object. For each module a frame names, it reads the module's `.sym` file from `target/crash-symbols/`. With `--fetch`, it downloads the files missing there from `lamina-pdf-symbols` with the repository's Wrangler (`wrangler r2 object get … --remote`, under your own login), and later runs reuse them without `--fetch`. Without `--fetch`, it never contacts the bucket. A report comes from a user's computer, so the command prints its strings with control characters replaced by `?`, and no terminal escape reaches your terminal. It then prints the crash reason and one line per frame, innermost first:

```text
Crash reason: EXC_BAD_ACCESS / KERN_INVALID_ADDRESS
#0 laminapdf-desktop-engine engine::draw (crates/…/render.rs:30) [inlined]
#0 laminapdf-desktop-engine engine::render (crates/…/render.rs:12)
#1 libpdfium.dylib +0x4d2
```

A function inlined into a frame gets a line of its own, marked `[inlined]`, above the frame it was inlined into. A frame whose module has no symbols, such as PDFium's or the OS's, prints its module and offset only: a native report never carries function names (the app has no symbols on the computer, and a name read from a dump's memory could be document text), so every name comes from the archived symbols here. To read symbols from a folder in the same layout instead of the bucket, for example a dry run's `symbols-desktop-*` artifact, add `--symbols <dir>`.

## 8. The Mac App Store build

Todo [D8c](../todos/done/D8c-app-store-build-and-upload.md); [ADR 0014 §11](../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build). The Mac App Store is a second way to install Lamina PDF, beside the direct download, and needs no notarisation: Apple's review replaces it. The workflow builds one universal app with the `app-store` feature (no updater, the sandbox's entitlements), signs it for the store, wraps it in a `.pkg` and uploads it to App Store Connect, where the owner submits the build for review. `.github/actions/desktop-app-store` does the work; `build-app-store-package` and `build-app-store` in `desktop-release.yml` call it.

### 8.1 What the owner provides

Five secrets in `desktop-release`, beside the keychain password and the App Store Connect key the direct download already uses (§2). The API key's role must be able to upload builds: Developer or App Manager.

| Secret | What it is |
| --- | --- |
| `APPLE_DISTRIBUTION_CERTIFICATE`, `APPLE_DISTRIBUTION_CERTIFICATE_PASSWORD` | the Apple Distribution certificate and its private key, exported as a `.p12`, base64 |
| `APPLE_INSTALLER_CERTIFICATE`, `APPLE_INSTALLER_CERTIFICATE_PASSWORD` | the Mac Installer Distribution certificate, the same way |
| `APPLE_APP_STORE_PROFILE` | the Mac App Store Connect provisioning profile for the explicit App ID `com.laminapdf.desktop` under team `35KBR7UHPZ`, base64; it must include the Apple Distribution certificate |

The job finds both certificates in its keychain, the app's by its hash and the installer's by its name (`productbuild` takes only a name), so it needs no identity secret. It reads the application identifier and the team from the profile and refuses one that isn't `35KBR7UHPZ.com.laminapdf.desktop`. It pins Apple's two Worldwide Developer Relations intermediates (G3, G6) by SHA-256, as §1.3's step pins Developer ID's.

### 8.2 What runs when

| Trigger | The store package |
| --- | --- |
| a pull request that changes the build | built, signed ad hoc and checked, with no secret; nothing is uploaded |
| a `desktop-v*` tag | signed with the store certificates, checked and uploaded, in `desktop-release`, after the owner approves |
| a dispatch with `app_store=package` or `upload` | the same two, for the staging version `0.0.(100 + run)`; a dry run only packages. `only_os` other than macOS refuses it |
| the nightly | none |

The store takes a version of up to three numbers, so it gets the release's own (`0.2.0`), never a nightly's pre-release, and a build number that must rise with each upload: the run's number and attempt (`123.1`), so a re-run uploads a newer build. The store job never holds a release's publish: a failure shows on the run and leaves the direct downloads alone. Keep that scheme: Apple compares the build number as dotted integers per version, so an upload made by hand with a larger first number (from Xcode or Transporter, say) would make every later build of that version look older.

The job passes the secrets to the action as inputs, and each step that needs one puts it in its own environment: the build, which runs cargo build scripts and npm packages, sees none. The signed package isn't kept as an artifact, since it went to Apple and holds the provisioning profile.

### 8.3 What the check proves

`.github/actions/desktop-app-store/check.sh` fails the job, before any upload, unless:

- the app, the engine and PDFium's library each hold arm64 and x86_64;
- the bundle identifier, version, build number, minimum system version (13.0) and category are right;
- the app's and the engine's entitlements are exactly the files in `apps/desktop/src-tauri/app-store/` (a signed app adds the two identifiers the profile grants), and PDFium's library has none;
- the app holds none of the updater's configuration: its public key, the endpoint it named and its install mode (`plugins.updater` is null in `tauri.appstore.conf.json`);
- nothing here proves the updater's and the crash reports' code is gone. It is still linked but unreachable (`Install::AppStore` and `crash::AVAILABLE` turn it off, as D8b's tests show). Compiling it out is a follow-up. Nor does it look at the menus: the build leaves "Check for updates…" and "Restart to update" out of them and the palette (the desktop crate's `menu` tests and `updateActions`' tests prove it);
- a signed package has an Apple Distribution signature, the profile inside, and the Mac Installer Distribution signature on the `.pkg`; an ad-hoc one has none of the three.

Before the store's signing, the build has its bundled engine draw a page, as every macOS build does. After it, the engine can run only inside the app's sandbox, so the check reads its signature and entitlements instead.

### 8.4 Trying it

1. `gh workflow run desktop-release.yml -f channel=staging -f only_os=macos -f dry_run=true -f app_store=package` builds and checks the package with no secret. The artifact `desktop-app-store-package` holds the `.pkg` and the app is signed ad hoc; to try the sandbox on a Mac, see `docs/agents/commands.md`'s local store build.
2. With the five secrets set, `-f dry_run=false -f app_store=upload` (and `-f only_os=macos`) signs the package and uploads it. Approve the `desktop-release` deployment. The build appears in App Store Connect's TestFlight after Apple has processed it (minutes to an hour).
3. Until the first real upload, the signed path (the certificates, the profile, `productbuild --sign` and `altool`) has never run: pull requests prove the ad-hoc path, and the signed one is tried in that first upload. `altool --upload-app` and `productbuild --sign <name>` were checked against Xcode 27's own help and manual, not against Apple's service.
4. The owner submits the build for review in App Store Connect (todo [D8d](../todos/ongoing/D8d-app-store-listing-and-first-review.md)).

### 8.5 Left to the owner

- **Export compliance.** The build doesn't set `ITSAppUsesNonExemptEncryption`: the app implements the PDF standard's encryption, so whether it qualifies for an exemption is the owner's answer to Apple's questions in App Store Connect, not a claim the build makes for them. Each upload then waits for that answer until the owner sets the key (or answers per build).
- **Hardened runtime.** The store build has none: the store requires App Sandbox, not the hardened runtime, and leaving it out keeps library validation from refusing PDFium.

## Sources

Checked 2026-11-12:

- Microsoft: [Artifact Signing quickstart](https://learn.microsoft.com/azure/artifact-signing/quickstart) (updated 2026-10-08), [Artifact Signing FAQ](https://learn.microsoft.com/azure/artifact-signing/faq) (updated 2026-10-27), [SmartScreen reputation](https://learn.microsoft.com/windows/apps/package-and-deploy/smartscreen-reputation) (updated 2026-08-17), [pricing](https://azure.microsoft.com/pricing/details/artifact-signing/)
- Tauri: [macOS signing](https://v2.tauri.app/distribute/sign/macos/), [Windows signing](https://v2.tauri.app/distribute/sign/windows/), [Linux signing](https://v2.tauri.app/distribute/sign/linux/), [RPM](https://v2.tauri.app/distribute/rpm/)
- Apple: [notarising macOS software](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- GitHub: [environments](https://docs.github.com/actions/deployment/targeting-different-environments/using-environments-for-deployment), [OIDC with Azure](https://docs.github.com/actions/security-for-github-actions/security-hardening-your-deployments/configuring-openid-connect-in-azure)
