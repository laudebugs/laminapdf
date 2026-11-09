# Cross-browser extension platform and PDF interception for Lamina (as of Nov 2026)

Method note: browser API support below comes mostly from MDN's browser-compat-data (BCD) JSON, fetched 2026-11-09 from `github.com/mdn/browser-compat-data` (main branch). BCD is maintained by the community and can lag behind releases. I read the pdf.js Chromium extension source directly from `mozilla/pdf.js` master on the same date. Edge, Brave, Opera, Vivaldi and Arc are Chromium-based, so they follow the Chrome column: BCD lists Edge and Opera as "mirror" of Chrome. I did not verify the fork-specific stores.

## 1. Manifest V3 status per browser; is MV2 still accepted?

### Takeaway
Use MV3 everywhere. Chrome (and Chromium forks) no longer runs MV2. Firefox still accepts MV2 and has no end date. The background context differs by browser: Chrome needs a service worker, Firefox needs an event page (`background.scripts`), and Safari accepts both. Declare both in one manifest, or ship per-browser manifests. Since Chrome 148, `browser.*` works in Chrome too.

### Cited Findings
- Chrome's MV2 timeline: MV2 was disabled for all users in Chrome 138 (July 24, 2025). "Manifest V2 extensions will cease to function for any user upgrading to Chrome 139", which also removed the enterprise policy. On **August 31, 2026**, "All remaining Manifest V2 extensions have been removed from the Chrome Web Store." — [Chrome MV2 support timeline](https://developer.chrome.com/docs/extensions/develop/migrate/mv2-deprecation-timeline)
- Firefox "has no plans to deprecate MV2". If that changes, Mozilla promises "a notice of at least 12 months". Firefox MV3 keeps event pages (DOM-based background scripts) and blocking webRequest, "a divergence from Chrome". One manifest can declare both `service_worker` and `scripts`, which works in Chrome 121+ and Firefox 121+. — [Mozilla Add-ons blog, Mar 2024](https://blog.mozilla.org/addons/2024/03/13/manifest-v3-manifest-v2-march-2024-update/). Feb 2025 press coverage restates the MV2 commitment ([BleepingComputer](https://bleepingcomputer.com/news/security/firefox-continues-manifest-v2-support-as-chrome-disables-mv2-ad-blockers)). I found no newer 2026 primary Mozilla statement.
- `background.service_worker`: Chrome 88, **Firefox: not supported**, Safari 15.4. `background.scripts` is MV2-only in Chrome but works in Firefox, and in Safari "for use in Manifest V2 or later". `background.type` (module) is supported in Chrome 92, Firefox 112 and Safari 16.4. `background.preferred_environment` is supported in Firefox 136 and Safari 18, not Chrome. — [BCD manifest/background](https://github.com/mdn/browser-compat-data/blob/main/webextensions/manifest/background.json)
- Namespace: "From Chrome 148, all Chrome Extension APIs are available under the `browser` namespace in addition to the existing `chrome` namespace." Chrome 148 also lets `runtime.onMessage` listeners return a Promise. An earlier attempt in Chrome 136 was rolled back. — [Chrome docs: browser namespace](https://developer.chrome.com/docs/extensions/develop/concepts/browser-namespace)
- Chrome's DNR reference marks `responseHeaders` as "Chrome 128+ browser.* since Chrome 148". — [Chrome DNR reference](https://developer.chrome.com/docs/extensions/reference/api/declarativeNetRequest)

### Inferences
- A single codebase can target `browser.*` with promises: natively in Chrome/Edge 148+, Firefox and Safari, with a polyfill or `globalThis.browser ?? chrome` for older Chromium builds and forks that lag behind upstream.
- The background script must not depend on DOM or service-worker-only globals, because it runs as an event page in Firefox and as a service worker in Chrome. pdf.js's Chromium extension avoids keeping state in background globals: it uses `chrome.storage.session` to survive service worker restarts (see §2).

### Gaps
- I did not fetch Edge's or Safari's own MV2 deprecation statements. BCD shows Safari still accepts MV2 manifests (persistent:false), but I found no Apple end date.
- I did not confirm Chromium-fork lag (Brave, Opera, Vivaldi, Arc) relative to Chrome 148 or 128.

## 2. PDF interception: how to route PDF navigations to the extension's viewer

### Takeaway
**Chromium (128+):** use the pdf.js approach. Dynamic declarativeNetRequest rules with `responseHeaders` conditions (`content-type: application/pdf`) redirect `main_frame`/`sub_frame` requests to a web-accessible viewer page with `?file=<url>`. Add a `webNavigation` fallback for `file://` and a content script for `<embed>`/`<object>`. **Firefox:** DNR has no response-header conditions, so use blocking `webRequest.onHeadersReceived` (still allowed in MV3) and return `redirectUrl`. **Safari:** there is neither a response-header DNR condition nor blocking webRequest, so interception is limited to URL patterns (`.pdf`) via DNR `extensionPath` redirects or `webNavigation` + `tabs.update`. Even that is buggy. Content-Type-only PDFs cannot be caught before Safari renders them.

### Cited Findings — Mozilla pdf.js Chromium extension (source read 2026-11-09)
- The manifest is MV3 with `minimum_chrome_version: "103"`. Permissions: `declarativeNetRequestWithHostAccess`, `webRequest`, `tabs`, `webNavigation`, `storage`, `alarms`. `host_permissions: ["<all_urls>"]`. A content script runs on `http/https/file` with `all_frames: true` at `document_start`. CSP is `extension_pages: "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'"`. `incognito: "split"`. `content/web/viewer.html` is listed in `web_accessible_resources` for `<all_urls>`. — [pdf.js extensions/chromium/manifest.json](https://github.com/mozilla/pdf.js/blob/master/extensions/chromium/manifest.json)
- `pdfHandler.js` registers **dynamic** DNR rules once per session, guarded by a `chrome.storage.session` flag. The comment notes that `runtime.onInstalled` does not always fire, for example in incognito ([crbug 41029550](https://issues.chromium.org/issues/41029550)). The rules, in priority order:
  1. `allow` for URLs containing `pdfjs.action=download`, which acts as an escape hatch.
  2. Redirect `^file://.*\.pdf$` (main_frame/sub_frame).
  3. In **sub_frame**, `allow` (that is, respect) `Content-Disposition: attachment*`. In main_frame the viewer is shown regardless, "since most often users want to view a PDF, and servers are often misconfigured".
  4. In main_frame, if the URL contains `=download` and has `Content-Disposition: attachment`, allow it. This is for Google Drive (pdf.js #6106).
  5. Redirect when `content-type` is `application/pdf` or `application/pdf;*`, with `excludedRequestMethods: ["post"]`. The source comment explains: "The viewer does not have the original request context and issues a GET request. The original response to POST requests is unavailable."
  6. Redirect `.pdf` URLs served as `application/octet-stream`.
  7. Redirect when `Content-Disposition` names a `*.pdf` file and `Content-Type` is octet-stream. Because `responseHeaders` entries are OR'ed, this uses an `excludedResponseHeaders` double negation to get AND.
  
  The redirect uses `regexSubstitution: VIEWER_URL + "?DNR:\\0"`, because "DNR does not support transformations such as encodeURIComponent". That is a pending proposal: [WECG #636](https://github.com/w3c/webextensions/issues/636#issuecomment-2165978322). — [pdf.js pdfHandler.js](https://github.com/mozilla/pdf.js/blob/master/extensions/chromium/pdfHandler.js)
- Feature detection: Chrome 123–127 accept `responseHeaders` but silently ignore it. Without detection, the rules would redirect *every* request ([crbug 347186592](https://crbug.com/347186592)). pdf.js probes with session rules: valid `responseHeaders` should be accepted and an empty array rejected. If the probe fails, it registers no rules and falls back to the content script (`maybeRenderPdfDoc`). — [pdfHandler.js](https://github.com/mozilla/pdf.js/blob/master/extensions/chromium/pdfHandler.js); background in [WECG #638](https://github.com/w3c/webextensions/issues/638#issuecomment-2181124486)
- `file://`: when "Allow access to file URLs" is off, DNR does not see the request. pdf.js listens to `webNavigation.onBeforeNavigate` (filter `urlPrefix:"file://"`, `pathSuffix:".pdf"`/`.PDF`) and calls `tabs.update` to the viewer. The viewer then prompts the user to enable file access, and a message handler opens `chrome://extensions/?id=…`. — [pdfHandler.js](https://github.com/mozilla/pdf.js/blob/master/extensions/chromium/pdfHandler.js)
- `<embed>`/`<object>` PDFs: `contentscript.js` detects them with a CSS animation trick (`pdfjs-detected-object-or-embed`) and swaps in a full-size iframe pointing at the viewer. It notes "Starting from Chrome 76, internal embeds do not have the original URL". — [pdf.js contentscript.js](https://github.com/mozilla/pdf.js/blob/master/extensions/chromium/contentscript.js)
- Chrome DNR semantics: rules with response-header conditions are evaluated after headers arrive. "If a request made it to this stage, the request has already been sent to the server". A redirect issues a new request. `extensionPath` must start with `/` and be web accessible: "A declarativeNetRequest rule cannot redirect from a public resource request to a resource that is not web accessible." Limits: 30,000 "safe" dynamic rules (Chrome 121+), 5,000 unsafe dynamic or session rules, 1,000 regex rules. — [Chrome DNR reference](https://developer.chrome.com/docs/extensions/reference/api/declarativeNetRequest)

### Cited Findings — per-browser API support (BCD)
- `RuleCondition.responseHeaders` / `excludedResponseHeaders`: **Chrome 128**, with the note "From Chrome 121 until Chrome 127, this condition was recognized but not applied". **Firefox: false. Safari: false.** `excludedRequestMethods`: Chrome 91, Firefox 113, Safari 26. `declarativeNetRequest.Redirect`: Chrome 84, Firefox 113, Safari 15.4. — [BCD declarativeNetRequest](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/declarativeNetRequest.json)
- Firefox tracking bug "[DNR] Rule condition to match on response headers" is still **NEW** with no target milestone (checked 2026-11-09). — [Bugzilla 1877486](https://bugzilla.mozilla.org/show_bug.cgi?id=1877486). The WebKit equivalent is also **NEW**, last changed 2024-08-06. — [WebKit 275158](https://bugs.webkit.org/show_bug.cgi?id=275158)
- `webRequest.BlockingResponse`: Chrome 17 (in MV3, blocking is limited to policy-installed extensions), Firefox 45, **Safari: false**. `webRequest.filterResponseData` is Firefox only (57). Safari `onHeadersReceived` (18.4) is observational only. — [BCD webRequest](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/webRequest.json). For Chrome MV3, blocking webRequest is limited to policy-installed extensions, per the [PDF-magic/opener PR #475](https://github.com/PDF-magic/opener/pull/475) research summary (secondary source).
- `webNavigation.onBeforeNavigate`: Chrome 16, Firefox 45, Safari 14. — [BCD webNavigation](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/webNavigation.json)
- Firefox: you can redirect from `onHeadersReceived` (fixed via [Bugzilla 1176092](https://bugzilla.mozilla.org/show_bug.cgi?id=1176092); see [Bugzilla 1224581](https://bugzilla.mozilla.org/show_bug.cgi?id=1224581)). Developers report redirecting PDF URLs to `moz-extension://…/viewer.html?url=…`, at the cost of breaking "Save page as". — [Mozilla Discourse](https://discourse.mozilla.org/t/developing-a-add-on-extension-to-view-pdfs/55619) (older thread)
- Safari DNR problems:
  - A redirect to a `safari-web-extension://` URL via `url` fails with "Redirection to URL with a scheme that is not HTTP(S)". The workaround is `extensionPath`, which rules out `regexSubstitution`, so the original URL cannot be passed in the query. Still open, labelled needing a Safari response. — [WECG #381](https://github.com/w3c/webextensions/issues/381)
  - "Disabling Safari content blockers also disables Safari web extension DeclarativeNetRequest". Status NEW (filed 2025-12-08). — [WebKit 303800](https://bugs.webkit.org/show_bug.cgi?id=303800)
  - Safari 26.0 redirect regression: the page reloads and then fails. — [Apple forums 803502](https://developer.apple.com/forums/thread/803502)
  - Redirect needs `declarativeNetRequestWithHostAccess`. — [Apple forums 721258](https://developer.apple.com/forums/thread/721258)
- Adobe Acrobat for Chrome: the user opts in with a "Set as default" prompt the first time a PDF opens. Admins force-install it by ID `efaidnbmnnnibpcajpcglclefindmkaj`. — [Adobe help](https://helpx.adobe.com/acrobat/kb/acrobat-reader-chrome-extension.html), [Adobe community](https://community.adobe.com/t5/acrobat-reader-discussions/make-adobe-plugin-for-chrome-default/m-p/14334241)

### Inferences
- **Content-Disposition: attachment.** Copy pdf.js: override it in main_frame, respect it in sub_frame, and add a `?lamina.action=download`-style escape hatch.
- **POST-originated PDFs.** Exclude them (Chrome DNR `excludedRequestMethods`, Firefox check `details.method`). The viewer can only re-GET, and the POST body is gone. Let the native viewer handle these.
- **Auth/cookies.** The viewer re-fetches the URL from the extension origin. In Chromium this is a cross-site request from `chrome-extension://`, so `SameSite=Lax/Strict` cookies may not be sent. pdf.js ships a `preserve-referer.js` (referenced via `canRequestBody` in pdfHandler.js) to keep the Referer header. This is an inference: verify with tests against cookie-gated PDFs. Firefox has an advantage here: `filterResponseData` could stream the original response bytes to the viewer and avoid the re-fetch entirely.
- **Firefox vs built-in pdf.js.** A blocking `onHeadersReceived` redirect happens before the response body reaches pdf.js's stream converter, so the extension wins without the user setting `pdfjs.disabled`. This is inference from how the redirect works, and I found no primary doc stating the order. WebExtensions have no API to set `pdfjs.disabled` (inference; the `browserSettings` API does not list it). Coexistence is the default: PDFs the extension does not redirect (POST, opted-out, iframes) still open in Firefox's pdf.js.
- **Safari.** Realistically, the extension can catch only URLs that *look* like PDFs (`.pdf` path) via `webNavigation.onBeforeNavigate` + `tabs.update` (pdf.js's `file://` pattern), or a DNR `extensionPath` redirect to a fixed viewer path that reads the URL some other way. PDFs identified only by Content-Type will open in Safari's native PDFKit viewer. A toolbar action or context menu ("Open in Lamina") on a native PDF tab is the reliable Safari entry point.

### Gaps
- I found no source code or official write-up for Kami's or Xodo's interception mechanism. Adobe's approach is documented only at the user level.
- Exact Chromium behaviour when an MV3 extension redirects a main_frame PDF with SameSite cookies was not tested or found.
- I found no Apple documentation stating whether Safari web extensions can intercept PDFs rendered by PDFKit. Apple's compatibility page did not render when fetched.

## 3. Firefox built-in pdf.js and Safari PDF handling

### Takeaway
In Firefox, an extension can coexist with and pre-empt pdf.js through blocking webRequest. Users do not need `pdfjs.disabled`. Firefox's own pdf.js pages are privileged, so extensions cannot script them. In Safari, PDF interception is partial at best (see §2).

### Cited Findings
- When pdf.js is built into Firefox, "the viewer is treated as part of the browser… a privileged page that extensions cannot modify". Since Firefox 60, add-ons cannot access PDFs rendered by pdf.js. — [Bugzilla 1454760](https://bugzilla.mozilla.org/show_bug.cgi?id=1454760)
- Firefox keeps blocking webRequest in MV3. — [Mozilla Add-ons blog](https://blog.mozilla.org/addons/2024/03/13/manifest-v3-manifest-v2-march-2024-update/)
- Safari `webRequest.BlockingResponse`: false. Safari DNR `responseHeaders`: false. — [BCD webRequest](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/webRequest.json), [BCD DNR](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/declarativeNetRequest.json)

### Inferences
- Content scripts cannot reach PDFs already showing in Firefox's pdf.js, so the redirect must happen at the network layer. Redirecting at the network layer is the only workable design in Firefox.

### Gaps
- I did not verify end to end that `onHeadersReceived` → `redirectUrl: moz-extension://…` beats pdf.js on Firefox 14x. A quick prototype test is recommended.

## 4. WASM and Web Workers in extension pages under MV3 CSP

### Takeaway
PDFium/hayro WASM in a worker inside an extension page works in all three engines if CSP includes `'wasm-unsafe-eval'`. Load scripts only from the package (`'self'`), with no CDN. SharedArrayBuffer and threaded WASM are possible in Chrome via the COOP/COEP manifest keys, **not in Firefox** (still open in 2026), and unconfirmed in Safari. Keep Lamina's single-threaded WASM build as the extension default.

### Cited Findings
- MV3 CSP allows only `'none'`, `'self'`, and `'wasm-unsafe-eval'` in script directives (plus localhost for development). Firefox MV2 can use WASM without the keyword, "this behavior isn't guaranteed" ([Bugzilla 1770909](https://bugzil.la/1770909)). — [MDN content_security_policy](https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/content_security_policy)
- `content_security_policy.extension_pages`: Chrome 88, Firefox 72, Safari 15.4. `sandbox` CSP: Chrome 88, Firefox 154, Safari: no. — [BCD content_security_policy](https://github.com/mdn/browser-compat-data/blob/main/webextensions/manifest/content_security_policy.json)
- pdf.js ships `script-src 'self' 'wasm-unsafe-eval'` in its MV3 Chromium build. — [pdf.js manifest.json](https://github.com/mozilla/pdf.js/blob/master/extensions/chromium/manifest.json)
- Chrome: extensions opt into cross-origin isolation with the `cross_origin_embedder_policy` / `cross_origin_opener_policy` manifest keys (M93). These apply to the background, popups, options page and extension tabs, enabling SharedArrayBuffer. — [Chrome cross-origin isolation](https://developer.chrome.com/docs/extensions/develop/concepts/cross-origin-isolation)
- Firefox: "Enable crossOriginIsolated and use of SharedArrayBuffer for Extension pages" is **REOPENED**, last changed 2026-07-16. The keys only produce an "unexpected property" warning. — [Bugzilla 1673477](https://bugzilla.mozilla.org/show_bug.cgi?id=1673477)
- Module background workers (`background.type: "module"`): Chrome 92, Firefox 112, Safari 16.4. — [BCD background](https://github.com/mdn/browser-compat-data/blob/main/webextensions/manifest/background.json)

### Inferences
- Running dedicated Workers (including module workers) from an extension page should behave like the web, with the worker script served from the extension origin. Vite's worker output must end up as files in the package, not blob: or data: URLs, which the MV3 CSP would block.
- Package size: Chrome Web Store and AMO accept packages of tens of MB, but I found no authoritative per-store limit in this pass. Bundling both PDFium (~several MB) and hayro WASM is plausible but worth trimming.

### Gaps
- Safari's support for COOP/COEP and SharedArrayBuffer in extension pages: no source found.
- Store package size limits: not verified this pass.

## 5. Cross-origin PDF fetches from extension pages (CORS, Range)

### Takeaway
With `host_permissions`, an extension page or service worker can fetch cross-origin PDFs regardless of CORS. This removes the web app's main limitation for remote URLs, and Range requests should work the same way. Content scripts do *not* get this exemption.

### Cited Findings
- Extension code "can talk to remote servers outside of its origin, as long as the extension requests host permissions". "Cross-origin requests are always treated as such in content scripts, even if the extension has host permissions." Do not let content scripts make the background fetch arbitrary URLs. — [Chrome: cross-origin network requests](https://developer.chrome.com/docs/extensions/develop/concepts/network-requests)
- pdf.js requests `host_permissions: ["<all_urls>"]` for this purpose. — [pdf.js manifest.json](https://github.com/mozilla/pdf.js/blob/master/extensions/chromium/manifest.json)

### Inferences
- Lamina's Range-request loader can run unchanged from the extension viewer page: no CORS preflight barrier, and `Range`/`Content-Range` readable. In Firefox MV3, host permissions are user-granted and can be denied, so the viewer needs a fallback (full GET, or a permission request prompt). This is inference: Firefox MV3 host permissions are opt-in, but I did not fetch a source for that this pass.
- `<all_urls>` triggers a stricter store review and a scary install warning. Optional host permissions requested on first use are a mitigation.

### Gaps
- Firefox and Safari specifics of extension-page CORS exemption were not separately sourced.

## 6. Other entry points

### Takeaway
A context menu and a toolbar action work everywhere. Side panels split three ways: Chrome `sidePanel`, Firefox `sidebar_action`, and none in Safari. The omnibox is unavailable in Safari. These are the only reliable "Open in Lamina" paths in Safari.

### Cited Findings
- `sidePanel`: Chrome 114+ (`open()` from 116). Firefox and Safari: no. `sidebarAction`: Firefox 54+ only. — [BCD sidePanel](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/sidePanel.json), [BCD sidebarAction](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/sidebarAction.json)
- `menus`/`contextMenus`: Chrome 6, Firefox 55, Safari 14. `omnibox`: Chrome 9, Firefox 52, Safari: no. — [BCD menus](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/menus.json), [BCD omnibox](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/omnibox.json)
- `extension.isAllowedFileSchemeAccess`: Chrome 12, Firefox 153 (reflects "Access local files on your computer"), Safari partial ("Always returns `false`"). — [BCD extension](https://github.com/mdn/browser-compat-data/blob/main/webextensions/api/extension.json)

### Inferences
- A context menu on links (`targetUrlPatterns: ["*://*/*.pdf*"]`) plus a toolbar action ("reopen current tab's PDF in Lamina") gives parity across all browsers, Safari included.
- For local files, an "Open file…" button in the extension viewer (File System Access API or `<input type=file>`) avoids `file://` permission problems entirely. Chromium's manifest `file_handlers` for extensions is ChromeOS-only (my recollection, not verified this pass).

### Gaps
- Extension `file_handlers` support was not verified.

## 7. Lighter alternative: redirect to https://app.laminapdf.com/#url=…

### Takeaway
A redirect-only extension is smaller and easier to review. It loses the two biggest wins: the CORS exemption and offline use. It also routes every PDF URL the user opens to a web origin, which weakens the privacy story even though no bytes upload. Bundling the reader is the stronger product. The redirect variant is a fallback, for example on Safari.

### Cited Findings
- Only extension contexts with host permissions bypass cross-origin restrictions. A web page at app.laminapdf.com does not get that exemption. — [Chrome: cross-origin network requests](https://developer.chrome.com/docs/extensions/develop/concepts/network-requests)
- Redirecting to an http(s) URL avoids Safari's "scheme that is not HTTP(S)" DNR limitation and allows `regexSubstitution` to carry the original URL. — [WECG #381](https://github.com/w3c/webextensions/issues/381)

### Inferences
- **Pros:**
  - No WASM or CSP work in the extension.
  - Reader updates ship without store review.
  - Tiny package.
  - In Safari, the redirect can carry the URL via `regexSubstitution` (http(s) target).
- **Cons:**
  - Most remote PDFs fail, because the servers do not send CORS headers. A hybrid fix is possible: the extension fetches the bytes and hands them to the web app via `postMessage`/`externally_connectable`, which partly reintroduces the complexity.
  - Cookie-gated PDFs fail outright.
  - No offline use.
  - The visited PDF URL appears in app.laminapdf.com's request logs only if it is put in the query. The fragment (`#url=`) is not sent to the server, which partially preserves privacy.
  - `file://` PDFs cannot be passed to a web origin at all.

### Gaps
- No store-review guidance specific to "redirector" extensions was found.
