# Lamina PDF web extension: stores, review, costs, tooling, CI and testing (researched 2026-11-09)

Scope: one TypeScript/Vite codebase shipping a PDF-reader extension that bundles several MB of Rust-built WASM (PDFium, hayro) and minified JS, to Chrome, Edge, Firefox, Safari and Chromium forks. GitHub star counts, npm downloads and release dates were pulled live through the GitHub and npm APIs on 2026-11-09. Everything else is cited inline.

## Stores: fees, review time, size limits, WASM / minified / remote-code policy, source submission, privacy forms, single purpose

### Takeaway
Bundled, minified JS plus local WASM is acceptable at every store, provided no code is fetched remotely and nothing is *obfuscated*. Chrome and Edge need `'wasm-unsafe-eval'` in the MV3 CSP. Firefox AMO is the heavy one: reviewers rebuild from submitted source and diff the output byte for byte, so the Rust→WASM plus Vite build must be reproducible on Ubuntu 24.04 ARM64 with open-source tools. Opera's rules, read literally, forbid minified code unless unminified source and build instructions are supplied. Edge and Chrome both ask for a single-purpose statement and a remote-code declaration.

### Cited Findings
**Cross-store: WASM and CSP**
- MV3 extension pages may use `content_security_policy.extension_pages: "script-src 'self' 'wasm-unsafe-eval'"`. Chrome treats this as the minimum policy, and the extension_pages policy can't be relaxed beyond it. Chrome supports the directive from 103 and Firefox from 102. — [Chrome CSP manifest docs](https://developer.chrome.com/docs/extensions/reference/manifest/content-security-policy); [MDN content_security_policy](https://developer.mozilla.org/en-us/add-ons/webextensions/manifest.json/content_security_policy)
- A Chrome developer-relations reply confirms that extensions using `wasm-unsafe-eval` can be published to the Chrome Web Store. — [chromium-extensions group thread](https://groups.google.com/a/chromium.org/g/chromium-extensions/c/sJiaTnFMLHQ)
- None of the official policy pages I read (Chrome MV3 requirements, AMO source-submission page, AMO policies, Opera acceptance criteria) mentions WebAssembly explicitly. — [Chrome MV3 requirements](https://developer.chrome.com/docs/webstore/program-policies/mv3-requirements); [AMO source submission](https://extensionworkshop.com/documentation/publish/source-code-submission/); [Opera acceptance criteria](https://help.opera.com/en/extensions/acceptance-criteria/)

**Chrome Web Store**
- Fee: one-time US$5 developer registration, per account. Introduced for all developers in 2020. — [Chromium blog 2020](https://blog.chromium.org/2020/03/new-developer-dashboard-and.html?m=0); [extensionbooster 2026 guide (secondary)](https://extensionbooster.net/blog/chrome-web-store-developer-registration-fee-2026-guide/)
- Remote code: "full functionality of an extension must be easily discernible from its submitted code". External resources "must not contain any logic". eval of remotely fetched strings is a violation. Remote logic is allowed only through documented APIs (Debugger, User Scripts). Sandboxed pages and iframes are exempt from the remote-loading restriction, but functionality must still be determinable. — [Chrome MV3 additional requirements](https://developer.chrome.com/docs/webstore/program-policies/mv3-requirements)
- Review time: not fixed ("depends on the nature of your item"). An April 2026 PSA in the chromium-extensions group acknowledged longer, less predictable review times. — [PSA: Increase in review times](https://groups.google.com/a/chromium.org/g/chromium-extensions/c/VJ6DcpEn51Y)
- Package size: no official figure found. One secondary guide claims 200 MB, which is unverified. — [luckystrike guide (secondary)](https://theluckystrike.github.io/chrome-extension-guide/docs/guides/chrome-extension-bundle-size-optimization)
- Staged rollout: percentage rollout only for items with >10,000 seven-day active users. The percentage can only go up, and raising it needs no new review. Publishing a new version mid-rollout stops the prior version's rollout. — [CWS API docs](https://developer.chrome.com/docs/webstore/using-api?hl=en); [CWS update docs](https://developer.chrome.com/docs/webstore/update?hl=en)
- After approval with staged publish, you have up to 30 days to publish. — [publishers.items.publish v2 reference](https://developer.chrome.com/docs/webstore/api/reference/rest/v2/publishers.items/publish)

**Microsoft Edge Add-ons**
- Certification "can take up to seven business days". — [Publish a Microsoft Edge extension (ms.date 2026-05-05)](https://learn.microsoft.com/en-us/microsoft-edge/extensions/publish/publish-extension)
- The Partner Center Privacy page requires: Single Purpose Description ("clear, narrow, and well-defined purpose"), per-permission justification, an "Are you using remote code?" declaration ("In Manifest V3, loading and executing remotely hosted code is not permitted"), data-usage checkboxes and certification, and a privacy policy URL if any personal data is collected. Inaccurate disclosures can mean rejection. — [same page](https://learn.microsoft.com/en-us/microsoft-edge/extensions/publish/publish-extension)
- Initial listing must be done in Partner Center. The API only updates existing products and can't change metadata. — [Edge Add-ons API](https://learn.microsoft.com/en-us/microsoft-edge/extensions/update/api/using-addons-api)
- Fee: registration is free as far as I know, but the page I fetched doesn't state it (see Gaps).

**Firefox AMO**
- Source required when the code went through minifiers, bundlers (webpack/browserify), template engines, or "any custom tool that preprocesses files". Source must be attached to *every* version that needs it. — [AMO source code submission](https://extensionworkshop.com/documentation/publish/source-code-submission/)
- Reviewers rebuild and diff: "There must be no differences." You need a README with OS and tool versions and exact commands, plus lockfiles. Build tools must be open source and run locally (no web-based tools). — [same](https://extensionworkshop.com/documentation/publish/source-code-submission/)
- Default reviewer environment if none is specified: Ubuntu 24.04.4 LTS Desktop, **ARM64**, 10 GB RAM, 6 vCPU, Node 24.14.0, npm 11.9.0, 35 GB disk. Source archive limit is 200 MB. — [same](https://extensionworkshop.com/documentation/publish/source-code-submission/)
- The page allows tools that "transpile code from other languages" but gives no WASM-specific guidance. — [same](https://extensionworkshop.com/documentation/publish/source-code-submission/)
- Policies: add-ons must be "self-contained and not load remote code for execution". Obfuscation is prohibited. Minification is allowed, but source must be provided "before any of these steps have been applied". Submitted source is "not redistributed". Mozilla announced updated Add-on Policies effective **Dec 1, 2026** (preview linked on the page). — [Add-on Policies](https://extensionworkshop.com/documentation/publish/add-on-policies/)
- Data-collection disclosure: from Nov 3, 2025, new extensions must declare `browser_specific_settings.gecko.data_collection_permissions` in manifest.json (use `"none"` if no personal data). Mozilla planned to extend this to all extensions in H1 2026. — [Mozilla Add-ons blog 2025-10-23](https://blog.mozilla.org/addons/2025/10/23/data-collection-consent-changes-for-new-firefox-extensions/); [Extension Workshop: built-in data consent](https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/)
- Policy §6.2.1: add-ons compatible only with Firefox ≥140 that use the built-in consent "must accurately state the data collection practices in the extension manifest". — [Add-on Policies](https://extensionworkshop.com/documentation/publish/add-on-policies/)
- Fee: none (AMO is free). The fetched pages state no fee, so this rests on general knowledge (see Gaps).

**Opera add-ons**
- "We must be able to review your code, so it can't be obfuscated or minified" (third-party libraries exempt). If you don't want code public, provide a download link to unobfuscated source plus instructions to reproduce. Obfuscated submissions "may be handled with a lower priority". "No external JavaScript is allowed." — [Opera acceptance criteria](https://help.opera.com/en/extensions/acceptance-criteria/)
- Submissions failing any criterion are rejected. Developers should test on Mac and Windows. — [Opera publishing guidelines](https://help.opera.com/en/extensions/publishing-guidelines/)

### Inferences
- For Lamina, a public GitHub repo plus a pinned, containerised build script (Rust toolchain via rust-toolchain.toml, wasm-bindgen/wasm-pack versions, pnpm lockfile) is the natural AMO and Opera source package. Byte-identical WASM output across x86_64 CI and the reviewers' ARM64 Ubuntu is the main risk, so state the environment explicitly (e.g. a Dockerfile) in the README. PDFium-to-WASM builds through Emscripten/C++ are a bigger reproducibility risk than pure-Rust hayro.
- Shipping the reader in an extension page (e.g. `viewer.html`) with all WASM bundled locally satisfies the "no remote code" rules everywhere. Fetching WASM from a CDN at runtime would violate them.
- The single-purpose statement fits naturally: "View and annotate PDF files".

### Gaps
- No official Chrome Web Store or AMO package-size limit was found (200 MB for CWS is a secondary claim). Edge's limit wasn't found either.
- Edge registration fee, Opera fee and Opera review times weren't confirmed from primary pages.
- No first-hand 2026 reports turned up on WASM-heavy extensions going through AMO review (e.g. reviewers asking for Rust/C++ source and toolchain).
- Whether Mozilla's H1-2026 extension of `data_collection_permissions` to existing extensions actually took effect is unconfirmed.
- I didn't fetch the Chrome Web Store privacy-practices tab requirements (single purpose, permission justification, data-use certification). They're believed to mirror Edge's form, which is unverified here.

## Safari: packaging, fees, platforms, signing, distribution, API gaps

### Takeaway
Safari still requires a containing app. Since 2026 Apple offers a browser-based **Safari Web Extension Packager in App Store Connect** (no Mac or Xcode needed), which builds macOS and iOS apps (iOS app also runs on iPadOS and visionOS) and goes through TestFlight and App Store review. Forum reports show it's flaky and not visible on every account. The CLI route (`xcrun safari-web-extension-packager` + Xcode) remains for CI on macOS runners. Distributing outside the App Store via Developer ID plus notarisation is possible for macOS, but there are reports of enablement friction.

### Cited Findings
- App Store Connect packager: "package and distribute your Safari extensions using App Store Connect from any web browser, without requiring a Mac or access to Xcode". You create an app record (macOS, iOS or both), then Users and Access → Xcode Cloud → Safari Web Extension Packager → upload the full extension contents. Output goes to TestFlight or App Store review. Packaging compute is deducted from the 25 Xcode Cloud hours/month included with membership. Phased releases are available. — [Apple: Packaging and distributing Safari Web Extensions with App Store Connect](https://developer.apple.com/tutorials/data/documentation/safariservices/packaging-and-distributing-safari-web-extensions-with-app-store-connect.md)
- "The iOS app and extension [run] on iOS, iPadOS, and visionOS." — [same](https://developer.apple.com/tutorials/data/documentation/safariservices/packaging-and-distributing-safari-web-extensions-with-app-store-connect.md)
- The packager also exists as a command-line tool used alongside Xcode on a Mac. — [same](https://developer.apple.com/tutorials/data/documentation/safariservices/packaging-and-distributing-safari-web-extensions-with-app-store-connect.md); WXT's docs say to run `xcrun safari-web-extension-packager` on `.output/safari-mv3` — [WXT publishing](https://wxt.dev/guide/essentials/publishing)
- Developer reports that the packager section is missing from the Xcode Cloud tab, that exports fail, that uploads hit network errors, and that bundle-ID prefix errors occur. — [forum 845035](https://developer.apple.com/forums/thread/845035); [forum 845245](https://developer.apple.com/forums/thread/845245); [forum 844441](https://developer.apple.com/forums/thread/844441); [forum 843612](https://developer.apple.com/forums/thread/843612); [forum 802966](https://developer.apple.com/forums/thread/802966)
- WWDC 2026 session 216 covers Safari extensions (likely the packager announcement; not watched). — [WWDC26 session 216](https://developer.apple.com/videos/play/wwdc2026/216/)
- Fee: the Apple Developer Program costs US$99/year. It's needed for App Store Connect, TestFlight, Xcode Cloud and Developer ID notarisation. — [flaviocopes (secondary)](https://flaviocopes.com/apple-developer-free-vs-paid/); [analyticsinsight (secondary)](https://www.analyticsinsight.net/app-development/apple-developer-program-membership-costs-and-benefits)
- Outside the App Store: Safari extensions can be distributed outside the Mac App Store when notarised (Reddit, April 2025, citing Apple docs). A forum post reports that a signed and notarised extension still needed "Allow unsigned extensions" to enable, for MDM distribution (unresolved). — [Reddit r/Safari](https://www.reddit.com/r/Safari/comments/1jp5scj/safari_extensions_can_now_be_distributed_again/mlbv9a2/?context=3); [Apple forum 695373](https://developer.apple.com/forums/thread/695373)
- WXT doesn't generate the Safari wrapper ("Not supported yet"), so Safari publishing is manual. — [WXT publishing](https://wxt.dev/guide/essentials/publishing)
- Safari API gaps: third-party guides conflict. `offscreen` is generally listed as unsupported, `declarativeNetRequest` as partial, and `sidePanel` gets contradictory answers. — [luckystrike Safari porting guide (secondary)](https://theluckystrike.github.io/chrome-extension-guide/docs/guides/chrome-extension-safari-porting); [Apple forum profile thread on DNR](https://origin-devforums.apple.com/forums/profile/bruno.cm). Apple provides a compatibility-assessment doc — [Assessing your Safari web extension's browser compatibility](https://developer.apple.com/documentation/SafariServices/assessing-your-safari-web-extension-s-browser-compatibility)

### Inferences
- For Lamina, a GitHub Actions macOS runner can run `xcrun safari-web-extension-packager`, then `xcodebuild archive`/`-exportArchive` (or fastlane `gym` + `deliver`/`pilot`) using an App Store Connect API key. The browser-based packager is a no-Mac fallback but isn't scriptable per the docs.
- PDF interception (redirecting top-level `.pdf` navigations to the viewer) relies on `declarativeNetRequest`/`webRequest`, which are the weakest areas on Safari. Expect a degraded flow there (e.g. "Open in Lamina" via an action button or context menu).

### Gaps
- No primary Apple page on current Safari API support (per-API matrix) was fetched. No first-hand 2026 App Store review experiences for Safari extensions were found.
- Notarised Developer ID distribution flow for Safari web extensions in 2026 is unconfirmed by a primary Apple doc.
- Safari WASM CSP behaviour (`wasm-unsafe-eval`) wasn't confirmed.

## Chromium forks: Brave, Opera, Vivaldi, Arc installing from the Chrome Web Store

### Takeaway
Desktop Brave, Vivaldi and Arc install directly from the Chrome Web Store. Opera installs CWS extensions as well (historically via its "Install Chrome Extensions" helper) and also has its own store. Mobile variants (Brave iOS on WebKit, Brave Android) don't run CWS extensions, and Arc is in maintenance mode with a reported shutdown date.

### Cited Findings
- Brave desktop: any CWS extension via "Add to Brave". Brave iOS (WebKit) can't run CWS extensions, and Brave Android support is limited. Some extensions misbehave. — [Brave extensions guide 2026 (secondary)](https://go.tabbit.ai/brave-extensions); [Brave community](https://community.brave.com/t/why-would-a-chrome-extension-not-work-in-brave/129858)
- Vivaldi: CWS is the standard route. Vivaldi followed Chrome in disabling MV2. — [Vivaldi forum](https://forum.vivaldi.net/post/372296)
- Opera: forum posts disagree on whether a helper extension is still needed. — [Vivaldi forum on Opera extensions](https://forum.vivaldi.net/topic/3330/opera-extensions/8?)
- Arc: in maintenance mode since 2025 after The Browser Company pivoted to Dia. Atlassian acquired the company. It still gets Chromium updates and runs Chrome extensions. CyberInsider reports a shutdown on Dec 22, 2026, while other 2026 sources say no EOL had been announced (as of June 2026). — [The Register](https://www.theregister.com/a/5305114); [TidBITS](https://tidbits.com/?p=76090); [SupaSidebar review](https://supasidebar.com/blog/arc-browser-review-2026); [CyberInsider](https://cyberinsider.com/?p=346729)

### Inferences
- A single CWS listing covers Brave, Vivaldi, Arc and (probably) Opera desktop. A separate Opera add-ons listing is optional but runs into Opera's no-minification rule.
- Arc shouldn't be a target worth engineering for.

### Gaps
- No primary Opera doc on CWS install support in 2026 was found, and the Arc shutdown date is unverified against a primary source.
- Feature differences in forks (e.g. Brave Shields interacting with fetches of PDFs) weren't researched.

## Build tooling: WXT, Plasmo, CRXJS, extension.js, webextension-polyfill, chrome vs browser types

### Takeaway
**Recommend WXT** for a Vite pnpm monorepo. It's Vite-based, the most actively released of the four (v0.21.5, 2026-10-26), about 1.09M npm downloads/week, and the only one with built-in multi-store `submit` plus Firefox sources-zip generation. Plasmo has had no release since May 2025 and is effectively in maintenance mode. CRXJS was revived (v2.7.0, June 2026) but is a plugin only. `webextension-polyfill` was **archived on 2026-07-30** because Chrome 148+ exposes the `browser` namespace natively.

### Cited Findings
Live stats (GitHub/npm API, 2026-11-09):
| Tool | Stars | Latest release | Last push | Open issues | npm weekly DL |
|---|---|---|---|---|---|
| wxt-dev/wxt | 10,711 | wxt-v0.21.5, 2026-10-26 | 2026-11-06 | 194 | wxt 1,090,058 |
| PlasmoHQ/plasmo | 13,175 | v0.90.5, 2025-05-17 | 2026-11-08 | 370 | plasmo 366,569 |
| crxjs/chrome-extension-tools | 4,185 | vite-plugin-v2.7.0, 2026-06-19 | 2026-11-02 | 18 | @crxjs/vite-plugin 641,924 |
| extension-js/extension.js | 5,207 | v4.1.39, 2026-11-06 | 2026-11-07 | 6 | extension 10,716 |
| mozilla/webextension-polyfill | 3,066 (archived) | 0.12.0, 2024-05-14 | 2026-07-30 | 0 | 2,363,962 |
| mozilla/web-ext | 3,153 | 10.8.0, 2026-10-15 | 2026-11-09 | 230 | 398,189 |
| fregante/chrome-webstore-upload | 411 | v6.0.1, 2026-10-04 | 2026-10-04 | 0 | 87,365 |
| PlasmoHQ/bpp (Plasmo publish action) | 207 | — | 2025-02-12 | 12 | — |
Also @types/chrome 6,477,699/week and @types/webextension-polyfill 813,877/week. Sources: [wxt](https://github.com/wxt-dev/wxt), [plasmo](https://github.com/PlasmoHQ/plasmo), [crxjs](https://github.com/crxjs/chrome-extension-tools), [extension.js](https://github.com/extension-js/extension.js), [webextension-polyfill](https://github.com/mozilla/webextension-polyfill), [web-ext](https://github.com/mozilla/web-ext), [chrome-webstore-upload](https://github.com/fregante/chrome-webstore-upload), [bpp](https://github.com/PlasmoHQ/bpp), [npm downloads API](https://api.npmjs.org/downloads/point/last-week/wxt)

- WXT's own comparison says Plasmo "appears to be in maintenance mode with little to no maintainers nor feature development" (vendor source). — [wxt.dev compare](https://wxt.dev/guide/resources/compare)
- Plasmo still uses an old Parcel major, which blocks e.g. Tailwind v4. CRXJS development slowed through 2025 and then the community revived it with Vite 8 support (blog sources, interested parties). — [samy portfolio 2026](https://samy-portfolio.pages.dev/blog/2026/browser-extension-frameworks-wxt); [extensionbooster WXT vs CRXJS 2026](https://extensionbooster.net/blog/wxt-vs-crxjs-chrome-extension-framework-comparison-2026/); [extensionbooster Plasmo alternatives 2026](https://extensionbooster.net/blog/plasmo-alternatives-chrome-extension-framework-migration-guide-2026/)
- WXT multi-browser: `wxt build -b firefox|safari|edge`, `wxt zip -b firefox` emits extension *and* sources ZIP (it strips config/hidden/test files, and a `.env` can change chunk hashes). `zip.downloadPackages` bundles private packages via `npm pack`. Edge reuses the Chrome zip. — [WXT publishing](https://wxt.dev/guide/essentials/publishing)
- `webextension-polyfill` archived 2026-07-30: "this polyfill has served its purpose, and will not receive any further updates". Chrome 148 added the `browser` namespace, and Chrome 152 extended it to all Chrome extensions. The polyfill is a no-op where `browser` exists. — [webextension-polyfill README](https://github.com/mozilla/webextension-polyfill)
- Chrome docs: from Chrome 148 all extension APIs are available under `browser`, and `chrome` stays. They recommend `minimum_chrome_version: "148"` and direct use of `browser` for new extensions. An earlier Chrome 136 attempt was rolled back over promise-returning `onMessage` listeners. — [Chrome: browser namespace](https://developer.chrome.com/docs/extensions/develop/concepts/browser-namespace)

### Inferences
- Lamina's reader is already a Vite + React SPA, so WXT can host it as an unlisted page/entrypoint (e.g. `entrypoints/viewer.html`) importing the existing workspace package. A background entry then handles PDF redirection. WXT's `browser` export (or plain `browser` on Chrome ≥148) removes the polyfill question.
- WASM and workers: WXT inherits Vite's asset handling (`?url` imports, `new Worker(new URL(...), {type:'module'})`), so the reader's existing WASM loading probably transfers. MV3 CSP still needs `'wasm-unsafe-eval'` set in the WXT manifest config. This is an inference, since I didn't check WXT-specific WASM docs.
- Avoid Plasmo (stale Parcel, no Vite). CRXJS is viable if WXT's conventions clash with the monorepo, but you'd hand-roll multi-browser manifests and store submission.

### Gaps
- I didn't verify WXT's documented guidance on WASM and web workers, HMR limits for MV3 service workers, or Vite version support. extension.js capabilities weren't researched beyond its repo stats.

## CI publishing: store APIs and GitHub Actions tooling, staged rollouts

### Takeaway
Chrome (API v2, OAuth or service account), Edge (API v1.1, API key + Client ID) and Firefox (AMO API via `web-ext sign`, which also uploads source) can all be fully automated, and `wxt submit` wraps all three. Every store needs its first listing created by hand. Safari needs a macOS runner with xcodebuild/fastlane, or the manual App Store Connect packager.

### Cited Findings
- `wxt submit` supports Chrome Web Store, Edge Add-ons and Firefox AMO. `wxt submit init` writes `.env.submit`, and `--dry-run` tests secrets. The GitHub Actions example uses `workflow_dispatch`, pnpm, Node 22, `pnpm zip` + `pnpm zip:firefox`, and passes `CHROME_EXTENSION_ID/CLIENT_ID/CLIENT_SECRET/REFRESH_TOKEN` and `FIREFOX_EXTENSION_ID/JWT_ISSUER/JWT_SECRET` plus the sources zip. First listings must be created manually. Safari is "Not supported yet". — [WXT publishing](https://wxt.dev/guide/essentials/publishing)
- Chrome Web Store API v2 base is `https://chromewebstore.googleapis.com`, with endpoints `upload` (`/upload/v2/publishers/{PUBLISHER_ID}/items/{ID}:upload`), `fetchStatus`, `publish`, `setPublishedDeployPercentage` and `cancelSubmission`. OAuth scope is `https://www.googleapis.com/auth/chromewebstore`, and a service account is optionally supported. The manifest version must increase. If visibility was changed in the dashboard, publish manually once before the API works. — [Use the Chrome Web Store API](https://developer.chrome.com/docs/webstore/using-api?hl=en)
- The v2 `publish` accepts `deployInfos` (initial rollout percentage 0–100) and a STAGED_PUBLISH mode (publish within 30 days of approval). — [publishers.items.publish](https://developer.chrome.com/docs/webstore/api/reference/rest/v2/publishers.items/publish)
- `chrome-webstore-upload` (fregante) v6.0.0 (May 2026) / v6.0.1 (Oct 2026) is actively maintained. — [GitHub](https://github.com/fregante/chrome-webstore-upload)
- Edge Add-ons API v1.1: enable it in Partner Center → Publish API, which gives a Client ID and an API key (with expiry). Headers are `Authorization: ApiKey …` and `X-ClientID: …`. Endpoints under `https://api.addons.microsoftedge.microsoft.com`: `POST /v1/products/{id}/submissions/draft/package` (zip) → operation status → `POST /v1/products/{id}/submissions` (with certification notes) → publish status. v1 (access-token) support ended Dec 31, 2024. There are no API endpoints to create products or edit metadata. — [Edge Add-ons API (updated 2026-09-02)](https://learn.microsoft.com/en-us/microsoft-edge/extensions/update/api/using-addons-api)
- Firefox: `web-ext sign` v8+ uses the AMO submission API and handles initial submission and updates for listed and unlisted channels. — [Extension Workshop signing overview](https://extensionworkshop.com/documentation/publish/signing-and-distribution-overview/). Mozilla's web-ext command reference says `--channel` is required, `--upload-source-code <zip>` was added in v8, and `--approval-timeout` (0 = don't wait) exists. — [web-ext command reference](https://extensionworkshop.com/documentation/develop/web-ext-command-reference/); [web-ext option types](https://cdn.jsdelivr.net/npm/web-ext-option-types@9.2.2/index.d.ts). Latest web-ext is 10.8.0 (2026-10-15). — [GitHub](https://github.com/mozilla/web-ext)
- Other Actions: `kewisch/action-web-ext`, `violentmonkey/amo-upload`. — [action-web-ext](https://awesome.ecosyste.ms/projects/github.com%2Fkewisch%2Faction-web-ext); [amo-upload npm](https://www.npmjs.com/package/amo-upload)
- Apple: App Store phased releases are supported, and TestFlight handles betas. — [Apple packager doc](https://developer.apple.com/tutorials/data/documentation/safariservices/packaging-and-distributing-safari-web-extensions-with-app-store-connect.md)

### Inferences
- Pipeline for Lamina: the release tag triggers a reproducible build job that emits chrome.zip, firefox.zip and firefox-sources.zip. `wxt submit` (or chrome-webstore-upload, the Edge REST API and `web-ext sign --channel listed --upload-source-code`) runs from Linux. A macOS job runs `safari-web-extension-packager` → `xcodebuild` → fastlane `pilot`/`deliver` with an App Store Connect API key and signing certs held in secrets (match or keychain import).
- Only Chrome (≥10k weekly users) and the App Store offer staged rollouts. Edge and AMO have none that I found.

### Gaps
- Official CWS service-account setup steps and the v1.1 deprecation timeline weren't fetched. AMO-side staged rollout doesn't exist as far as I could find (unconfirmed). Edge API key expiry duration wasn't stated.

## Testing extensions: Playwright / Puppeteer / Firefox / Safari

### Takeaway
Playwright officially supports extensions only in its bundled Chromium via `launchPersistentContext` with `channel: 'chromium'` (headless works). Branded Chrome and Edge removed side-loading flags. Firefox extension E2E needs `web-ext run` or third-party helpers (`playwright-webextext`). I found no automated Safari extension testing path.

### Cited Findings
- Playwright: extensions "only in Chromium" with a persistent context. Chrome/Edge "removed the command-line flags needed to side-load extensions". Use `channel: 'chromium'` for headless. Get the MV3 service worker via `context.serviceWorkers()` / the `'serviceworker'` event. Workers suspend after about 30 s idle, and in-flight `evaluate()` calls may throw "Service worker restarted". — [Playwright: Chrome extensions](https://playwright.dev/docs/chrome-extensions)
- Firefox: Playwright has no official extension loading (long-standing feature request). `playwright-webextext` installs add-ons into Playwright Firefox through the remote debugging protocol, with incomplete MV3 support. — [Playwright issue #2874](https://github.com/microsoft/playwright/issues/2874); [playwright-webextext npm](https://npmjs.com/package/playwright-webextext); [author's DEV post](https://dev.to/ueokande/install-your-chrome-extensions-or-firefox-add-ons-to-the-browser-with-playwright-1lpl)
- Cloud options (BrowserStack, TestingBot) document Playwright extension testing. — [BrowserStack](https://www.browserstack.com/docs/automate/playwright/references/extension-testing-on-browsers); [TestingBot](https://testingbot.com/support/playwright/browser-extension-testing)
- Safari: a blog explains loading and debugging Safari 26 extensions without Xcode (manual). — [stefanvd blog](https://www.stefanvd.net/blog/?p=2105)

### Inferences
- Since the viewer is just an extension page, most Lamina tests can run against the plain SPA (existing web tests) plus a thin Playwright-Chromium suite for the PDF interception path. Firefox and Safari probably get smoke tests (web-ext lint/run, a manual TestFlight check).

### Gaps
- Puppeteer's 2026 extension support (Puppeteer has had `enableExtensions`/Firefox WebDriver BiDi work) wasn't verified. Whether safaridriver can drive Safari with web extensions enabled wasn't verified.
