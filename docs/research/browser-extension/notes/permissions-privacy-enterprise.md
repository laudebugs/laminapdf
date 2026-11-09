# Lamina PDF extension: permissions, privacy, security and enterprise deployment

Research date: 2026-11-09. Several official pages returned only part of their content. Claims tagged **[unverified]** were not checked against a primary source on that date; treat them as leads, not facts.

## Q1. Which permissions does PDF interception need, and what warnings do they show?

### Takeaway
Fully automatic PDF takeover on every site needs broad host access. In Chrome, broad host access shows the scariest install warning ("Read and change all your data on all websites"). Most other useful permissions are silent or minor: declarativeNetRequest shows "Block content on any page", tabs and webNavigation show "Read your browsing history", and downloads shows "Manage your downloads". No rigorous public study puts a number on how much warnings cut install conversion.

### Cited Findings
- Chrome warning strings: `declarativeNetRequest` shows "Block content on any page". `declarativeNetRequestFeedback`, `tabs` and `webNavigation` each show "Read your browsing history". `downloads` shows "Manage your downloads". `declarativeNetRequestWithHostAccess`, `webRequest`, `contextMenus`, `storage`, `activeTab` and `scripting` show no warning of their own. — [Chrome permissions list](https://developer.chrome.com/docs/extensions/reference/permissions-list)
- Some warnings disappear when paired with a broader one: "the 'tabs' warning won't show if the extension also requests '<all_urls>'". — [Chrome permission warnings](https://developer.chrome.com/docs/extensions/develop/concepts/permission-warnings)
- If an update adds a permission that carries a warning, Chrome **disables the extension until the user accepts** the new permission. This is a major retention risk, so a PDF extension should request every install-time permission in its first release. — [Chrome permission warnings](https://developer.chrome.com/docs/extensions/develop/concepts/permission-warnings)
- `activeTab` shows no warning and grants temporary host access to the current site after a user gesture. — [Chrome permission warnings](https://developer.chrome.com/docs/extensions/develop/concepts/permission-warnings)
- Host-permission warning text: in Chrome, `<all_urls>` and `*://*/*` show "Read and change all your data on all websites" **[unverified: the warnings page points to the permissions reference, which did not show the string]**. — [Chrome permission warnings](https://developer.chrome.com/docs/extensions/develop/concepts/permission-warnings)
- Install-conversion impact: no rigorous, citable study measured the install drop caused by the all-sites warning. The figures in circulation, such as "22% lower revocation" and a "2024 study of 1,200 extensions", come from blogs with no source and should not be used. Developer guides report anecdotal conversion gains after moving to optional host permissions. — [chrome-extension-guide (blog, low quality)](https://chrome-extension-guide.pages.dev/2025/03/01/chrome-extension-permissions-explained/); [remio.ai (vendor blog, unsourced)](https://www.remio.ai/post/browser-extension-overload-is-spurring-fresh-demand-for-minimal-permission-lite-default-sets)
- How interception works **[unverified]**:
  - In MV3 Chrome, the PDF.js extension pattern detects `Content-Type: application/pdf` on main-frame responses and redirects to the extension viewer page. In MV3 this goes through `declarativeNetRequest` (DNR). DNR gained `responseHeaders` matching conditions around Chrome 128, which lets an extension redirect on Content-Type without blocking `webRequest`.
  - DNR redirects to the extension's own page need either host permissions or `declarativeNetRequestWithHostAccess` plus host permissions.
  - Firefox MV3 still supports blocking `webRequest`.
  - Primary source to verify: [chrome.declarativeNetRequest reference](https://developer.chrome.com/docs/extensions/reference/api/declarativeNetRequest)

### Inferences
- **Minimum permission set for full auto-takeover:** `declarativeNetRequestWithHostAccess` plus host permissions `<all_urls>` (or `http://*/*` and `https://*/*`), with `storage`. Add `contextMenus` ("Open with Lamina"), which costs no warning. Avoid `tabs`, `webNavigation`, `downloads`, `scripting` and `declarativeNetRequestFeedback` unless strictly needed: each adds a warning line or broadens what the extension can see.
- **Low-friction alternative:** install with **no host permissions**. Ship a toolbar button and context-menu "Open in Lamina" built on `activeTab`, plus a drag-and-drop or file-picker viewer page. Then offer an "Open all PDFs automatically" toggle that calls `permissions.request({origins:["<all_urls>"]})` from `optional_host_permissions`. This leaves zero warnings at install, and the scary prompt appears only when the user asks for takeover.

### Gaps
- No citable install-conversion study tied to permission warnings.
- Edge, Brave, Opera and Vivaldi warning strings were not verified; they likely mirror Chromium's.
- DNR `responseHeaders` support version and behaviour were not re-verified.

## Q2. Optional and runtime permissions, activeTab, and per-site access controls

### Takeaway
All four browser families now let users limit host access per site, and Firefox and Safari make host access revocable or ask per site. A PDF-takeover extension must therefore handle "permission missing" gracefully. It should check `permissions.contains` and fall back to an "Open in Lamina" button, so it never fails silently.

### Cited Findings
- **Firefox MV3 host permissions over time:**
  - Mozilla's original MV3 design (2022) made host permissions opt-in, managed from the unified extensions button. — [Mozilla Add-ons blog, Nov 2022](https://blog.mozilla.org/addons/2022/11/17/unified-extensions-button-and-how-to-handle-permissions-in-manifest-v3/)
  - From Firefox 109, all MV3 `host_permissions` were treated as optional. — [Bugzilla 1839129](https://bugzilla.mozilla.org/show_bug.cgi?id=1839129)
  - From **Firefox 127**, users are asked to grant MV3 host permissions in the install prompt, as with MV2. They can still revoke them at any time in about:addons, so Mozilla recommends checking with the permissions API. — [Mozilla Add-ons blog, May 2024](https://blog.mozilla.org/addons/2024/05/14/manifest-v3-updates/); [Bugzilla 1889402](https://bugzilla.mozilla.org/show_bug.cgi?id=1889402)
- In Firefox 127+, host permissions in `host_permissions` and `content_scripts` appear in the install prompt, but **host permissions added by an update are not shown to the user**. — [MDN host_permissions](https://developer.mozilla.org/docs/Mozilla/Add-ons/WebExtensions/manifest.json/host_permissions)
- Recommended Firefox pattern: check `permissions.contains` in `runtime.onInstalled`. — [MDN content PR 34094](https://github.com/mdn/content/pull/34094)
- Users can opt in or out of `optional_host_permissions` from the extension's Permissions tab in about:addons. Extensions request them at runtime with `permissions.request()`, which may show a dialog. — [MDN optional_host_permissions](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/optional_host_permissions)
- **Safari:** host access is granted per website.
  - When an extension first needs a site, the prompt offers "Allow for One Day", "Always Allow on This Website" or "Always Allow on Every Website". — [iDownloadBlog](https://www.idownloadblog.com/?p=968693); [Apple Safari User Guide](https://support.apple.com/guide/safari/ibrw56d804bd/mac)
  - Users can later set each site to Ask, Allow or Deny. — [Apple: Managing Safari web extension permissions](https://developer.apple.com/documentation/safariservices/managing-safari-web-extension-permissions)
  - A July 2024 anecdotal report describes a Safari bug where "Always Allow on This Website" wiped all-sites access (single, unconfirmed source). — [Michael Tsai blog](https://mjtsai.com/blog/?p=44204)
  - Developers have long complained about Safari's per-site friction. — [Apple Developer Forums](https://developer.apple.com/forums/thread/661919)
- **Chrome:** users can set extension site access to "On click", "On specific sites" or "On all sites" from the puzzle-piece menu **[unverified]**. activeTab grants temporary access to the current tab without a warning. — [Chrome permission warnings](https://developer.chrome.com/docs/extensions/develop/concepts/permission-warnings)

### Inferences
- In Safari, automatic takeover of PDF links on arbitrary sites will show a per-site prompt the first time unless the user picks "Every Website". The Safari onboarding screen should explain that choice. Treat Safari as "button-first, takeover opt-in".
- Design rule for every browser: on each PDF navigation, if host access is missing, do nothing to the page. Show a badge or action-button state ("PDF detected – open in Lamina"), and route the user's click through activeTab.
- Ask for optional permissions only from a user gesture inside the extension's own settings or onboarding page. This is required for `permissions.request` in Chrome and Firefox.

### Gaps
- Safari's handling of DNR redirects to extension pages under per-site permissions was not verified.
- Whether Safari exposes `optional_host_permissions` the same way as Chrome and Firefox was not verified.

## Q3. Store privacy requirements: what a no-data-collection extension must declare

### Takeaway
On Firefox, declare `"data_collection_permissions": {"required": ["none"]}`, which is **mandatory for new AMO submissions since Nov 3, 2025**. On Chrome Web Store, complete the privacy-practices tab, certifying no collection and compliance with the Limited Use policy, and publish a privacy policy. On Apple, the App Store privacy label for the containing app can be "Data Not Collected". Lamina's promise lines up naturally with all three.

### Cited Findings
- **Firefox `data_collection_permissions`:**
  - The key lives at `browser_specific_settings.gecko.data_collection_permissions` and takes `required` and `optional` lists. Users must accept required types to install. Optional types are requested later with `permissions.request()`. `technicalAndInteraction` can only be optional. — [Extension Workshop: built-in data consent](https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/)
  - Extensions that collect no data, or only optional data, must declare `"required": ["none"]`. — [Extension Workshop](https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/)
  - From **3 Nov 2025**, all new extensions must use the built-in consent system. Existing add-ons will be expected to adopt it later. — [Extension Workshop](https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/)
  - Supported in Firefox desktop 140+ and Firefox for Android 142+. On older versions, extensions that collect data must show their own consent UI or set `strict_min_version`. — [Extension Workshop](https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/)
  - Declared data appears in the install prompt and in about:addons under "Permissions and data". — [Extension Workshop](https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/)
- **Chrome Web Store Limited Use:**
  - Data may be used only for the disclosed practices and the extension's single purpose. Collecting browsing activity is banned except for a prominently described user-facing feature.
  - Transfers are allowed only when necessary, for legal compliance, for security, or in a merger with consent. Personalised ads, data brokers and credit uses are prohibited.
  - Developers must make an affirmative public statement, such as "The use of information received from Google APIs will adhere to the Chrome Web Store User Data Policy, including the Limited Use requirements".
  - Source: [CWS Limited Use](https://developer.chrome.com/docs/webstore/program-policies/limited-use)
- The privacy-practices tab, where developers state their single purpose, justify each permission and certify data use, was not retrieved **[unverified details]**. — [CWS program policies](https://developer.chrome.com/docs/webstore/program-policies/limited-use)

### Inferences
- A PDF extension "handles" web content and PDF bytes locally. Under CWS rules, data handled only on the device and never sent anywhere is generally not "collected". Still, write a privacy policy saying exactly that, and justify `<all_urls>` as "detect PDF responses to open them in the local viewer".
- Firefox's `"required":["none"]` puts "no data collected" directly in the install UI. That is a free trust signal Lamina should use.
- Any future opt-in telemetry or crash reporting must go in `optional`, never `required`. Opt-in crash reports would count as `technicalAndInteraction`.

### Gaps
- Exact CWS privacy-tab fields and Edge Add-ons privacy requirements were not fetched.
- Apple privacy label rules for Safari web extensions shipped inside a Mac or iOS app were not retrieved. Unverified: labels attach to the container app on the App Store.

## Q4. Security: CSP, untrusted PDFs, fingerprinting, remote code, supply chain, signing keys

### Takeaway
The largest real-world risk is supply chain: a hijacked publisher account pushes a malicious auto-update, as in the Cyberhaven incident of December 2024. Chrome's opt-in **verified CRX uploads** (May 2025) is the key mitigation, because it requires the developer's own private key on every upload. MV3's default CSP and its ban on remote code fit an on-device WASM viewer well.

### Cited Findings
- **Cyberhaven incident:**
  - On 24 Dec 2024, a phishing email posing as Google warned of a policy violation. It led a Cyberhaven employee with Chrome Web Store admin access to grant a malicious OAuth app, "Privacy Policy Extension", rights to manage their store items. — [BleepingComputer](https://bleepingcomputer.com/news/security/new-details-reveal-how-hackers-hijacked-35-google-chrome-extensions); [Obsidian Security](https://www.obsidiansecurity.com/blog/behind-the-breach-malicious-attack-on-cyberhavens-chrome-extension-developer-team)
  - The attacker published malicious v24.10.4, which auto-updated to users early on 25 Dec. It was detected at 23:54 UTC on 25 Dec, and clean v24.10.5 followed. Exposure was roughly 01:32 UTC on 25 Dec to 02:50 UTC on 26 Dec. Firefox and Edge builds were unaffected. — [Island.io](https://www.island.io/info/cyberhaven-and-extension-takeover-incident); [Reco.ai](https://reco.ai/blog/cyberhaven-supply-chain-attack-how-one-phishing-email-led-to-over-400-000-compromised-browsers)
  - Cyberhaven said about 400k users were affected. The code stole cookies and access tokens. — [Reco.ai](https://reco.ai/blog/cyberhaven-supply-chain-attack-how-one-phishing-email-led-to-over-400-000-compromised-browsers); [Security Affairs](https://securityaffairs.com/172491/hacking/chrome-browser-extensions-compromise.html)
  - The wider campaign hit at least 16 (early count) to 35 extensions. Public CWS contact emails made developers easy to target. — [Security Affairs](https://securityaffairs.com/172491/hacking/chrome-browser-extensions-compromise.html); [BleepingComputer](https://bleepingcomputer.com/news/security/new-details-reveal-how-hackers-hijacked-35-google-chrome-extensions)
  - Conflict: one source says the attack bypassed MFA. Cyberhaven's account says the employee had MFA and Advanced Protection, and the OAuth grant gave store access without compromising the Google account. The OAuth-consent route explains both versions. — [Reco.ai](https://reco.ai/blog/cyberhaven-supply-chain-attack-how-one-phishing-email-led-to-over-400-000-compromised-browsers) vs [Island.io](https://www.island.io/info/cyberhaven-and-extension-takeover-incident)
  - Mitigation advice: block OAuth grants to unapproved apps. — [BleepingComputer](https://bleepingcomputer.com/news/security/new-details-reveal-how-hackers-hijacked-35-google-chrome-extensions)
- **Chrome verified CRX uploads (May 2025):**
  - The developer registers an RSA public key. After that, any upload not signed with the matching private key is rejected, "even if the account or publishing workflow is compromised".
  - The store re-signs verified packages with its own key, so the extension ID stays the same. Developers opt in from the Package tab in the Developer Dashboard.
  - Source: [Chrome blog: Verified uploads in the CWS](https://developer.chrome.com/blog/verified-uploads-cws); [CWS: Update your item](https://developer.chrome.com/docs/webstore/update)
- **Unverified, not checked against a primary source:**
  - The MV3 default extension-page CSP is `script-src 'self' 'wasm-unsafe-eval'; object-src 'self'`. MV3 forbids loosening `script-src` to remote origins, and WASM requires `'wasm-unsafe-eval'`. — [Chrome manifest CSP](https://developer.chrome.com/docs/extensions/reference/manifest/content-security-policy)
  - MV3 forbids remotely hosted code; all executable code must ship in the package. — [Chrome MV3 remote code](https://developer.chrome.com/docs/extensions/develop/migrate/remote-hosted-code)
  - `web_accessible_resources` can be probed by websites to detect installed extensions. Chrome MV3 offers `use_dynamic_url: true` to rotate the resource URL. Firefox already uses a per-install random UUID in `moz-extension://` URLs. — [Chrome manifest web_accessible_resources](https://developer.chrome.com/docs/extensions/reference/manifest/web-accessible-resources)

### Inferences
- **CSP:** keep the viewer page's CSP at the MV3 default plus `'wasm-unsafe-eval'`. Add `connect-src 'self'`, or tighter, so the viewer page physically cannot make network requests. The exception is the one fetch of the PDF URL itself, which should run in the background or service worker, or through a scoped `connect-src` covering only the PDF's origin.
- **Untrusted PDF content:** never execute PDF JavaScript; Lamina's WASM renderer presumably has no JS engine. Render annotations' links as inert until clicked. Treat parser crashes as contained, since WASM memory is sandboxed, but impose memory and time limits so a hostile PDF cannot hang the tab.
- **Fingerprinting:** expose no `web_accessible_resources` at all if the DNR redirect targets an extension page, or use `use_dynamic_url`. This keeps sites from detecting "this user runs Lamina".
- **Supply chain:** opt into CWS verified uploads, keeping the private key in an HSM or offline (not in CI secrets). Turn on 2FA and Advanced Protection on all store accounts and keep the number of store admins small. Use a dedicated, non-public contact email. Publish release hashes. Firefox (AMO) signs with Mozilla's key and uses human or automated review. Safari ships through Apple's notarised app.

### Gaps
- Edge Add-ons and AMO equivalents to verified uploads were not researched.
- No source was found on PDF-parser CVEs in WASM viewers specifically.

## Q5. Proving privacy: reproducible builds and source-to-store matching

### Takeaway
No browser store offers reproducible-build attestation. The closest mechanisms are AMO's mandatory source-code submission for minified or transpiled code, and third-party CRX viewers that let anyone diff the published package against the repo. Lamina can publish deterministic build instructions plus SHA-256 of the store package and its unpacked files.

### Cited Findings
- No authoritative source on reproducible extension builds was retrieved.
- **Unverified:**
  - AMO requires source code and build instructions when a submission contains minified, concatenated or machine-generated code, so reviewers can reproduce the build. — [Extension Workshop: source code submission](https://extensionworkshop.com/documentation/publish/source-code-submission/)
  - CRXcavator (Duo/Cisco) was discontinued around 2023. Tools like "Chrome extension source viewer" (CRX Viewer) let users download and inspect the published CRX.

### Inferences
- **Practical proof stack:**
  1. Pin toolchain versions (Rust/wasm-pack, Node lockfile) and build in a container so the output is deterministic.
  2. Publish CI-built zip hashes in a GitHub release with SLSA provenance or a Sigstore attestation.
  3. Write a "verify" doc: download the CRX or XPI, strip the signature (for CRX, the header; for XPI, `META-INF`), and diff against the release zip. CWS re-signing changes only the CRX header, not the zip contents.
  4. Carry the web app's network indicator into the extension viewer: a visible "0 requests" badge driven by the viewer's own fetch wrapper, backed by a strict `connect-src` CSP.

### Gaps
- Status of Chrome/CWS support for provenance or attestation was not checked.
- No current (2026) tooling survey of extension source viewers.

## Q6. Enterprise deployment (Chrome, Edge, Firefox, Safari) and PDF policies

### Takeaway
Admins can force-install and pin the extension in every Chromium browser through ExtensionSettings or ExtensionInstallForcelist, and in Firefox through policies.json ExtensionSettings. Chrome's ExtensionSettings can **block** permissions and hosts but cannot **grant** optional permissions. Force-installed extensions do, however, get their manifest permissions without a prompt [unverified]. No policy makes an extension "the default PDF handler". AlwaysOpenPdfExternally only turns off the built-in viewer so PDFs download.

### Cited Findings
- **Chrome ExtensionSettings** — [Google Admin help: ExtensionSettings](https://support.google.com/chrome/a/answer/9867568)
  - `installation_mode` takes `allowed`, `blocked`, `force_installed` (cannot be removed; needs `update_url`), `normal_installed` (auto-installed but can be disabled) or `removed` (Chrome 75+).
  - `runtime_blocked_hosts` and `runtime_allowed_hosts` take up to 100 host patterns each.
  - `blocked_permissions` blocks API permissions. A blocked optional permission is simply declined at runtime.
  - `toolbar_pin` takes `default_unpinned`, `default_pinned` or `force_pinned`.
  - `file_url_navigation_allowed` (Chrome 119+) lets extensions navigate to file URLs.
  - `update_url` (CWS: `https://clients2.google.com/service/update2/crx`), `override_update_url` and `minimum_version_required` set where updates come from and the oldest version allowed.
- The Chrome page describes **no field that grants** API or host permissions. `runtime_allowed_hosts` only carves exceptions out of blocked hosts. — [Google Admin help](https://support.google.com/chrome/a/answer/9867568)
- **AlwaysOpenPdfExternally (Edge):** when enabled, Edge's PDF viewer is turned off and PDFs download for the default app to open. When unset or disabled, Edge opens PDFs. If Edge is the OS default PDF app, PDFs keep opening in Edge. — [Microsoft Learn: AlwaysOpenPdfExternally](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-browser-policies/alwaysopenpdfexternally)
- The same policy name exists in Chrome and Samsung Internet: `true` means external viewer, `false` means internal viewer, unset means the user decides. — [Samsung Browser policy](https://developer.samsung.com/browser/policy/always-open-pdf-externally.html)
- Vendors whose PDF extensions take over PDFs tell admins to set AlwaysOpenPdfExternally so PDFs reach the extension. — [BlackIce TIFF Viewer GPO doc](https://blackice.com/Help/Internet/TIFF%20Viewer%20webhelp/WebHelp/Enable_PDF_opening_in_Edge_Chrome_extension_through_Group_Policy_deployment.htm)
- Admins report wanting per-site exceptions to this policy, which Edge does not offer. — [Microsoft Tech Community](https://techcommunity.microsoft.com/discussions/edgeinsiderenterprise/allow-domain-based-exclusion-for-always-download-pdf-files-behaviour/3690144)
- **Firefox:**
  - The `3rdparty` policy (Firefox 68+) passes admin settings to extensions, which read them through `storage.managed`. — [Mozilla policy templates](https://mozilla.github.io/policy-templates/)
  - Policies also exist for `Extensions`, `ExtensionSettings`, `PDFjs` (disable or configure PDF.js) and `Handlers`. From Firefox 140, `DisableBuiltinPDFViewer` changes the PDF handler to send files to the OS. — [Mozilla policy templates](https://mozilla.github.io/policy-templates/)
  - The docs have moved to firefox-admin-docs.mozilla.org. — [Firefox admin docs](https://firefox-admin-docs.mozilla.org/)
  - Admins have asked how to auto-grant "access data on all websites" for MV3 extensions installed by policy. — [Mozilla Discourse](https://discourse.mozilla.org/t/allowing-accessing-data-on-all-websites-for-group-policy-installations-with-manifest-v3/131351)
- **Unverified:**
  - Chrome's ExtensionInstallForcelist takes `id;update_url` strings.
  - Edge has matching `ExtensionSettings` and `ExtensionInstallForcelist` policies, with the Edge Add-ons update URL `https://edge.microsoft.com/extensionwebstorebase/v1/crx`.
  - Firefox ExtensionSettings supports `installation_mode` (`force_installed`, `normal_installed`, `allowed`, `blocked`), `install_url`, `updates_disabled`, `default_area` and `private_browsing` (to allow in private windows).
  - Safari web extensions ship inside a Mac app, so MDM deploys the containing app. macOS 15 and iOS 18 era declarative management added Safari extension management settings (allowed, always on, always off, and per-domain permissions).
  - Sources to verify: [Apple Platform Deployment](https://support.apple.com/guide/deployment/welcome/web); [Microsoft Learn Edge policies](https://learn.microsoft.com/en-us/deployedge/microsoft-edge-policies)

### Inferences
- An **enterprise recipe for Lamina**:
  1. Force-install the extension and pin it.
  2. Ship a managed-storage schema (`storage.managed_schema` in Chrome, `3rdparty` in Firefox). Admins can then set "auto-open PDFs: on", "allowed or blocked hosts" and "disable any network fetch other than the PDF itself".
  3. Leave AlwaysOpenPdfExternally **unset**. Lamina's DNR redirect intercepts the navigation before the built-in viewer. Setting the policy would also send downloads to the OS app, and Edge ignores it when Edge is the default handler.
- Document the `runtime_blocked_hosts` interaction. Admins who block Lamina on sensitive intranet hosts lose takeover on those hosts, and the browser's built-in viewer takes over again.

### Gaps
- Not verified: whether force-installed Chrome and Edge extensions auto-receive `optional_host_permissions` (they do not, to my knowledge), and whether they bypass Chrome's per-site "On click" user setting.
- Exact Safari MDM payload keys for extension management were not verified.
- Firefox policy behaviour for MV3 host permissions on forced installs was not resolved (open Discourse thread).

## Q7. Incognito/private browsing and file:// access defaults

### Takeaway
Nothing in this section was checked against a primary source **[unverified]**. Extensions are off in incognito or private mode by default in Chrome, Edge and Firefox, and Safari has a per-extension private-browsing toggle. file:// access is off by default in Chromium ("Allow access to file URLs"), so local-PDF takeover needs a user-flipped toggle or an admin policy.

### Cited Findings
- Chrome 119+ `file_url_navigation_allowed` lets admins allow extensions to navigate to file URLs. — [Google Admin help: ExtensionSettings](https://support.google.com/chrome/a/answer/9867568)
- **[unverified]** Chrome and Edge:
  - Extensions do not run in Incognito/InPrivate unless the user enables "Allow in Incognito" on the extension's details page.
  - "Allow access to file URLs" is off by default. An extension can check it with `extension.isAllowedFileSchemeAccess()`.
  - The manifest `incognito` key (`spanning`, `split` or `not_allowed`) sets how the extension runs in incognito.
  - Sources to verify: [Chrome manifest incognito](https://developer.chrome.com/docs/extensions/reference/manifest/incognito); [chrome.extension API](https://developer.chrome.com/docs/extensions/reference/api/extension)
- **[unverified]** Firefox:
  - Since Firefox 67, new extensions do not run in private windows unless the user allows it.
  - Extensions generally cannot read `file://` content through fetch, and Firefox's own PDF.js handles local PDFs.
  - Source to verify: [Firefox support: extensions in private browsing](https://support.mozilla.org/kb/extensions-private-browsing)
- **[unverified]** Safari: users must allow each extension separately in Private Browsing (Safari 17+). — [Apple Safari User Guide](https://support.apple.com/guide/safari/ibrw56d804bd/mac)

### Inferences
- Use `incognito: "split"` or `"spanning"` deliberately. "Split" keeps any IndexedDB recent-files list separate from private sessions. Never persist "recent documents" from incognito.
- For local files, detect missing file access and show a one-click deep link to the extension's details page with instructions. Also offer the viewer's own file picker or drag-and-drop, which needs no file:// permission at all.

### Gaps
- None of Q7's defaults were confirmed against primary docs. They should be verified before publication.
