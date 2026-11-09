# Browser extension research

Whether Lamina PDF should ship a browser extension for its reader, and how to build one that works in Chrome, Edge, Firefox, Safari and the Chromium forks. Researched on 2026-11-09; the [browser extension plan](../../plans/browser-extension.md) turns it into work.

| Note | Summary |
| --- | --- |
| [browser-extension.md](browser-extension.md) | The report. Recommends bundling the whole reader in the extension, because only an extension page can fetch a PDF from a site that sends no CORS headers. Chromium takes PDFs over by response header, Firefox by blocking `webRequest`, and Safari only by button, context menu or `.pdf` URL. Ends with a phased plan and the questions a prototype must settle. |
| [notes/platform-and-pdf-interception.md](notes/platform-and-pdf-interception.md) | Manifest V3 in each browser, how the pdf.js extension takes PDFs over, WASM and workers under the extension CSP, fetching across origins, and the other ways in: context menu, toolbar button, side panel. |
| [notes/distribution-and-tooling.md](notes/distribution-and-tooling.md) | The stores: fees, review, source-code rules (AMO rebuilds the package and compares bytes), Safari's app wrapper, build tools (WXT recommended), publishing from CI, and testing. |
| [notes/permissions-privacy-enterprise.md](notes/permissions-privacy-enterprise.md) | Install warnings per permission, optional host access, the stores' privacy declarations, supply-chain attacks such as Cyberhaven's, and deploying the extension through enterprise policies. |
| [notes/competitors-and-demand.md](notes/competitors-and-demand.md) | Adobe Acrobat, the PDF.js extension, Kami and smaller entrants: users, ratings, business models and complaints, plus what the browsers' built-in viewers still lack. |
