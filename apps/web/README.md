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
- **`src/open/`** reads the address's fragment: `url-fragment.ts` parses and writes it, `navigation.ts` decides what a change of it asks for, and `watch.ts` acts on it when the page loads and on every `hashchange` and `popstate`.
- **`src/document.ts`** holds the pure parts: the window title, the repair list and its copied text. `pnpm --filter @laminapdf/web test` runs their tests and `src/open/`'s in Node.

## Opening a linked file

A link of the form `https://app.laminapdf.com/#url=<address>&page=<n>` opens the PDF at `<address>`, at page `<n>`. The address sits in the fragment, after `#`, which browsers never send to a server. The reader's document worker then fetches the file straight from its site with HTTP Range requests, so a large file shows its first page after only a small part has arrived.

The address must be encoded as a whole with `encodeURIComponent`: `#url=https%3A%2F%2Fexample.com%2Freport.pdf%3Fv%3D2&page=5` opens `https://example.com/report.pdf?v=2` at page 5. The reader refuses an address pasted without encoding, since its own `&page=` could then belong to the address. It also opens only `https://` addresses. When only `page` changes, the reader moves within the open document; Back and Forward never fetch it again. Leaving the document drops `url` from the address without adding a history entry, so a reload shows the home screen.

The file's site must send it uncompressed and allow other sites to read it, with these response headers:

```
Access-Control-Allow-Origin: *
Access-Control-Allow-Headers: Range, If-Range
Access-Control-Expose-Headers: Content-Range, Content-Length, ETag
```

A site that sends no CORS headers gets a page that explains the refusal and links to the file, so you can download it and open it here: files go straight from their site to your browser, never through a server of ours. A site that ignores `Range` works too, since the reader then downloads the file whole, up to 256 MiB.

Every request for the file omits cookies and the referrer and skips the browser's HTTP cache. An encrypted file asks for its password, which goes to the document worker once and is never kept on the page. Because JavaScript strings can't be overwritten, the worker's copy stays in its memory until it is collected. After a worker crash, the reader asks for the password again.

## Headers and hosting

`wrangler.jsonc` serves `dist/` as static assets, answers unknown paths with the app, and has no `main` script. `public/_headers` sets [ADR 0011 §4](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#4-security-headers-appswebpublic_headers-for-)'s headers on every response:

- **A strict Content Security Policy.** Scripts, styles, fonts, workers and WebAssembly come only from the app's own origin, and nothing inline runs. `'wasm-unsafe-eval'` is the one exception, because compiling WebAssembly needs it. React Aria would add one inline style for pressable elements, so `src/main.tsx` claims that style's ID and `src/app.css` carries its rule instead.
- **Caching.** Vite's files under `/assets/` have hashed names and are cached for a year. HTML keeps the static-assets default, `public, max-age=0, must-revalidate`, so a new release reaches you on the next load. No rule for every path sets `Cache-Control`: Cloudflare would join it with the assets rule's value into one comma-separated header.
- **One address per page.** `/index.html` redirects to `/`, as Cloudflare's assets server does for every `index.html`.

## Deploys

Each reader pull request gets a preview URL in a comment, each merge to `main` deploys https://staging-app.laminapdf.com, and a `web-v*` tag deploys https://app.laminapdf.com. For every one of these deploys, `.github/workflows/web-deploy.yml` builds the reader with `cargo xtask web build`, and a smoke test checks the deploy once it is live. [DEPLOY.md](DEPLOY.md) covers the setup, the rollbacks and deploying by hand.

## Tests

`e2e/` runs the app in a real browser against `wrangler dev`, the way production serves it. It first checks the headers and that the build has no inline code. It then opens a local file four ways: the file input, a drop, a paste and `showOpenFilePicker` (Chromium only). It also opens a repaired file and refuses a file that isn't a PDF. Throughout, it fails on any CSP violation and on any request to another origin.

Three more suites use `e2e/remote-host.ts`, a local HTTPS server on another port that plays the file's site. It serves each file with CORS and ranges, with CORS but no ranges, or with neither, and logs every request it receives:

- **`remote.e2e.test.ts`** opens linked files. Page 1 of a 50 MB file appears after the reader reads less than a quarter of it. A site without CORS gets the explanation, and a change of `page` alone never opens the file again. It also checks the link format: an unencoded address and `http://` are both refused.
- **`encrypted.e2e.test.ts`** opens every encrypted fixture, each revision of the standard security handler from 2 to 6. A file with an empty user password opens at once. Any other asks for its password, rejects a wrong one, and opens with the user or the owner password.
- **`privacy.e2e.test.ts`** checks what leaves the browser while a local file, a linked file and an encrypted file of each kind open. It records every request of the page and its worker, and fails when a request body carries 32 bytes of the document or when a request to the reader's origin names the linked address. It also fails on a request to the reader's origin that isn't a GET and on a Referer or cookie reaching the file's site. Any cookie at all, any other origin and the password in any request fail it too.

```sh
pnpm --filter @laminapdf/web build
LAMINA_PDF_E2E_BROWSER=firefox pnpm --filter @laminapdf/web e2e   # chromium (default), firefox or webkit
```

`cargo xtask pdf-runtimes chromium|firefox|webkit` typechecks the app, builds it and runs the e2e suite in that browser, after the packages' own browser tests. The app's typecheck runs there rather than in `cargo xtask check ts` because it needs the built viewer.

`e2e/smoke.test.ts` checks a deployed reader instead, in Chromium: the same headers, and `hello.pdf` opened from the file input. The deploy workflow runs it after each deploy, and it runs against any address:

```sh
LAMINA_PDF_SMOKE_URL=https://staging-app.laminapdf.com pnpm --filter @laminapdf/web run smoke
```
