# @laminapdf/web

The web reader: a Vite single-page app that puts `@laminapdf/ui`'s reader chrome around `@laminapdf/viewer`. It opens PDFs from your device inside the browser tab and sends nothing anywhere. It is served by one Cloudflare Worker with static assets and no code of its own, so the server can't receive a document even by mistake ([ADR 0011](../../docs/decisions/0011-cloudflare-topology-and-privacy.md)).

## Run it

The app imports the built viewer, which needs Lamina PDF's WebAssembly, so build that once first:

```sh
cargo xtask wasm                                   # the WebAssembly modules and @laminapdf/sdk
pnpm --filter @laminapdf/viewer run build:wasm     # the viewer and its render backends
pnpm --filter @laminapdf/tokens --filter @laminapdf/icons --filter @laminapdf/fonts --filter @laminapdf/ui run build
```

`cargo xtask web build` does all three and builds the app too, as the deploys do. It skips the WebAssembly modules' tests, which `cargo xtask wasm` runs.

Then:

```sh
pnpm --filter @laminapdf/web dev        # Vite's dev server, with hot reload
pnpm --filter @laminapdf/web build      # dist/: what deploys
pnpm --filter @laminapdf/web preview    # dist/ through `wrangler dev`, with the production headers, on port 8788
```

Open a PDF with **Choose a file**, by dropping it anywhere in the window, or by pasting it. Add `#debug` to the address for the debug menu, which switches the render backend between PDFium and hayro, and for the first page's render time in the status bar.

## How it fits together

- **`src/main.tsx`** loads the Paper tokens, fonts and component styles, preloads the first paint's font, and renders the app with one platform (`createWebPlatform()`) for the page.
- **`src/session.ts`** is the engine. As soon as the app loads, it starts the render backend's document worker, so a file chosen in the meantime waits only for what is left. It opens each file through the viewer, collects the file's warnings for the repair panel, and on a backend switch starts a new worker and reopens the file.
- **`src/app.tsx`** composes the chrome: the home screen's drop zone, the viewer, the failure page, the password dialog, the repaired notice and its details panel, and the debug menu.
- **`src/open/`** reads the address's fragment: `url-fragment.ts` parses and writes it, `navigation.ts` decides what a change of it asks for, and `watch.ts` acts on it when the page loads and on every `hashchange` and `popstate`. `history.ts` writes each of the viewer's jumps (a link, the outline, a thumbnail, go to page, find) as a browser history entry. Its fragment keeps `url` and names the page, so the browser's Back returns to the exact place without opening the file again.
- **`src/network/`** counts every request the page and its document worker make, for the status bar's network indicator and its panel. `page-feed.ts` is the first module `main.tsx` imports, so it starts counting before any other code runs. `classify.ts` decides which requests are document traffic, and `monitor.ts` keeps the log.
- **`src/document.ts`** holds the pure parts: the window title, the repair list and its copied text. `pnpm --filter @laminapdf/web test` runs their tests, `src/open/`'s and `src/network/`'s in Node.

## Opening a linked file

A link of the form `https://app.laminapdf.com/#url=<address>&page=<n>` opens the PDF at `<address>`, at page `<n>`. The address sits in the fragment, after `#`, which browsers never send to a server. The reader's document worker then fetches the file straight from its site with HTTP Range requests, so a large file shows its first page after only a small part has arrived.

The address must be encoded as a whole with `encodeURIComponent`: `#url=https%3A%2F%2Fexample.com%2Freport.pdf%3Fv%3D2&page=5` opens `https://example.com/report.pdf?v=2` at page 5. The reader refuses an address pasted without encoding, since its own `&page=` could then belong to the address. It also opens only `https://` addresses. When only `page` or `nameddest` changes, the reader moves within the open document; Back and Forward never fetch it again. Leaving the document drops `url` from the address without adding a history entry, so a reload shows the home screen.

`&nameddest=<name>` opens a named destination, a place the PDF's author named, such as `chapter-2`, instead of a page, as in pdf.js: `#url=…&nameddest=chapter-2`. `page` and `nameddest` also work on a file opened from your device.

The file's site must send it uncompressed and allow other sites to read it, with these response headers:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Headers: Range, If-Range
Access-Control-Expose-Headers: Content-Range, Content-Length, ETag
```

A site that sends no CORS headers gets a page that explains the refusal and links to the file, so you can download it and open it here: files go straight from their site to your browser, never through a server of ours. A site that ignores `Range` works too, since the reader then downloads the file whole, up to 256 MiB.

Every request for the file omits cookies and the referrer and skips the browser's HTTP cache. An encrypted file asks for its password, which goes to the document worker once and is never kept on the page. Because JavaScript strings can't be overwritten, the worker's copy stays in its memory until it is collected. After a worker crash, the reader asks for the password again.

## The network indicator

The status bar counts the network requests made for the open document: "0 network requests for this document" for a file opened from disk. Pressing it opens the network panel, which lists every request since the file opened. Document traffic is grouped by site. The reader's own files (its scripts, styles, fonts and WebAssembly engines, all from this site) are listed apart, since none of them is document traffic. A request to any site other than this one and the linked file's host turns the indicator to a warning and names the site.

The page and the document worker are both counted from the moment they start, in two ways. Code on the page and in the document worker calls `fetch`, `XMLHttpRequest`, `WebSocket` and `sendBeacon` through wrappers that report every call, failed or not. Everything else the browser loads (the page, scripts, styles, fonts, images, module imports) comes from its Resource Timing records. A request that the Content Security Policy blocks sent nothing: the panel lists it as blocked and doesn't count it.

The policy itself stays as ADR 0011 sets it, with `connect-src 'self' https:` so linked files can load. Narrowing it for local-only sessions wouldn't help: a `<meta>` policy can only add restrictions to the page, and a worker takes the policy of its own script's response, so the document worker would stay as it is. So the indicator, rather than the policy, shows whether an HTTPS request was made.

Browser extensions make requests no page can see, so the panel says so; your browser's developer tools show those too.

## Reader mode

Reader mode turns a PDF into one column of text that the reader can size. It opens from the Reader button in the top bar, the `R` key, or "Reader mode" in the command palette. Reader settings (in the inspector) set the text size from 14 to 32 px, the line spacing and the line length, for every document. The column takes the page's paper and ink.

For a tagged PDF, the column follows the file's structure tree, so headings, lists, quotes and notes come out as real HTML in the order the author tagged, whatever order the page draws them in. Figures, formulas and tables stay pictures: each is a crop of its page, drawn by the document worker, named by its alternate text or by "The file gives no description of it". A crop is drawn only when it comes within a screen of the view, and its pixels are freed once it scrolls far away, so a long scan doesn't hold every page at once. The text a crop holds, such as a table's cells or a page image's text, follows it for screen readers.

Many files have no tags, or tags that don't match their pages, so reader mode checks the tree before it trusts it. It turns to the page layout when:

- the file has no tags;
- the tree is empty;
- the tree's content sits mostly in bare spans;
- the tree covers too little of the visible text on more than a quarter of its pages;
- the pages' layout contradicts the tree's order.

The column then reads the page layout the way extraction does: columns in order, headings by size and weight, list items by their markers, running heads and feet left out. A badge says "Approximate", and a notice explains why and asks the reader to check figures and numbers against the page. Tables and pictures are crops, and a "Figure" or "Table" line next to one is its caption. Each block offers "View original page" on hover or focus, which leaves reader mode at the block and outlines it on the page for two seconds. A scanned page, or one whose column order extraction doubts, is shown as its picture with one sentence of reason and "Open page n", rather than as text in an order that may be wrong.

Reader mode keeps the reader's place both ways. It starts the column at the block at the top of the page view, and "Back to page n", Escape or the toggle return the page view to the block at the column's top. The page view stays laid out under the column, unseen and inert, so it can move there at once. Only a file with no text and no picture on any page shows "Reader mode isn't available for this document". The checks and the layout reading run in `laminapdf-extract` (`reflow`), and the reflow comes from the extract module only, which the reader loads.

## Printing, downloads and properties

The More menu holds Download a copy, Print… (also `Ctrl P`, or `⌘P` on a Mac; while a document is open, the shortcut opens the reader's print, not the browser's) and Document properties…. All three work on this device; nothing about the file goes to any server.

**Print** draws each chosen page in the document worker at 150 or 300 dpi, with comments and highlights as their print flags say. The worker encodes each page as a PNG, so the reader's own page holds only small image files, never pixels, one at a time. A page that would pass 16 million pixels at the chosen resolution is drawn at a lower one, and a High-quality print of more than a billion pixels in all prints at Standard, with a notice. Cancel stops between pages and frees what was drawn.

**The print frame** holds one image per page at its page's size in points. The frame is hidden, and its own `print()` opens the browser's print window, so the reader around it never prints. The frame inherits the reader's Content Security Policy, which allows no inline style. Each page's size therefore goes into a stylesheet that script builds and the frame adopts, as a named page (`@page p3 { size: 792pt 612pt }`). Where the browser honours named pages, a document that mixes portrait and landscape prints each page the right way up. Fit to paper sets those sizes and shrinks a page the paper can't hold; Actual size leaves the paper to the print window. When a file's author restricted printing, the dialog says so and asks once more before printing; see [R09](../../docs/design/paper/reader-a.md#r09-print-and-download-a-copy).

**Download a copy** saves the file's bytes exactly as opened: a local file's own bytes, or a linked file's from the document worker's cache and its site, unchanged. An encrypted file stays encrypted. The name comes from the file, or a linked file's address, made safe to save: no path, no characters Windows refuses, and `.pdf` at the end.

**Document properties** reads the file with Lamina PDF's own parser, not the render engine: the Info and XMP fields (XMP wins where they differ), the PDF version, pages, sizes, file size, tagging, the fonts the pages use, the encryption and the author's permissions, and the signatures present, which are counted and never verified.

When the document can be edited (an encrypted one can't), Document properties opens on a Description tab: a form for the document's title, authors, subject and keywords, its creation date, and the apps that made it. The other facts move to a File tab. Remove metadata… marks values for removal, and Keep takes one back. Apply makes the form's changes one step of the edit session, which Undo takes back, and each value is written to the Info dictionary and the XMP metadata together. Nothing reaches the file until Save, which also sets the modification date. Under a field, the dialog warns when its value would break the PDF/A or PDF/UA claim the file makes. A notice says when a save would keep a removed value in the file's earlier version: an incremental save adds the change after the original bytes, and only Save as optimised drops that earlier version.

## The capability page

More › What Lamina PDF can do opens `/capabilities` over the open document. For each operation of the feature inventory, the page shows what Lamina PDF reads, writes and round-trips in the core, the TypeScript library, the web reader and the desktop app, as its tests prove. Its data is `docs/capabilities/matrix.json`, which `cargo xtask capabilities` writes and the build bundles, so the page makes no request. Search, a group and the columns shown filter it, and the filters are kept in the browser. Back returns to the document as it was; the docs site links to the address directly.

## Page appearance

View settings › Page appearance draws the pages in Sepia, Dark or High contrast, apart from the UI theme. Images and annotation colours stay as they are. You can apply it to this document or to all documents; a choice for this document overrides the one for all. A toast confirms it, with Undo.

With no choice made, a system in forced colours (Windows contrast themes) gets High contrast, drawn in the system's own text and background colours.

The choices stay in this browser's `localStorage`. A local file's choice is kept under a hash of its name and size. A linked file's choice lasts only until the tab closes, because the reader stores nothing about linked files.

## Offline, installing and what stays in the browser

After one visit the reader works with no network. Its service worker (`src/sw/service-worker.ts`, compiled into `dist/sw.js` by `plugins/pwa.ts`) keeps the reader's own files: the page, its scripts, styles, fonts, icons and the WebAssembly it loads. It answers only requests to the reader's own site, so a file opened from a `#url=` link never passes through it and is never cached. Offline, it serves the page with the headers it was stored with, so the Content Security Policy and the other headers above still hold.

A new version installs in the background and waits: a toast offers Reload, and only Reload switches to it. While offline, a notice under the top bar says what still works.

**Installing.** `manifest.webmanifest` makes the reader installable as an app, with its own window and icon. Where the browser offers an install (Chromium), More › Install Lamina PDF… shows it; other browsers install sites from their own menus. An installed copy in Chromium opens PDFs from the operating system's Open with menu (the manifest's `file_handlers`).

**What the browser keeps.** The reader keeps three things, all in this browser and never sent anywhere:

- The Recent list on the home screen, in IndexedDB: each local file's name, size, page count and when it was opened. Chromium also keeps the file's handle, so pressing its row reopens it; other browsers ask for the file again. A file opened from a link is never listed, and its address is never stored (ADR 0011).
- Where you stopped reading in each document: its page, place on the page, zoom and rotation, in IndexedDB. Each position is kept under a fingerprint of the file, a hash of its first bytes, never under its name or address. So a renamed copy reopens where you left off, and so does the file after an app adds a note or a signature to its end. The 500 documents read most recently are kept. A link that names a page (`#page=3`) opens there instead.
- A copy of each local file larger than 200 MiB, in the origin-private file system (OPFS). The document worker reads these large files from the copy rather than holding them in memory. The copies together stay within half the space the browser leaves free, at most 2 GiB; the copies opened longest ago are deleted first.

Settings › Storage and privacy › Remember recent local files is on by default. Turning it off empties the list and deletes the copies, and a large file's copy is then deleted when its document closes.

Settings › Storage and privacy › Clear recent files and cached documents removes the list, every copy and every reading position.

Settings › Storage and privacy › Remember where I stopped reading is on by default. Turning it off forgets every position and keeps no new ones. Forget reading positions, beside it and in the command palette, forgets them and leaves the setting on.

**Coming back to a document.** The local file open when the reader last closed comes back at the next start, at its page, where the browser kept the permission to read it (Chromium, after "Allow on every visit"). Elsewhere the home screen lists it first, under Continue reading, with the page you stopped at; pressing it asks for the file and opens it there. A file from a link comes back with the browser's own tab restore, which keeps the link in the address. Safari deletes a site's storage after seven days without a visit unless the reader is installed, so positions can be lost there.

## Headers and hosting

The reader shows the website's icon. `vite.config.ts` copies `favicon.svg`, `favicon-32.png` and `apple-touch-icon.png` from `apps/site/public/` into the build, and serves them in development. `index.html` links them as the website and the docs site do, and `test/icons.test.ts` checks both.

`wrangler.jsonc` serves `dist/` as static assets, answers unknown paths with the app, and has no `main` script. `public/_headers` sets [ADR 0011 §4](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#4-security-headers-appswebpublic_headers-for-)'s headers on every response:

- **A strict Content Security Policy.** Scripts, styles, fonts, workers and WebAssembly come only from the app's own origin, and nothing inline runs. `'wasm-unsafe-eval'` is the one exception, because compiling WebAssembly needs it. React Aria would add one inline style for pressable elements, so `src/main.tsx` claims that style's ID and `src/app.css` carries its rule instead.
- **Caching.** Vite's files under `/assets/` have hashed names and are cached for a year. HTML keeps the static-assets default, `public, max-age=0, must-revalidate`, so a new release reaches you on the next load. No rule for every path sets `Cache-Control`: Cloudflare would join it with the assets rule's value into one comma-separated header.
- **One address per page.** `/index.html` redirects to `/`, as Cloudflare's assets server does for every `index.html`.
- **What search engines may index** (`plugins/search.ts`). Every build writes a `robots.txt` from the site kit's one policy, which lets crawlers in. A build without `LAMINA_PDF_ENV=production` (staging, the previews, a local build) also adds `X-Robots-Tag: noindex` to every response, so it is crawled but never indexed ([ADR 0011 §1](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#1-topology)). Only the build of a `web-v*` tag sets the variable (`web-deploy.yml`'s `production-checks`).

## Deploys

Each reader pull request gets a preview URL in a comment, each merge to `main` deploys https://staging-app.laminapdf.com, and each release (a `web-v*` tag) deploys https://app.laminapdf.com. For every one of these deploys, `.github/workflows/web-deploy.yml` builds the reader with `cargo xtask web build`, and a smoke test checks the deploy once it is live. [DEPLOY.md](DEPLOY.md) covers the setup, the rollbacks and deploying by hand.

## Releases

The reader releases on its own schedule, apart from the packages. Once a day, after the full nightly passes on `main`, the web release train (`.github/workflows/web-release-train.yml`) checks whether what the reader ships changed since its last release. If it did, the train tags `web-v<next>` on that commit. The patch number goes up by itself; for a new minor or major, raise `version` in `package.json` and the next release takes it. Settings › About shows the released version, and each release's notes (the pull requests that changed it) are on its GitHub release. [CHANGELOG.md](CHANGELOG.md) has 0.1.0's. `cargo xtask web next-release` makes the decision; [tools/release/README.md](../../tools/release/README.md#the-web-readers-train) covers the switch and releasing by hand. Before production deploys, the tag's `production-checks` job checks the tag against `package.json` (`cargo xtask web check-tag`) and runs the release gates:

- **Startup size:** `cargo xtask web size` checks the scripts the built `index.html` loads at startup, gzipped, against [`size-budget.json`](size-budget.json). The fast lane runs the same check on every pull request.
- **Lighthouse:** `pnpm --filter @laminapdf/web lighthouse:local` serves the build with its deployed headers and checks [`lighthouserc.json`](lighthouserc.json)'s scores.
- **Timings:** `cargo xtask viewer-perf --gate` takes the median of three runs of the viewer's harness and checks it against the web plan's targets plus 25 %. A miss blocks the release unless `packages/viewer/perf/waivers.json` names it with an issue.

## Releases

Each release is a `web-v<version>` tag on a commit of `main`, with its notes in [CHANGELOG.md](CHANGELOG.md). Settings › About shows the tag's version, and `package.json` holds the base version a tag's must match in major and minor. Before production deploys, the release lane's `production-checks` job runs the release gates:

- **Startup size:** `cargo xtask web size` checks the scripts the built `index.html` loads at startup, gzipped, against [`size-budget.json`](size-budget.json). The fast lane runs the same check on every pull request.
- **Lighthouse:** `pnpm --filter @laminapdf/web lighthouse:local` serves the build with its deployed headers and checks [`lighthouserc.json`](lighthouserc.json)'s scores.
- **Timings:** `cargo xtask viewer-perf --gate` takes the median of three runs of the viewer's harness and checks it against the web plan's targets plus 25 %. A miss blocks the release unless `packages/viewer/perf/waivers.json` names it with an issue.

## Tests

`e2e/` runs the app in a real browser against `wrangler dev`, the way production serves it. It first checks the headers and that the build has no inline code. It then opens a local file four ways: the file input, a drop, a paste and `showOpenFilePicker` (Chromium only). It also opens a repaired file and refuses a file that isn't a PDF. Throughout, it fails on any CSP violation and on any request to another origin.

Three more suites use `e2e/remote-host.ts`, a local HTTPS server on another port that plays the file's site. It serves each file with CORS and ranges, with CORS but no ranges, or with neither, and logs every request it receives:

- **`remote.e2e.test.ts`** opens linked files. Page 1 of a 50 MB file appears after the reader reads less than a quarter of it. A site without CORS gets the explanation, and a change of `page` alone never opens the file again. It also checks the link format: an unencoded address and `http://` are both refused.
- **`encrypted.e2e.test.ts`** opens every encrypted fixture, each revision of the standard security handler from 2 to 6. A file with an empty user password opens at once. Any other asks for its password, rejects a wrong one, and opens with the user or the owner password.
- **`privacy.e2e.test.ts`** checks what leaves the browser while a local file, a linked file and an encrypted file of each kind open. It records every request of the page and its worker, and fails when a request body carries 32 bytes of the document or when a request to the reader's origin names the linked address. It also fails on a request to the reader's origin that isn't a GET and on a Referer or cookie reaching the file's site. Any cookie at all, any other origin and the password in any request fail it too.

`e2e/output.e2e.test.ts` prints a 50-page file with landscape pages among portrait ones. It replaces the print frame's `print()` before it runs, then checks that all 50 images loaded at their pages' sizes, with a named `@page` rule each. Throughout, it checks that neither the page nor the frame met a CSP violation and that nothing was fetched. It prints a range by number and by page label, refuses a page the file lacks, and stops on Cancel. For a file whose author restricted printing, the reader asks once more, with Cancel focused, before it prints. Its downloads of a local file, a linked file and an encrypted file must match the originals' SHA-256. Its properties must show what pypdf reads in `fixtures/corpus/expected.json`, close with Escape and pass axe.

`e2e/properties.e2e.test.ts` edits each field of Document properties, applies, saves (a download), and reads the file back with Lamina PDF's SDK and with pdf.js: the Info dictionary and the XMP metadata must both hold the new values. It also sets a creation date in a file that had no XMP and removes values with Remove metadata…. Then it checks the warning under the title of a PDF/UA file left without one, and opens the capability page over a document and from its address. Its tests tagged `[cap J.01 web round-trip]` and the like prove the web column of the [capability matrix](../../docs/capabilities/README.md).

Playwright's WebKit reports each native `<select>` as an inline style under the reader's policy, even on a page holding only a bare `<select>`. The suites' CSP check (`e2e/reader.ts`) leaves out, in WebKit only, the reports raised where a script creates a `<select>`.

`e2e/page-appearance.e2e.test.ts` chooses Dark from View settings. The paper must turn dark while a highlight stays yellow, Undo must bring the colours back, and the choice must come back with the same file but not with another. In Chromium, emulated forced colours must start the reader in High contrast.

`e2e/offline.e2e.test.ts` loads the reader once, cuts the network and reloads. The page must come from the service worker with exactly the server's headers, an inline script must stay blocked, and a local file must open, render and be searchable. 

`e2e/installability.e2e.test.ts` validates the manifest against SchemaStore's schemas (pinned in `e2e/fixtures/`), fetches every icon it names, and asks Chromium whether anything stops an install. It also logs whether the browser offered an install, without failing on it.

`e2e/recent-files.e2e.test.ts` lists and reopens a file, checks that a linked file leaves nothing in IndexedDB, and checks that Clear empties both stores.

`e2e/reading-position.e2e.test.ts` builds a 40-page file, reads it to page 12 at 150 % and opens it again: it must come back there, announced, with Back to page 1. A renamed copy and the file after an incremental save must come back there too, while a different file with the same name and size must not. A link must come back by its fingerprint, with its address never stored, and `&page=3` must win. The store may hold nothing but views, page counts and times under fingerprints, and Forget reading positions and the setting must empty it.

Every suite's browser accepts the e2e server's self-signed certificate for service workers (`e2e/browser.ts`), so the other suites run with the worker active too.

`e2e/network.e2e.test.ts` holds the network indicator to Playwright's own record of the browser's requests, which includes the document worker's. On each flow, the panel's counts of document traffic and of app files must equal that record, split by the same `classifyRequest`. The flows are a local file (0 document requests) and a linked file (its host only). The suite then makes requests on purpose:

- a raw `fetch` from the worker and an HTTPS request from the page to another site, which are counted and flagged;
- an image and a plain-HTTP request from the page, and a `data:` fetch from the worker, which the policy blocks and the panel lists as blocked.

It also opens the panel from the keyboard and runs axe.

`e2e/text.e2e.test.ts` checks the text layer in the built reader. Copying page 1 of the five-script reference PDF gives the lines in `fixtures/text/page-text.json`. The same holds for its AES-256 copy once the test types its password. A tagged file's first heading owns the spans of its text, and an untagged file says once that it is untagged.

`e2e/find.e2e.test.ts` checks find. Ctrl+F or ⌘F opens the find bar with the focus in its field, and typing shows the first highlight and the count. Enter and Shift+Enter move between matches across pages, and Match case and Whole words change the count. Escape closes the bar, clears the highlights and gives the focus back. Find also works in an encrypted file once its password is typed, and in a linked file read by range requests.

`e2e/devices.e2e.test.ts` runs the reader in Playwright's phone and tablet profiles. Every pull request runs Pixel 8 in Chromium, at its own width and narrowed to 320 px. The nightly's WebKit run adds iPhone 15 and iPad Pro 11. Firefox has no mobile emulation. Each opens a file and checks the layout: the bottom bar on the phones, the desktop shell on the tablet, nothing past the window's edge, and 44 px targets in both bars. It zooms with a double tap and pans with one finger. A pinch must keep the point under the fingers within 2 px. Find works from the phone's Search button and from the tablet's keyboard. axe must report no serious or critical violations.

```sh
pnpm --filter @laminapdf/web build
LAMINA_PDF_E2E_BROWSER=firefox pnpm --filter @laminapdf/web e2e   # chromium (default), firefox or webkit
```

`cargo xtask pdf-runtimes chromium|firefox|webkit` typechecks the app, builds it and runs the e2e suite in that browser, after the packages' own browser tests. The app's typecheck runs there rather than in `cargo xtask check ts` because it needs the built viewer.

`e2e/smoke.test.ts` checks a deployed reader instead, in Chromium: the same headers, and `hello.pdf` opened from the file input. The deploy workflow runs it after each deploy, and it runs against any address:

```sh
LAMINA_PDF_SMOKE_URL=https://staging-app.laminapdf.com pnpm --filter @laminapdf/web run smoke
```
