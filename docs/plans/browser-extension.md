# Plan: the reader as a browser extension (Round K)

- **Status:** Draft
- **Date:** 2026-11-09
- **Research:** [browser extension research](../research/browser-extension/browser-extension.md) and its four notes
- **Todos:** Round K on the [todo board](../todos/README.md); §5 lists them
- **Builds on:** the [web reader](web-app.md) (Round C), whose `apps/web` the extension packages, and the [Paper design](paper-design.md), which draws every screen before it is built
- **Priority:** nice to have. Round K starts only after the web reader's v0.1 launch ([C7c](../todos/done/C7c-performance-gates-and-launch.md)), and nothing else waits on it.

## 1. Goal

The web reader can open a remote PDF only when the site that serves it sends CORS headers, and most sites don't. An extension removes that limit. Its own pages may fetch from any site the user lets it read, so the same reader opens the PDFs people actually click ([research §1](../research/browser-extension/browser-extension.md#the-extension-fixes-the-web-readers-cors-wall-but-each-engine-intercepts-pdfs-differently)). Round K packages the whole reader, WASM included, into one extension for Chrome, Edge, Firefox and Safari. When the user asks, it opens every PDF instead of the browser's own viewer. The privacy promise stays intact: the extension declares that it collects no data and uploads nothing, and anyone can check its build.

## 2. What the research found

- **The reader moves in with little change.** Every engine runs WASM in an extension page once the content security policy allows `'wasm-unsafe-eval'`. The worker and the WASM must ship as files, and Firefox gives extensions no `SharedArrayBuffer`, so the extension uses the single-threaded build.
- **Each engine takes PDFs over in its own way.** Taking a PDF over means opening it in Lamina PDF instead of the browser's viewer; the user turns it on with a setting called automatic opening. Chromium 128+ redirects any response served as `application/pdf` with declarativeNetRequest rules, as the pdf.js extension does. Firefox does it with blocking `webRequest`, which it keeps in Manifest V3. Safari can catch only URLs that end in `.pdf`, so a toolbar button and a context menu carry it.
- **Chromium always fetches the file twice.** The redirect fires after the browser has sent the first request, so one-time links and some cookie-gated files fail. A one-click "open in the browser's viewer" fallback is therefore a core feature.
- **Install with no warnings, then ask.** Access to all sites is an optional permission, requested when the user turns on automatic opening. Chrome turns an extension off when an update adds a permission that shows a warning, so the first release fixes the permission set.
- **Ship with WXT.** It builds on Vite, writes each browser's manifest, and publishes to the Chrome Web Store, Edge Add-ons and AMO from CI. Chrome 148+ has `browser.*`, and Mozilla archived `webextension-polyfill` in July 2026.
- **AMO review is the slow part.** AMO's reviewers rebuild the extension from source on ARM64 Linux and expect the same bytes, which is hard for the PDFium WASM build.
- **Room to stand out, demand unproven.** Adobe's extension has 351M Chrome users and documented bugs. The PDF.js extension has 1M users and no update since October 2024. Newer local-only entrants have almost no users.

## 3. Rules for this round

- **The extension is the web reader.** It imports `apps/web`'s reader and `@laminapdf/ui`; it never forks them. Extension-only code covers taking PDFs over, permissions, onboarding and the fallback.
- **Nothing remote runs.** All code ships in the package. The viewer page fetches only the PDF the user opened, and the network indicator ([U4a](../todos/done/U4a-network-indicator-and-privacy-proof.md)) shows every request, as it does on the web.
- **No warnings at install.** The manifest asks only for permissions without an install warning. Access to all sites is optional, requested from a screen that says why. Local files open through the reader's file picker and drag and drop; `file://` takeover would cost a "Read your browsing history" warning, so the extension leaves it out.
- **The browser's viewer is always one click away.** Every takeover offers it, and a failed fetch falls back to it.
- **One build, checkable by anyone.** The package is reproducible from a tag, its hashes are published with each release, and Chrome uploads are signed with Lamina's own key.

## 4. What Round K doesn't do

- **Editing in the extension.** The extension ships the reader. When the [editor](full-editor.md) arrives, the extension can carry it in a later round.
- **A redirect-only extension** that sends PDFs to `app.laminapdf.com`. It keeps the CORS limit ([research §1](../research/browser-extension/browser-extension.md#the-extension-fixes-the-web-readers-cors-wall-but-each-engine-intercepts-pdfs-differently)).
- **Replacing PDFs embedded in pages** (`<embed>`, `<object>`), side panels, and an enterprise settings schema. §10 lists them for later.
- **Mobile browsers other than Safari.** Chromium on Android installs no Chrome Web Store extensions, and Firefox for Android waits for demand on desktop (§10).
- **Telemetry.** The extension sends nothing. Opt-in telemetry follows [U5c](../todos/ready/U5c-opt-in-usage-telemetry.md) if it ever comes.

## 5. Todos

### K1. Prove it, decide it, draw it

| Todo | Delivers |
| --- | --- |
| [K1a](../todos/ready/K1a-browser-extension-spike.md) | A spike that answers the research's open questions: the reader under the extension CSP, Chromium and Firefox takeover, cookie-gated and one-time links, package size, and whether the WASM builds byte for byte on ARM64 |
| [K1b](../todos/ready/K1b-adr-0033-browser-extension.md) | ADR 0033: bundle the reader, Manifest V3, WXT, the permission model, the privacy rules and the release order |
| [K1c](../todos/ready/K1c-extension-screens-in-paper.md) | The extension's screens drawn in Paper and described in an area document: toolbar popup, onboarding, settings, fallback notice |

### K2. Chromium: Chrome, Edge, Brave, Vivaldi, Opera

| Todo | Delivers |
| --- | --- |
| [K2a](../todos/ready/K2a-extension-app-and-manual-open.md) | `apps/extension`: the reader in a WXT extension, opened from the toolbar button, the context menu and a file picker, with browser tests |
| [K2b](../todos/ready/K2b-automatic-pdf-takeover-chromium.md) | Automatic opening in Chromium: pdf.js-style rules, the Chrome 128 check, optional access to all sites from onboarding, the fallback, `#page=` links |
| [K2c](../todos/ready/K2c-chromium-release.md) | The release lane and the Chrome Web Store and Edge Add-ons listings: `wxt submit`, signed uploads, privacy declarations, published hashes |

### K3. Firefox

| Todo | Delivers |
| --- | --- |
| [K3a](../todos/ready/K3a-firefox-extension.md) | The Firefox build on AMO: takeover by blocking `webRequest`, reading the first response instead of fetching again, no-data declaration, the source package AMO rebuilds |

### K4. Safari

| Todo | Delivers |
| --- | --- |
| [K4a](../todos/ready/K4a-safari-extension.md) | Safari on macOS, then iOS and iPadOS: the app wrapper built on a macOS runner, button and context menu first, `.pdf` links where they work, App Store release |

## 6. Sequence

```
 C7c (web reader v0.1) ─► K1a ─► K1b ─┬─► K1c ─┐
                                      └────────┴─► K2a ─► K2b ─► K2c ─┬─► K3a
                                                                       └─► K4a  (also needs D6a: the Apple account)
```

- **After the web launch.** K1a waits for [C7c](../todos/done/C7c-performance-gates-and-launch.md), so the extension packages the reader that launched and never competes with Round C for time.
- **Nothing depends on Round K.** The roadmap's other rounds keep their order; Round K runs beside the desktop rounds (D) and the editor (E) as people are free.
- **Chromium first.** One Chrome Web Store listing also reaches Brave, Vivaldi and Opera on desktop, and Edge's store takes the same package.
- **Firefox and Safari in either order.** Both start after the Chromium release. K4a also needs the Apple Developer account that [D6a](../todos/ongoing/D6a-signing-runbook-and-owner-setup.md) sets up.

## 7. Changes to existing plans

- [Roadmap](roadmap.md): Round K in the sequence, the rounds table and a milestone, M2b.
- [Todo board](../todos/README.md): the `K` prefix.
- [Web reader](web-app.md): no change. Round K reuses the reader and asks nothing new of Round C.

## 8. Risks

- **Demand is unproven.** Local-only PDF extensions have few users. K2c ships to the Chromium stores before Firefox and Safari take any work, so their usage decides how far Round K goes.
- **AMO review can stall the Firefox release.** If PDFium's WASM won't rebuild byte for byte on ARM64, the Firefox build ships hayro alone. K1a measures this before anything is built.
- **Store rules change.** Chrome stopped running Manifest V2 extensions in Chrome 139 and removed the last ones from its store on 2026-08-31, and Mozilla changes its add-on policies on 2026-12-01. ADR 0033 records the rules the extension follows, and each release checks them.
- **A stolen store account could publish a malicious update**, as in the Cyberhaven attack of December 2024. Signed uploads, two-factor sign-in and the published hashes close that path.

## 9. Open questions

K1a answers questions 1 to 7 before ADR 0033 is written; questions 8 and 9 name the todo that answers them.

1. Does a Firefox `onHeadersReceived` redirect beat the built-in pdf.js viewer on current releases?
2. Do cookie-gated and one-time PDF links survive Chromium's second fetch from the extension, and does keeping the original `Referer`, as pdf.js does, help?
3. How large is the package with PDFium and hayro, and do the stores accept it?
4. Can PDFium's WASM build byte for byte on x86_64 and ARM64, or does Firefox ship hayro alone?
5. Which `minimum_chrome_version` do Brave, Opera and Vivaldi tolerate for `browser.*` and response-header rules?
6. Can a strict `connect-src` confine the viewer while it fetches PDFs from any site?
7. Does `use_dynamic_url` stop sites from detecting the extension through its viewer page?
8. Do enterprise force-installs grant optional host permissions? K2c checks it on a managed Chrome profile and records the result in `RELEASE.md`.
9. Is Safari's button-first flow useful enough to ship? K4a decides on a TestFlight build; the Apple account's fee is already paid for the desktop app (D6a).

## 10. Later

- Replace PDFs embedded in pages (`<embed>`, `<object>`) through a content script.
- A side panel in Chrome and Firefox for reading next to the page.
- Enterprise docs and a managed-storage schema, so administrators can install the extension with `ExtensionSettings` or `policies.json` and turn automatic opening on for everyone.
- Firefox for Android, if Firefox desktop shows demand.
- The editor in the extension, once the web editor ships.

## 11. To-dos

- [ ] Answer the open questions with a spike ([K1a](../todos/ready/K1a-browser-extension-spike.md))
- [ ] Decide the extension's shape in ADR 0033 ([K1b](../todos/ready/K1b-adr-0033-browser-extension.md))
- [ ] Draw the extension's screens in Paper ([K1c](../todos/ready/K1c-extension-screens-in-paper.md))
- [ ] Build the extension and open PDFs by hand ([K2a](../todos/ready/K2a-extension-app-and-manual-open.md))
- [ ] Open PDFs automatically in Chromium ([K2b](../todos/ready/K2b-automatic-pdf-takeover-chromium.md))
- [ ] Release to the Chrome Web Store and Edge Add-ons ([K2c](../todos/ready/K2c-chromium-release.md))
- [ ] Release on Firefox ([K3a](../todos/ready/K3a-firefox-extension.md))
- [ ] Release on Safari ([K4a](../todos/ready/K4a-safari-extension.md))
