# Showing what an example makes: previews and live code in the docs

**Status:** research, 2026-11-08. Feeds the [docs examples plan](../../plans/docs-examples.md).

A reader of a recipe page sees the code and a link, "open the reference PDF", that leaves the page. They can't see what the code makes without opening a file, and they can't try a change without setting up a project. Other documentation sites solve one or both problems in five ways: pictures of the output, live samples, hosted sandboxes, a hosted bundler, and a playground of their own. This note compares the five for a PDF library whose docs site has strict security headers. It recommends pictures of every document an example makes, now, and later a small runner of our own for the few TypeScript examples worth editing.

## Where the docs stand

The docs site shows the tested example files themselves (`Snippet.astro` reads them at build time), in TypeScript and Rust tabs. Ten recipe pages end the code with a link to the example's PDF, which the build copies from `fixtures/golden/examples/` into `/examples/pdf/`. Nothing on the page shows the PDF.

Three facts limit the choices:

- **The headers.** `apps/docs/public/_headers` allows scripts, frames and requests only from the docs site itself (`script-src 'self' 'wasm-unsafe-eval'`, `frame-src 'self'`, `connect-src 'self'`), as [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) §3 and §4 ask of every site we serve. Code that runs eval-style (`new Function`) is blocked too.
- **The weight.** Running an example in the browser needs `@laminapdf/sdk`'s text module, about 1.2 MB gzipped (`packages/wasm/size-budgets.json`), and showing the result in the page needs a renderer: PDFium at 2.2 MB or hayro at 1.9 MB.
- **What already exists.** Every example PDF is rendered to PNG at 72 dpi by PDFium and hayro as render goldens (`fixtures/render-golden/`), and `@laminapdf/viewer` renders PDFs in the browser.

## How other docs sites do it

| Approach | Who | How it works | For us |
| --- | --- | --- | --- |
| Pictures of the output | Android (Jetpack Compose), Typst's ecosystem | Static code, then a figure of what it draws, made at build time. Android's "Style text" page follows each Compose snippet with a PNG of the result. | No JavaScript, no download, works with the headers as they are. The pictures must stay in step with the code. |
| Live samples | MDN | The page's own code blocks are merged into HTML and run in a sandboxed iframe; a Play button opens them in the MDN Playground to edit. | The pattern fits: our own runner, on our own site. MDN runs HTML, not a library with WebAssembly. |
| Hosted sandbox | TanStack (StackBlitz, CodeSandbox links), Kotlin docs (Kotlin Playground) | Each example opens a full project on another service: StackBlitz runs Node in the browser with WebContainers; Kotlin Playground compiles on JetBrains' server. | Breaks the headers and ADR 0011's same-site rule; installs our packages from npm, so it shows the published version, not the docs' version; the WebContainer API needs a commercial licence for non-open-source use; cold starts take seconds. |
| Hosted bundler | React (Sandpack) | A bundler in an iframe on another origin builds and runs the files; it can be self-hosted on a subdomain. | React-shaped and large; self-hosting means a second origin to run and keep in step. |
| Playground of its own | Svelte, Vue (`@vue/repl`), TypeScript | A browser editor plus the project's own compiler in the page or a worker, served as static files. | The closest fit: same site, the docs' own version, no server. Costs code we write and keep. |

PDF libraries themselves mostly link out: pdfmake keeps a separate playground page, and dubroy.com's PDF Playground renders raw PDF syntax with pdf.js. None found shows code and the document side by side on each docs page, so tabs that do would set the Lamina PDF docs apart.

## The options for Lamina PDF

1. **A Result tab with pictures of the pages** (Android's approach). Each example that makes a document gets Code and Result tabs; Result shows every page as an image rendered at 2× (144 dpi) from the example's golden PDF, with the link to open the PDF kept under it. The pictures cost nothing to load until the tab opens, need no script, and show the same bytes the tests check. The render-golden tooling (`tools/render`) can write them, and a docs check can fail when a golden PDF changes without its pictures.
2. **The reference PDF in our own viewer**, lazily. Zoom and scroll, at the cost of 2.2 MB on first open. Pictures already show what a reader needs; the viewer adds little on a docs page.
3. **A hosted sandbox** (StackBlitz, CodeSandbox). Rejected: it breaks the headers and ADR 0011's same-site rule, runs the published package rather than the docs' version, needs a licence for a private repository, and starts slowly.
4. **A self-hosted Sandpack bundler.** Rejected: a second origin to run, built around React, and heavier than what we need.
5. **A runner of our own** for TypeScript examples (Svelte's and MDN's approach). An "Edit and run" button on the Result tab loads an editor (CodeMirror 6, MIT) and strips the types (Sucrase, MIT; no type check). It runs the code against the docs' own `@laminapdf/sdk` in a sandboxed iframe and draws the new PDF with `@laminapdf/viewer`. The iframe page lives on the docs site with its own, looser headers (`'unsafe-eval'` for that page only). Its `sandbox="allow-scripts"` attribute, without `allow-same-origin`, keeps the code it runs away from the docs' storage. Nothing loads until the reader presses the button; then about 3.5 MB (the SDK's text module and PDFium) and the editor. Rust examples stay read-only: compiling Rust in the browser isn't practical.

## Recommendation

Do option 1 for every example that makes a document, now: it answers "what does this make?" on the page, for both languages, at no cost to page weight. Do option 5 next, for the handful of TypeScript examples where changing a value teaches something (a page size, a font, a missing alt text that PDF/UA refuses), loaded only when the reader asks. Leave hosted sandboxes out: they conflict with the site's headers and with docs that match their own version.

## Sources

- Android: [Style text](https://developer.android.com/develop/ui/compose/text/style-text), code followed by figures of the result.
- TanStack: [Query examples](https://tanstack.com/query/v4/docs/framework/examples), with StackBlitz and CodeSandbox links; [WebContainer API](https://webcontainers.io/api) (licence terms) and [its announcement](https://blog.stackblitz.com/posts/webcontainer-api-is-here/).
- Kotlin: [Kotlin Playground on npm](https://npmjs.com/package/kotlin-playground), [mkdocs-kotlin-playground](https://github.com/twangodev/mkdocs-kotlin-playground).
- React: [Sandpack: hosting the bundler](https://sandpack.codesandbox.io/docs/guides/hosting-the-bundler), [Josh Comeau's playground](https://joshwcomeau.com/react/next-level-playground).
- MDN: [Live samples](https://developer.mozilla.org/en-US/docs/MDN/Writing_guidelines/Page_structures/Live_samples), [Introducing the MDN Playground](https://developer.mozilla.org/en-US/blog/introducing-the-mdn-playground/).
- Vue: [`@vue/repl`](https://cdn.jsdelivr.net/npm/@vue/repl@4.7.2/README.md); LiveCodes, a self-hostable playground: [github.com/live-codes/livecodes](https://github.com/live-codes/livecodes).
- PDF tools: [pdfmake](https://github.com/covera/pdfmake) (playground link in its README), [PDF Playground](https://dubroy.com/pdf-playground/).
- Typst: [mdbook-typst-doc](https://github.com/fenjalien/mdbook-typst-doc) and [showman](https://typst.app/universe/package/showman), code and its rendered output together.
