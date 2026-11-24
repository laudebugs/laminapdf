# Mac App Store: the owner's setup, the listing and the review

This guideline holds everything the owner needs to put Lamina PDF on the Mac App Store: the pointer to the Apple setup (the runbook's §1.12), the listing's text ready to paste, the privacy label and export answers with their evidence, the notes for App Review, the screenshot plan, what happens when the store build and the direct download share a Mac, and an outline for submitting each release. It belongs to [D8d](../todos/ongoing/D8d-app-store-listing-and-first-review.md), and the store build it describes is [D8c](../todos/done/D8c-app-store-build-and-upload.md)'s. The release runbook, [desktop-release.md](desktop-release.md), keeps the direct download's signing.

**Who does what.** The owner does every step that needs the Apple account. An agent never has it, so an agent can draft and check text and code but can't click, export or submit anything. Nothing here was run on a Mac against Apple's live screens: each Apple page was read on 2026-11-22, and a step or label that couldn't be verified says "check Apple's current page". Recommendations are labelled as such.

**Decided already** ([ADR 0014 §11](../decisions/0014-desktop-distribution.md#11-the-mac-app-store-build)): the store build is a separate install line with the bundle identifier `com.laminapdf.desktop`, App Sandbox, no updater and no install ID, built from a stable release's commit. Apple's review replaces notarisation, which Apple hasn't enabled for the team (D6e and D7c wait on it).

## 1. The owner's Apple setup

The owner's step-by-step checklist is §1.12 of [the release runbook](desktop-release.md#112-mac-app-store-setup).

## 2. The listing

Paste each field into App Store Connect. Counts are characters, and Apple counts keywords and App Review notes in bytes; the text below is plain ASCII, so both agree. Limits are from Apple ([App information](https://developer.apple.com/help/app-store-connect/reference/app-information/app-information), [Platform version information](https://developer.apple.com/help/app-store-connect/reference/app-information/platform-version-information), checked 2026-11-22).

**Where every claim comes from.** The description is written for customers, in plain words. Each claim is backed by a source or by use of the installed build.

- **Source.** The owner approved the website's words ([`copy.ts`](../../apps/site/src/data/copy.ts), 2026-11-05) and the download page's ([`downloads.ts`](../../apps/site/src/data/downloads.ts)): the app reads PDFs "on your own computer, in tabs and windows", "files stay on your machine; the app works offline", and nothing you open is uploaded. The website says nothing yet about the store, so the store sentences come from ADR 0014 §11.
- **Removed.** On 2026-11-24 an agent used build 0.0.205 (105.2) on a Mac. Find, the page sidebar and outline, themes and accent colour, Reader mode, focus mode, crop and resize, reverse, and back and forward are greyed out there, so no text claims them ([#793](https://github.com/laudebugs/laminapdf/issues/793)). The app follows the Mac's light or dark appearance and can't set a theme. The app menu has no "Check for updates…" ([#788](https://github.com/laudebugs/laminapdf/issues/788)).
- **Working.** Open (the Open panel, Finder, drag, the Recent list), tabs and Move tab to new window, zoom, Fit width, Fit page, the page number box, Organise (the thumbnail grid, rotate, delete, Move to page, Page setup, the per-page panel with Duplicate pages), File > Document properties (viewing, and editing a title with Apply and File > Save, which writes the new title to the file), Save, Print, Settings and the command palette.
- **Known problem.** Remove metadata (a checkbox per value, then Mark for removal, Apply and Save) clears Author and Subject from the newest revision, but the save appends an incremental update, so the removed values stay in older revisions of the file ([#795](https://github.com/laudebugs/laminapdf/issues/795)). The description doesn't promise it.
- **Not yet tried on a Mac.** Insert's options, Save a copy and Save as, and Open With or a drop on the Dock icon.

| Field | Limit | Value | Count |
| --- | --- | --- | --- |
| Name | 30 | `Lamina PDF` | 10 |
| Subtitle | 30 | `Read PDFs on your own Mac` | 25 |
| Promotional text | 170 | see below | 149 |
| Description | 4,000 | see below | 818 |
| Keywords | 100 bytes | see below | 94 |
| Primary category | | Productivity | |
| Secondary category | | none (recommendation: Utilities is the closest, but a second category gains little; the owner may add it) | |
| Support URL | | `https://laminapdf.com/support` | |
| Marketing URL | | `https://laminapdf.com/` | |
| Privacy policy URL | required | `https://laminapdf.com/privacy/` after §3.3's change | |
| Copyright | | `2026 Laurence Ininda` | 20 |
| Price | | Free (Price Schedule: US$0.00, all territories) | |

**Promotional text** (149; changes without a new build):

```text
Read PDFs in tabs and windows, reorder pages and edit a document's details. It works offline, and nothing you open is uploaded. Free and open source.
```

**Keywords** (94 bytes; commas, no spaces after them, no other app's trademark; "PDF" is left out because the name carries it):

```text
reader,viewer,pages,organize,rotate,reorder,metadata,offline,private,document,open source,tabs
```

**Description** (818; written for customers in plain words, with no technical terms; plain text, no HTML):

```text
Lamina PDF is a simple, private PDF reader for your Mac. Open a PDF, read it, tidy it up, and keep it on your own computer. Nothing you open is ever uploaded, and it works without an internet connection.

What you can do

- Read your PDFs in tabs, or give each one its own window.
- Zoom in, fit a page to your screen, or jump straight to any page.
- Tidy up a document: rearrange pages, turn them around, or remove the ones you don't need. Save over the original or keep a new copy.
- Change a document's title, author and keywords.
- Print when you need to.
- Follow your Mac's light or dark appearance.

Private by design

No account, no sign-in and no tracking. Lamina PDF opens only the files you choose. Updates come through the App Store.

Free and open

Lamina PDF is free, and anyone can read its source code.
```

**Support URL.** `https://laminapdf.com/support` is a page on the website (`apps/site/src/pages/support.astro`) that shows the owner's support address, `support@laminapdf.com`, as text and as a link, so it carries the contact information Apple asks for. The same address goes in the app record's contact details if the owner wants it there. The page promises no response time, and it asks people not to send their PDFs.

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

**As answered, 2026-11-24.** The owner answered "Data Not Collected" in App Store Connect. The answer waits for the owner's Publish click, which is an attestation that the owner makes.

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

**As answered, 2026-11-24.** The owner answered that the app uses "standard algorithms in addition to the OS's" and Apple let the build through to Ready to Submit. The owner then removed France from availability. Apple's page on export-compliance documentation says that, for industry-standard encryption that isn't Apple's own, the French encryption declaration is to be uploaded, and that this is required only for distribution in France. No document is needed now. The same page lists a CCATS as an acceptable document. On the parts that could be read, it says nothing about other countries and mentions no self-classification report. Whether US export rules need anything from the developer, including BIS's annual report, stays the owner's question to a lawyer. If a document is ever needed, upload it in App Store Connect › the app › App Information › App Encryption Documentation › the + button (Account Holder, Admin or App Manager). This guideline gives no legal advice.

### 3.3 The privacy policy page

A privacy page exists: <https://laminapdf.com/privacy/> (`apps/site`, ADR 0011's promises). It describes the web reader and the website, and says only that the desktop app keeps a list of recent files. It doesn't say what the desktop app sends, so as it stands it understates the store build's good news and says nothing of the direct download's update checks. **A change to a promise goes back to the owner**, as the page's own comment says, so this is a draft, not an edit. Add this section before submitting (the direct download's lines are ADR 0014 §9 and §10):

> **The desktop app.** The desktop app opens PDFs on your computer and sends none of them anywhere. It keeps its settings, a list of recent files and your open tabs on your computer.
>
> The version from the Mac App Store has no updater, no analytics and no crash reporting of its own, and sends nothing to us. The App Store updates it. If you share analytics with app developers in System Settings, macOS may send Apple's own crash reports about the app, which Apple makes available to us in App Store Connect.
>
> The version from our website checks for updates once a day, and you can turn that off in Settings. A check sends your IP address, a user agent, the version and an anonymous install ID, and we keep nothing about it. It sends a crash report only if you turn that on, and the report holds no document, file name or folder.

## 4. The review notes

These go in App Store Connect › the version › **App Review Information**. The notes field takes 4,000 bytes ([Platform version information](https://developer.apple.com/help/app-store-connect/reference/app-information/platform-version-information)); this text is 2,036.

- **Sign-in required:** leave unchecked. There is no account.
- **Demo account:** none. Put `Not needed: the app has no accounts or sign-in.` in the notes, and leave the user name and password empty.
- **Contact:** the owner's first name, last name, email address and phone number. The number needs the international form with a leading `+` and country code. The agent can't fill these.
- **Attachment:** none. The notes say any PDF will do, and the sample files below are spare.

```text
Lamina PDF is a PDF reader for the Mac. Not needed: the app has no accounts, no sign-in and no demo account.

How to try it
1. Open the app. Choose File > Open a file, pick a PDF and it opens in a tab. You can also open a PDF from Finder (Open With > Lamina PDF), choose it from the Recent list on the start screen, or drop it on a window.
2. Use the zoom controls and the page number box in the toolbar, or View > Fit width and Fit page.
3. Choose the Organise tab to see every page as a thumbnail. Select a page to rotate, duplicate or delete it, or use Move to page.
4. Choose File > Document properties to see a document's title, author, subject and keywords.
5. Save changes with File > Save or Save a copy. The Save panel lets you choose where to write. File > Print opens the system print panel.
Any PDF will do for a test.

What the sandbox allows
- The app reads and writes only files you choose in the Open and Save panels or open from Finder. It keeps security-scoped bookmarks so File > Open recent and the restored tabs reopen those files after a relaunch.
- The app has no update check; the Mac App Store updates it. The app's own code makes no network request: it has no updater, no analytics and no crash reporting. The App Sandbox network client entitlement is present because WKWebView needs it to load the app's own page.

Things you may ask about
- Engine process: PDFs are parsed and drawn by a helper process inside the app bundle (laminapdf-desktop-engine, signed by us), started by the app and sandboxed with it. PDFium, the open-source PDF library from the Chromium project, is bundled in the app and loaded by that helper. Nothing is downloaded or executed from outside the bundle.
- No JavaScript from a PDF is ever run. Lamina PDF includes no script engine and treats scripts in a document as data.
- File associations: the app declares the PDF type as an alternate viewer. It does not make itself the default and does not change your defaults.

Source code and issues: https://github.com/laudebugs/laminapdf
```

**The update sentence can stay.** Build 0.0.205 has no "Check for updates…" in the app menu (checked 2026-11-24, [#788](https://github.com/laudebugs/laminapdf/issues/788)), so the notes say the app has no update check.

Each claim, checked against the code and the installed build: the engine's name is the bundle's `externalBin` (`tauri.bundle.conf.json`); the file type is declared with rank `Alternate` and role `Viewer` (`tauri.conf.json`); "no JavaScript" is [ADR 0021](../decisions/0021-pdf-javascript-policy.md). The paths in the notes were used on 2026-11-24, except Open With from Finder, which is still to check on a Mac. The first draft's Help and Report an issue lines were dropped from the notes because the build wasn't checked for them.

**Sample PDFs safe to give out.** The fixtures' README says every file in `fixtures/` was made by Lamina PDF's own scripts and is `MIT OR Apache-2.0`, with three exceptions ([fixtures/README.md](../../fixtures/README.md#licence)). Avoid the exceptions: `text-example.pdf` (DejaVu), anything in `fonts/`, and `sba-413.pdf` (a US agency's form; fine to use, but not ours to licence). Suggested attachments, renamed for the reviewer:

| Attach as | From | Why |
| --- | --- | --- |
| `invoice.pdf` | `fixtures/golden/examples/invoice.pdf` (12 KB) | a two-page tagged invoice with selectable text |
| `outline.pdf` | `fixtures/golden/links-and-outlines.pdf` (5 KB) | four pages, an outline, links, page labels; shows Find and the sidebar |
| `metadata.pdf` | `fixtures/golden/metadata.pdf` (3 KB) | every metadata field, for Document properties |

`fixtures/golden/examples/multilingual.pdf` (28 KB) shows scripts well, but it embeds Noto subsets under the SIL Open Font Licence, which allows redistribution; the three above need no such note. For a password prompt, the encrypted fixtures' passwords are in `fixtures/encrypted/passwords.json` (for example `r6-aes-256-user.pdf`), but the matrix doesn't prove the desktop app opens them, so don't offer one until the first build is tried.

## 5. Screenshots

**What was submitted.** The ten screenshots are of the real app, the installed store build 0.0.205, at 2560×1600, a size Apple accepted. They are JPEG with no alpha channel. They show five scenes, each in light and in dark:

1. Read, the document open at page 2.
2. Organise, the thumbnail grid.
3. Document properties, with sample values typed and then cancelled, so the file stayed unchanged.
4. A full page with a figure, through Fit page.
5. The start screen with the Recent list.

Every scene uses the Mamba paper (arXiv 2312.00752, by Gu and Dao, licensed CC BY 4.0). The owner uploaded the set in App Store Connect on 2026-11-24. They were made by running the installed build and capturing each window. The Mac's light or dark appearance was switched for the light set and then restored, since the app follows it. The app's colour theme and accent colour can't be changed in this build (the Theme, Accent colour and Page appearance items are disabled, [#793](https://github.com/laudebugs/laminapdf/issues/793)), so the screenshots vary only by the Mac's light and dark appearance.

The submitted set didn't come from [`tools/desktop/app-store-screenshots.mjs`](../../tools/desktop/app-store-screenshots.mjs), which stays for browser previews of the web reader and isn't for the store.

**Sizes** (checked 2026-11-22 on [Screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/screenshot-specifications)): one of 1280×800, 1440×900, 2560×1600 or 2880×1800 (all 16:10); `.jpeg`, `.jpg` or `.png`; no alpha channel; one to ten per size. A Mac screenshot (Shift-Command-5) of a window keeps its shadow and alpha, so capture the whole screen or flatten the image in Preview (File › Export, untick Alpha).

A screenshot must not show a feature the store build lacks. The scenes leave out annotation, forms and signing, which the web reader has and the desktop app doesn't yet.

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

## 8. State on 2026-11-24

The first submission is prepared in App Store Connect and isn't sent yet.

- Version 1.0, with build 0.0.205 (105.2) attached; manual release; free, available in 173 countries or regions. The owner removed China mainland (Apple confirmed the removal within 24 hours) because Apple lists a Security Assessment and an AI-content declaration as missing there, and an ICP number. The compliance page still showed both items as Missing Info right after the change, which Apple says can take 24 hours to clear. The owner also removed France (§3.2).
- Age rating 4+; category Productivity; support URL and privacy URL entered. The contact details are entered at the owner's request and aren't recorded here.
- The listing text, the ten screenshots (§5) and the TestFlight test information are filled in. The export answer and the privacy label are in §3.

Still open, all the owner's: the Publish click on the privacy label (the owner's attestation), Add for Review and Submit, and the owner's question about US export rules (§3.2). The Document properties check on 2026-11-24 found that a saved title edit works and that Remove metadata leaves the removed values in older revisions ([#795](https://github.com/laudebugs/laminapdf/issues/795)); Save a copy, Save as and Open With from the Dock are still untried.

## References

- Apple: [Certificates overview](https://developer.apple.com/help/account/certificates/certificates-overview), [Register an App ID](https://developer.apple.com/help/account/identifiers/register-an-app-id), [App Store provisioning profile](https://developer.apple.com/help/account/provisioning-profiles/create-an-app-store-provisioning-profile), [API keys](https://developer.apple.com/help/app-store-connect/get-started/app-store-connect-api), [Add a new app](https://developer.apple.com/help/app-store-connect/create-an-app-record/add-a-new-app), [App privacy details](https://developer.apple.com/app-store/app-privacy-details/), [Export regulations](https://developer.apple.com/documentation/security/complying-with-encryption-export-regulations), [Screenshot specifications](https://developer.apple.com/help/app-store-connect/reference/screenshot-specifications), [App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/).
- This repository: [ADR 0014](../decisions/0014-desktop-distribution.md) §6 and §9 to §11, [ADR 0021](../decisions/0021-pdf-javascript-policy.md), [the desktop plan §6 D8](../plans/desktop-app.md#d8-mac-app-store), [D8a](../todos/done/D8a-mac-app-store-plan.md), [D8b](../todos/done/D8b-app-sandbox-readiness.md).
