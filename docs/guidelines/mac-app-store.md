# Mac App Store: the owner's setup, the listing and the review

This guideline holds everything the owner needs to put Lamina PDF on the Mac App Store: the Apple setup in order, the listing's text ready to paste, the privacy label and export answers with their evidence, the notes for App Review, the screenshot plan, what happens when the store build and the direct download share a Mac, and an outline for submitting each release. It belongs to [D8d](../todos/ongoing/D8d-app-store-listing-and-first-review.md), and the store build it describes is [D8c](../todos/done/D8c-app-store-build-and-upload.md)'s. The release runbook, [desktop-release.md](desktop-release.md), keeps the direct download's signing.

**Who does what.** The owner does every step that needs the Apple account. An agent never has it, so an agent can draft and check text and code but can't click, export or submit anything. Nothing here was run on a Mac against Apple's live screens: each Apple page was read on 2026-11-22, and a step or label that couldn't be verified says "check Apple's current page". Recommendations are labelled as such.

**Decided already** ([ADR 0014 §11](../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build)): the store build is a separate install line with the bundle identifier `com.laminapdf.desktop`, App Sandbox, no updater and no install ID, built from a stable release's commit. Apple's review replaces notarisation, which Apple hasn't enabled for the team (D6e and D7c wait on it).

## 1. The owner's Apple setup

The block below, from `### 1.12` to the next `##` heading, is the owner's checklist and moves into [desktop-release.md](desktop-release.md) as §1.12 unchanged. It refers to the runbook's other sections by number only, so it reads the same there. The runbook's §8.1 names the five new secrets, and its §2 says where each goes.

### 1.12 Mac App Store setup

This is the checklist for the store side. Do the steps in order; each needs the one before it. Everything here is Apple Developer account work, and only the Account Holder or an Admin can do most of it. The team is individual, Team ID `35KBR7UHPZ` (§1.2). After step 6, the `desktop-release.yml` workflow can build and upload the store package (`app_store=upload`).

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

If `security find-identity -v -p codesigning` finds no valid identity after the import, or Keychain Access says the certificate "is not trusted", install Apple's current intermediate certificate from [apple.com/certificateauthority](https://www.apple.com/certificateauthority/) (the Developer ID setup needed the same, §1.3). The workflow's keychain on GitHub Actions needs it too, which D8c adds.

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
- [ ] **Create**, then paste the listing from §2 of the Mac App Store guideline into the version page and the App Information page.

#### Step 6: The secrets

Add each to the **`desktop-release`** environment (repository Settings › Environments › `desktop-release` › Environment secrets, §1.8). Encode each file as in step 2. **No secret value is ever pasted into a chat, an issue, a pull request or a commit**, and the agent never needs to see one. Delete the exported `.p12` and `.p8` files from disk once they are in the password manager.

| Secret | Value | New or reused |
| --- | --- | --- |
| `APPLE_DISTRIBUTION_CERTIFICATE` | base64 of the Apple Distribution `.p12` | new |
| `APPLE_DISTRIBUTION_CERTIFICATE_PASSWORD` | that `.p12`'s password | new |
| `APPLE_INSTALLER_CERTIFICATE` | base64 of the Mac Installer Distribution `.p12` | new |
| `APPLE_INSTALLER_CERTIFICATE_PASSWORD` | that `.p12`'s password | new |
| `APPLE_APP_STORE_PROFILE` | base64 of the `.provisionprofile` | new |
| `KEYCHAIN_PASSWORD` | the workflow's temporary keychain password | reused (§1.3) |
| `APPLE_API_KEY` | the Key ID | reused, or new with step 4 |
| `APPLE_API_ISSUER` | the Issuer ID | reused, or new with step 4 |
| `APPLE_API_KEY_P8` | the `.p8` file's contents | reused, or new with step 4 |

Section 8.1 names the five new secrets, and §2 (Secrets inventory) says where each goes and when it renews. If §8.1's names differ from this table, §8.1 wins. Add the renewal dates of the two certificates and the profile to §4's calendar.

## 2. The listing

Paste each field into App Store Connect. Counts are characters, and Apple counts keywords and App Review notes in bytes; the text below is plain ASCII, so both agree. Limits are from Apple ([App information](https://developer.apple.com/help/app-store-connect/reference/app-information/app-information), [Platform version information](https://developer.apple.com/help/app-store-connect/reference/app-information/platform-version-information), checked 2026-11-22).

**Where every claim comes from.** The owner approved the website's words ([`copy.ts`](../../apps/site/src/data/copy.ts), 2026-11-05) and the download page's ([`downloads.ts`](../../apps/site/src/data/downloads.ts)): the desktop app reads PDFs "on your own computer, in tabs and windows", "files stay on your machine; the app works offline", and nothing you open is uploaded. The description adds only features that the [capability matrix](../capabilities/README.md) proves on the desktop surface (pages and properties) or that the app's own menus carry (`apps/desktop/src-tauri/menus.json`: Open, Find, zoom and layout, theme, Reader mode, Print). **Recommendation:** the owner confirms each menu-backed line on the first store build, since nothing here was run on a Mac. The website says nothing yet about the store, so the store sentences in the description come from ADR 0014 §11.

| Field | Limit | Value | Count |
| --- | --- | --- | --- |
| Name | 30 | `Lamina PDF` | 10 |
| Subtitle | 30 | `Read PDFs on your own Mac` | 25 |
| Promotional text | 170 | see below | 149 |
| Description | 4,000 | see below | 1,452 |
| Keywords | 100 bytes | see below | 99 |
| Primary category | | Productivity | |
| Secondary category | | none (recommendation: Utilities is the closest, but a second category gains little; the owner may add it) | |
| Support URL | | see below | |
| Marketing URL | | `https://laminapdf.com/` | |
| Privacy policy URL | required | `https://laminapdf.com/privacy/` after §3.3's change | |
| Copyright | | `2026 Laurence Ininda` | 20 |
| Price | | Free (Price Schedule: US$0.00, all territories) | |

**Promotional text** (149; changes without a new build):

```text
Read PDFs in tabs and windows, reorder pages and edit a document's details. It works offline, and nothing you open is uploaded. Free and open source.
```

**Keywords** (99 bytes; commas, no spaces after them, no other app's trademark; "PDF" is left out because the name carries it):

```text
reader,viewer,pages,organize,rotate,reorder,find,metadata,offline,private,document,open source,tabs
```

**Description** (1,452; plain text, no HTML):

```text
Lamina PDF reads PDFs on your own Mac, in tabs and windows. Files stay on your computer, the app works offline, and nothing you open is uploaded.

What you can do

- Open a PDF from the Open panel, from Finder or by dropping it on a window. Keep several documents open in tabs, and move a tab to its own window.
- Find a word anywhere in a document, and step from match to match.
- Move around with the page sidebar, the outline of headings, Go to page and the back and forward buttons.
- Change how a document looks: zoom, fit width or fit page, page layout, a light or dark theme and an accent colour. Reader mode and focus mode keep the page in front of you.
- Organise pages: move, rotate, duplicate, delete and reverse pages, insert blank pages, and crop or resize them. Save the result over the original or as a new copy.
- Edit a document's title, author, subject, keywords and dates, or remove metadata you don't want to share.
- Print.

Private by design

Lamina PDF asks for no account and sends nothing about you or your files. The app can read only files you pick in the Open and Save panels or open from Finder. This version, from the Mac App Store, keeps no usage data and sends no crash reports of its own. The App Store updates it.

Open source

Lamina PDF is built from one Rust core that also powers a library for Rust and TypeScript and a web reader. The code is open source, under the MIT or Apache-2.0 licence.

Lamina PDF is free.
```

**Support URL.** The repository has no support address or page, and Apple wants a page with real contact information. **Needs an owner decision.** Recommendation: add a short `/support/` page to the website (a contact address the owner chooses, plus the issue tracker), and use `https://laminapdf.com/support/`. Until it exists, `https://github.com/laudebugs/laminapdf/issues` is the fallback, though it shows no contact details. The agent doesn't invent an address.

**Age rating.** Answer the questionnaire in App Store Connect › App Information › Age Rating ([Set an app age rating](https://developer.apple.com/help/app-store-connect/manage-app-information/set-an-app-age-rating); Apple's question list wasn't readable, so check the current screen). Expected answers: no in-app controls or capabilities that restrict content, no web access inside the app, no user-generated content, messaging, advertising or chance-based activity, and **None** for every content descriptor. That should calculate to **4+**. Documents the user opens aren't the app's content.

**Content rights.** The app shows no third-party content of its own. Answer that it doesn't contain, show or access third-party content, or check Apple's current wording.

**What's New** (every version after the first): write it per release from the changelog, in plain words.

## 3. The privacy label and export compliance

### 3.1 The label: "Data Not Collected"

**Recommended answer:** in App Store Connect › App Privacy, answer that you don't collect data, so the page shows **Data Not Collected**. Apple defines "collect" as transmitting data off the device so that you or your partners can access it for longer than it takes to serve the request, and says data processed only on the device isn't collected ([App privacy details](https://developer.apple.com/app-store/app-privacy-details/), checked 2026-11-22). It also says you aren't responsible for disclosing data that Apple collects. A privacy policy URL is required either way.

What the store build does, from the code (the `app-store` feature, [`Cargo.toml`](../../apps/desktop/src-tauri/Cargo.toml)):

| What | Evidence | Result |
| --- | --- | --- |
| Update checks | `lib.rs` doesn't register `tauri_plugin_updater` under the feature; `update::start` returns at once when the install is `AppStore`; the policy turns checking off (`Off::AppStore`); no install ID is made (`update_reset_install_id`) | none sent |
| Crash reports | `crash::AVAILABLE` is `false` under the feature: the reports, the consent prompt, the upload and the native-crash monitor never start (`lib.rs`, `crash/mod.rs`) | none sent |
| Analytics | none exist in the app | none sent |
| The webview's own page | loads from the app, under the content security policy quoted below (`tauri.conf.json`) | none |
| Links the user chooses | Help opens `docs.laminapdf.com`, Report an issue opens GitHub's issue form, and a link in a PDF opens in the browser, each only when clicked | the browser's, not the app's |
| Local logs | `tauri_plugin_log` writes to the container's log folder; document paths only at `debug` | stays on the Mac |
| macOS's own crash reports | sent to Apple, and to App Store Connect only if the user shares analytics (ADR 0014 §11) | Apple's, not ours |

The policy, in full:

```text
default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' blob: data: lamina: http://lamina.localhost; connect-src 'self' ipc: http://ipc.localhost lamina: http://lamina.localhost; worker-src 'self' blob:; font-src 'self'; object-src 'none'; frame-src 'none'; base-uri 'none'; form-action 'none'
```

**Where the code doesn't match the ADR's wording, for the owner:**

1. **Network code is linked in.** ADR 0014 §11 says the store build "carries no updater, no updater key". D8c's `tauri.appstore.conf.json` nulls `plugins.updater` (the runbook's §8.3), so the key and endpoint leave the store build's configuration. The updater's code, its crates (`tauri-plugin-updater`, `reqwest` and `rustls`, which `Cargo.toml` depends on without the feature) and the `updates.laminapdf.com` strings are still linked into the binary, unreachable. The label stays true, because nothing runs; the ADR's sentence is true of behaviour only. **Recommendation:** compiling them out is a separate follow-up, and a test that a store build makes no request (Tools › Network activity and a network monitor, §6's checks) belongs with D8c.
2. **The network entitlement is present** (`com.apple.security.network.client`) because WKWebView needs it. A reviewer may ask; the review notes (§4) answer it.
3. **"Check for updates…" stays in the app and Help menus** (`menus.json` is shared). The code turns checking off in this build and Settings says "Updates come from the App Store". **To check on a Mac:** choose it and confirm nothing is sent and the message is right.
4. **The website says the wrong thing for this build.** The download page's "What the app sends" says "Lamina PDF checks for updates once a day… Crash reports go only if you turn them on", which is the direct download's behaviour. The privacy page covers the web reader and lists the desktop only for its recent files. See §3.3 and §6.

**To check on a Mac (the label's evidence):** run the first store build with Little Snitch, Wireshark or `nettop`, open a document, change settings and quit, and confirm no connection from `Lamina PDF` or its engine. Record the result in the todo.

### 3.2 Export compliance

**This isn't legal advice, and the owner decides.** Apple treats an app that "uses, accesses, contains, implements, or incorporates encryption" as an export of encryption software, and asks questions at each submission unless the build's `Info.plist` answers them ([Complying with encryption export regulations](https://developer.apple.com/documentation/security/complying-with-encryption-export-regulations), read 2026-11-22). Apple's two pages that hold the question wording and the exemption list weren't readable, so the question text below is from memory and **must be checked on Apple's screen**.

**What the code does.**

- **Decrypts, never encrypts.** `laminapdf-syntax` carries the PDF standard security handler to open protected files: AES-128 and AES-256 (`aes` crate), RC4 (its own module), MD5 and SHA-2, all read-only for legacy ciphers (`crypt/mod.rs`). There is no encryption on write: the capability matrix lists "Encrypt a PDF" (G.02) with no proof, and `SaveOptions::with_remove_encryption` saves a protected file as plaintext after the user agrees. The brief for this todo said the app "encrypts"; today it doesn't. If E11 adds encryption on write, redo this analysis.
- **Its own implementations, not the OS's.** The AES and hashes are Rust crates in the app, not Apple's CryptoKit, so Apple's "built into the operating system" exemption doesn't cover them.
- **TLS** (`rustls` with `ring`) is linked, as §3.1 says, but never run in the store build. The webview's TLS is Apple's.
- **All algorithms are published standards** (ISO 32000-2's handler uses AES, SHA-2 and MD5; RC4 is in ISO 32000-1). Nothing is proprietary or non-standard.

**Questions Apple asks** (recalled, verify): whether the app uses encryption; whether it qualifies for an exemption under Category 5, Part 2 of the EAR; whether it uses proprietary or non-standard algorithms; and whether it is distributed in France. Apple lists the exemptions on its "Determine and upload app encryption documentation" page, whose text wasn't readable; check the wording on screen.

**Analysis.** Three routes are open.

**The exemption route.** Category 5, Part 2 of the Export Administration Regulations (EAR) exempts some items. A PDF reader isn't an information-security product, and its cryptography only opens a document the user already holds the password to. The EAR has a note on items whose primary function isn't security, and a carve-out for decryption-only items. Whether either applies is for the owner to confirm, because the regulation's text wasn't verified here.

**The mass-market route.** The owner self-classifies the app under Export Control Classification Number 5D992.c, using licence exception ENC (15 CFR 740.17(b)(1)). That holds only for standard algorithms, and it needs an annual self-classification report to the Bureau of Industry and Security (BIS) unless Apple holds your documentation ([Apple's note](https://developer.apple.com/documentation/security/complying-with-encryption-export-regulations); [BIS's report page](https://www.bis.doc.gov/index.php/policy-guidance/encryption/4-reports-and-reviews/a-annual-self-classification)).

**The documentation route.** The owner uploads documents to Apple and gets a code. It is the slowest route: about two business days once the documents are complete, per [Apple](https://developer.apple.com/help/app-store-connect/manage-app-information/determine-and-upload-app-encryption-documentation).

**The build doesn't answer for the owner.** The store build deliberately does not set `ITSAppUsesNonExemptEncryption` in its `Info.plist`. The app implements the PDF standard's encryption, so whether that is exempt is the owner's answer to Apple, not a claim the build makes. App Store Connect therefore asks the export-compliance questions at each submission, and the owner answers them there. Until the owner answers, each uploaded build shows "Missing Compliance" in App Store Connect, and that blocks TestFlight testing as well as review. Setting the key to `NO` later, once the owner has decided, is a one-line change in the store build's configuration.

**Recommendation (not legal advice).** Answer that the app uses encryption, that the algorithms are standard and not proprietary, and that it only decrypts the user's own documents. Take the exemption if the live question offers one that fits that use (decryption of data or files), and say so plainly if none does. Before the first submission the owner reads the live questions, decides between the exemption and mass-market routes, and for the mass-market route calendars the BIS report (annual, due 1 February per BIS's page: verify). **Distribution in France:** Apple lists the French controlled categories as secure storage, secure communications and anti-virus. A reader with decryption isn't one by name, though the owner may still ask a lawyer, or untick France under Pricing and Availability.

### 3.3 The privacy policy page

A privacy page exists: <https://laminapdf.com/privacy/> (`apps/site`, ADR 0011's promises). It describes the web reader and the website, and says only that the desktop app keeps a list of recent files. It doesn't say what the desktop app sends, so as it stands it understates the store build's good news and says nothing of the direct download's update checks. **A change to a promise goes back to the owner**, as the page's own comment says, so this is a draft, not an edit. Add this section before submitting (the direct download's lines are ADR 0014 §9 and §10):

> **The desktop app.** The desktop app opens PDFs on your computer and sends none of them anywhere. It keeps its settings, a list of recent files and your open tabs on your computer.
>
> The version from the Mac App Store has no updater, no analytics and no crash reporting of its own, and sends nothing to us. The App Store updates it. If you share analytics with app developers in System Settings, macOS may send Apple's own crash reports about the app, which Apple makes available to us in App Store Connect.
>
> The version from our website checks for updates once a day, and you can turn that off in Settings. A check sends your IP address, a user agent, the version and an anonymous install ID, and we keep nothing about it. It sends a crash report only if you turn that on, and the report holds no document, file name or folder.

## 4. The review notes

These go in App Store Connect › the version › **App Review Information**. The notes field takes 4,000 bytes ([Platform version information](https://developer.apple.com/help/app-store-connect/reference/app-information/platform-version-information)); this text is 2,281.

- **Sign-in required:** leave unchecked. There is no account.
- **Demo account:** none. Put `Not needed: the app has no accounts or sign-in.` in the notes, and leave the user name and password empty.
- **Contact:** the owner's first name, last name, email address and phone number. The number needs the international form with a leading `+` and country code. The agent can't fill these.
- **Attachment:** the three sample files below.

```text
Lamina PDF is a PDF reader for the Mac. It needs no account, no sign-in and no demo account.

How to try it
1. Open the app. Choose File > Open a file, pick a PDF and it opens in a tab. You can also open a PDF from Finder (Open With > Lamina PDF, or drag it onto the Dock icon) or drop it on a window.
2. Try Edit > Find in document, the page sidebar, View > Theme, and the pages view to move, rotate or delete pages.
3. Save changes with File > Save or Save a copy. The Save panel lets you choose where to write.
Sample PDFs are attached: invoice.pdf (two pages, selectable text), outline.pdf (four pages with an outline and links) and metadata.pdf (for File > Document properties). They are test files made for this project under the MIT or Apache-2.0 licence.

What the sandbox allows
- The app reads and writes only files you choose in the Open and Save panels or open from Finder. It keeps security-scoped bookmarks so File > Open recent and the restored tabs reopen those files after a relaunch.
- The app has no update check; the Mac App Store updates it. The app's own code makes no network request: it has no updater, no analytics and no crash reporting. The App Sandbox network client entitlement is present because WKWebView needs it to load the app's own page. Tools > Network activity lists every request a document causes.
- Printing uses the system print panel (File > Print).

Things you may ask about
- Engine process: PDFs are parsed and drawn by a helper process inside the app bundle (laminapdf-desktop-engine, signed by us), started by the app and sandboxed with it. PDFium, the open-source PDF library from the Chromium project, is bundled in the app and loaded by that helper. Nothing is downloaded or executed from outside the bundle.
- No JavaScript from a PDF is ever run. Lamina PDF includes no script engine and treats scripts in a document as data.
- File associations: the app declares the PDF type as an alternate viewer. It does not make itself the default and does not change your defaults.
- Help opens our documentation site in your default browser and Report an issue opens a GitHub form, only when you choose them. A link inside a PDF opens in your browser only when you click it.

Source code and issues: https://github.com/laudebugs/laminapdf
```

**Don't paste the update sentence yet.** "The app has no update check" is true once the follow-up pull request that hides "Check for updates…" in the store build merges: until then the menus still show it (§3.1, point 3). Paste that sentence only after the menu change is in the build you submit.

Each claim, checked against the code: the engine's name is the bundle's `externalBin` (`tauri.bundle.conf.json`); the file type is declared with rank `Alternate` and role `Viewer` (`tauri.conf.json`); "no JavaScript" is [ADR 0021](../decisions/0021-pdf-javascript-policy.md); the menu names are in `menus.json`; Help and Report an issue are `layout.ts`'s two links. **To check on the first build** before pasting: the menu paths, the shortcuts (`menus.json` lists Mod+O for Open and Mod+F for Find, but the notes leave them out until a Mac confirms they work), that Print opens the system panel in the sandbox, that "Open With" and the Dock drop work, and that `Tools › Network activity` exists in the store build.

**Sample PDFs safe to give out.** The fixtures' README says every file in `fixtures/` was made by Lamina PDF's own scripts and is `MIT OR Apache-2.0`, with three exceptions ([fixtures/README.md](../../fixtures/README.md#licence)). Avoid the exceptions: `text-example.pdf` (DejaVu), anything in `fonts/`, and `sba-413.pdf` (a US agency's form; fine to use, but not ours to licence). Suggested attachments, renamed for the reviewer:

| Attach as | From | Why |
| --- | --- | --- |
| `invoice.pdf` | `fixtures/golden/examples/invoice.pdf` (12 KB) | a two-page tagged invoice with selectable text |
| `outline.pdf` | `fixtures/golden/links-and-outlines.pdf` (5 KB) | four pages, an outline, links, page labels; shows Find and the sidebar |
| `metadata.pdf` | `fixtures/golden/metadata.pdf` (3 KB) | every metadata field, for Document properties |

`fixtures/golden/examples/multilingual.pdf` (28 KB) shows scripts well, but it embeds Noto subsets under the SIL Open Font Licence, which allows redistribution; the three above need no such note. For a password prompt, the encrypted fixtures' passwords are in `fixtures/encrypted/passwords.json` (for example `r6-aes-256-user.pdf`), but the matrix doesn't prove the desktop app opens them, so don't offer one until the first build is tried.

## 5. Screenshots

**These are drafts of the reader's interface in a browser, not the store build.** Apple's guideline 2.3.3 wants screenshots that show the app in use, so the final images must be retaken from the real store build on a Mac once D8c's first build exists ([App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/)). The scenes below make that quick: take the same six shots, in the same order, with the same files.

**Sizes** (checked 2026-11-22 on [Screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/screenshot-specifications)): one of 1280×800, 1440×900, 2560×1600 or 2880×1800 (all 16:10); `.jpeg`, `.jpg` or `.png`; no alpha channel; one to ten per size. **Recommendation:** upload 2880×1800, which Apple scales down for smaller displays, and keep the 1440×900 set as a spare. A Mac screenshot (Shift-Command-5) of a window keeps its shadow and alpha: capture the whole screen at a 1440×900 or 2880×1800 resolution, or flatten the image in Preview (File › Export, untick Alpha).

**The scenes** (the script, [`tools/desktop/app-store-screenshots.mjs`](../../tools/desktop/app-store-screenshots.mjs), makes each in the browser):

| # | Scene | Shows | Caption | File |
| --- | --- | --- | --- | --- |
| 1 | `01-read` | A report open with its pages beside it | Read PDFs in tabs and windows, on your own Mac | `golden/tagged-report.pdf` |
| 2 | `02-find` | Find open with "chapter" typed, matches highlighted | Find any word in a document | `golden/links-and-outlines.pdf` |
| 3 | `03-outline` | The outline in the sidebar | Jump to any heading from the outline | `golden/links-and-outlines.pdf` |
| 4 | `04-organise` | Organise mode over the pages | Reorder, rotate and delete pages | `golden/tagged-across-pages.pdf` |
| 5 | `05-properties` | The Document properties form | Edit a document's title, author and keywords | `golden/metadata.pdf` |
| 6 | `06-dark` | The dark theme over a multilingual page | Light and dark themes | `golden/examples/multilingual.pdf` |

**What the drafts show.** The six scenes were made and looked at on 2026-11-22 (2880×1800 and 1440×900, kept outside the repository). Scenes 1, 2, 3 and 6 read well. Scene 4 is sparse: its thumbnails are small in a large window, so on the Mac choose a document with more pages. Scene 5's form fills the window's height. Every scene shows the web reader's top bar, whose **Comment** and **Fill & Sign** tabs the desktop app's menus don't carry, and its status bar says "Offline-safe: nothing uploaded", which the desktop app may word differently. That is one more reason to retake them in the store build.

The scenes leave out annotation, forms and signing, which the web reader has and the desktop app doesn't yet (the matrix's desktop column covers pages and properties only). A screenshot must not show a feature the store build lacks.

**To make the drafts** (outside the repository, nothing is committed or uploaded):

```sh
cargo xtask web build
node tools/desktop/app-store-screenshots.mjs <folder outside the repository> [--only 02-find]
```

The script serves `apps/web/dist` locally, opens each fixture in Chromium and saves each scene at 2880×1800 and 1440×900 from a 1440×900 window. It refuses an output folder inside the repository. It waits for real conditions (a page drawn sharp, the sidebar's tab, Find's match count) instead of timers, turns animations and the text caret off, and aborts every request that leaves the local server, failing the run if one was made. It then reads each PNG's header and fails unless the image has the right size and no alpha channel, which Apple refuses; if it ever does, flatten the image in Preview (File › Export, untick Alpha). The fonts come from the host's operating system and the bundled web fonts, so the drafts differ a little from the Mac's own rendering.

## 6. When both builds are installed

The store build and the direct download share the bundle identifier `com.laminapdf.desktop` and the name `Lamina PDF.app`. The sources read include no Apple documentation for this case, so each answer below says what is known, what the code shows, and what to check on a Mac. **Nothing in this section was run on a Mac.**

### Which one opens PDFs

The app declares PDF as an alternate viewer (`rank: Alternate`, role `Viewer`), so neither build should become the default on its own: Preview or the user's choice stays (to check on a Mac). Nothing in the app sets a default, and [ADR 0008 §10](../decisions/0008-desktop-shell.md#10-opening-files-from-the-os) governs opening files from the OS. When the user picks Lamina PDF in Get Info › Open with, macOS's LaunchServices stores the choice by bundle identifier, so with two copies of one identifier it may pick either. Which it picks isn't documented.

**To check on a Mac:**

1. Install both: the direct copy in `~/Applications` and the store copy in `/Applications`, if the store lets you.
2. Run `/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -dump | grep -A12 "com.laminapdf.desktop"` and count the registered paths.
3. Control-click a PDF, choose Open With, and see whether two entries appear.
4. Choose one with Get Info › Open with › Change All, double-click a PDF and see which launches. Control-click the running app's Dock icon, then Options › Show in Finder, to see which copy it is.
5. Repeat with the other copy.
6. Run `open -b com.laminapdf.desktop` and see which opens.

### Does the store update or replace a copy it didn't install

What the sources say, none from Apple's current documentation:

- The Mac App Store marks an app as installed when its bundle identifier and version match, and it doesn't update apps installed another way ([Engadget, 2011](https://www.engadget.com/2011/01/07/third-party-apps-not-purchased-via-mac-app-store-may-appear-as-i/); [9to5Mac](https://9to5mac.com/?p=46405)).
- Apple's older installer documentation says the installer upgrades an existing app with the same bundle identifier ([Xcode help](https://help.apple.com/xcode/mac/8.3/en.lproj/dev16fb2b911.html)), so a store install could overwrite a direct copy at `/Applications/Lamina PDF.app`.
- A store-installed app carries a receipt, and one without a receipt that launches as a store build can be refused ([Apple Developer Forums](https://developer.apple.com/forums/thread/68327)); a direct copy has none and isn't affected.

So the safest reading is that the store's updates apply only to the copy it installed, and a first store install might replace a direct copy at the same path. **To check on a Mac, with a Mac that has no copy:**

1. Install the direct build to `/Applications`, open a PDF and note its settings (see "Where each keeps its settings" below). Install the store build from TestFlight (D8c's first upload) or the store. Does Finder, or the installer, ask to replace? Run `ls "/Applications/Lamina PDF.app/Contents/_MASReceipt"` and `codesign -dv --verbose=2 "/Applications/Lamina PDF.app"` to see which copy is there.
2. Install the store build first, then drag the direct build over it. Does Finder offer to replace, and does the direct app then keep updating itself?
3. With both at different paths and the store copy one version older than the direct, open the App Store's Updates tab: does it offer an update for the direct copy, or ignore it?
4. With the direct copy at `/Applications` and the store copy not installed, find Lamina PDF in the store: does the button say Open (it matched), Get, or Update?

### Where each keeps its settings

Both keep preferences, recent files, window state, the session, update settings and crash reports in the app's config and data folders (`app_config_dir` and `app_data_dir`, [`lib.rs`](../../apps/desktop/src-tauri/src/lib.rs); the files are `preferences.json`, `recent.json` and others, [`prefs.rs`](../../apps/desktop/src-tauri/src/prefs.rs), [`store.rs`](../../apps/desktop/src-tauri/src/store.rs)). Tauri puts these under `~/Library/Application Support/<identifier>`; App Sandbox redirects them into the container, so the two never share files:

| | Direct download | Store build |
| --- | --- | --- |
| Settings and data | `~/Library/Application Support/com.laminapdf.desktop/` | `~/Library/Containers/com.laminapdf.desktop/Data/Library/Application Support/com.laminapdf.desktop/` |
| Logs | `~/Library/Logs/com.laminapdf.desktop/` | the container's `Data/Library/Logs/` |
| Webview storage | `~/Library/WebKit/com.laminapdf.desktop/` | the container's `Data/Library/WebKit/` |
| Recent files | plain paths and bookmarks | security-scoped bookmarks beside the paths (`scoped.rs`) |

The paths come from Tauri's rules and ADR 0014 §11, not from a run. **To check on a Mac:** open a document in each build, change a setting, then `ls` those folders; confirm a setting changed in one doesn't show in the other, and that a recent file listed by one isn't listed by the other. A file the direct build saved is readable by the store build only after the user opens it in a panel, because the sandbox reaches no other file.

### The download page's words

The page may offer the store beside the direct download only after Apple approves the app, so this is text for later (draft; the website's copy goes to the owner, as `downloads.ts` says; no store link exists yet):

> **Two ways to install on a Mac.** Get Lamina PDF from the Mac App Store, and the store updates it. Or download it here, and the app updates itself. Both are the same app, with the same name. Choose one: each keeps its own settings and recent files, so a document you opened in one doesn't appear in the other.
>
> The Mac App Store version asks for no update permission, has no crash reporting of its own and sends nothing to us.

And the "What the app sends" paragraph needs a store sentence: "From the Mac App Store, the store updates the app and it sends nothing." Add what §6's checks find about which copy opens PDFs. Keep the sentences the checks confirm and drop the rest.

## 7. Submitting each release

*Outline; D8c's job and the first review will refine it.* The first review, and any answer Apple's reviewers asked for, go into [desktop-release.md](desktop-release.md) when D8c lands, as the todo says.

1. **D8c uploads the build** to App Store Connect from the release workflow, for a stable release only. A build processes for a while, then shows in TestFlight and under the version's **Build** section. Apple may email if processing fails.
2. **Answer export compliance, then check it on TestFlight.** The build leaves `ITSAppUsesNonExemptEncryption` unset, so every upload shows "Missing Compliance" until the owner answers Apple's question in App Store Connect (§3.2), and that blocks TestFlight testing too. Once answered, test it as an internal tester on a Mac with no other copy: open fixtures from Finder and the Open panel, Print, and check §3.1 and §6.
3. **Add the build to the version** (App Store tab, the version, **Build** › **+**), and write What's New.
4. **Re-check the version's fields**: screenshots, description, privacy answers, age rating, export compliance (answered in App Store Connect at each submission, §3.2) and the App Review information.
5. **Add for review** (Submit for Review). Choose the release option: manual release, so the owner decides the day after approval.
6. **When approved**, release it, then check the store page installs on a clean Mac, and update the download page's store link and the runbook.
7. **If Apple rejects it**, reply in App Store Connect's Resolution Center, change what they name and resubmit; record the finding in the runbook.

## References

- Apple: [Certificates overview](https://developer.apple.com/help/account/certificates/certificates-overview), [Register an App ID](https://developer.apple.com/help/account/identifiers/register-an-app-id), [App Store provisioning profile](https://developer.apple.com/help/account/provisioning-profiles/create-an-app-store-provisioning-profile), [API keys](https://developer.apple.com/help/app-store-connect/get-started/app-store-connect-api), [Add a new app](https://developer.apple.com/help/app-store-connect/create-an-app-record/add-a-new-app), [App privacy details](https://developer.apple.com/app-store/app-privacy-details/), [Export regulations](https://developer.apple.com/documentation/security/complying-with-encryption-export-regulations), [Screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/screenshot-specifications), [App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/).
- This repository: [ADR 0014](../decisions/0014-desktop-distribution.md) §6 and §9 to §11, [ADR 0021](../decisions/0021-pdf-javascript-policy.md), [the desktop plan §6 D8](../plans/desktop-app.md#d8-mac-app-store), [D8a](../todos/done/D8a-mac-app-store-plan.md), [D8b](../todos/done/D8b-app-sandbox-readiness.md).
