# Exemplary developer documentation sites and the interactive features that make docs work

Scope note: about 20 searches and fetches, November 2026. Many sources here are vendor blogs: Mintlify, Kapa, Algolia, StackBlitz and CodeSandbox. These are marked **[vendor]**. Opinions are marked **[opinion]**. Measured figures are marked **[measured]**. Where this pass found no source, the point is listed under Gaps rather than filled from memory.

## Which docs sites are most often cited as excellent, and what specifically is praised?

### Takeaway
Stripe is the site most consistently named as the benchmark. The reasons given are personalised, application-like docs (your test key in the samples, conditional content, interactive diagrams), all built on its own Markdoc. react.dev is the second reference point, for teaching with 600+ editable sandboxes and challenges. Twilio is praised for quickstarts and fast time to "hello world". The evidence for Tailwind and Laravel is only anecdotal. Documentation is developers' top learning resource, which is why this investment pays.

### Cited Findings
- **Why docs matter, [measured, survey]:** the 2024 Stack Overflow Developer Survey (65,000+ respondents) found technical documentation was the top online resource for learning to code, at about 83–84%. Stack Overflow came second at about 80% and written tutorials third at 68%. Among those who learn from documentation, 90% use API and SDK documents. — [Stack Overflow blog, 2024 results](https://stackoverflow.blog/2024/07/24/developers-want-more-more-more-the-2024-results-from-stack-overflow-s-annual-developer-survey/); [survey page](https://survey.stackoverflow.co/2024/developer-profile/)
- **Stripe, design goal:** Stripe's Docs Product team (Ryan Paul) says the docs should feel like an application, not a user manual. They have client-side interactivity: "checklists and collapsible sections", tab switchers, and conditional content that changes with the reader's location or the Stripe features they use. — [How Stripe builds interactive docs with Markdoc](https://stripe.dev/blog/markdoc) (also at [stripe.com/blog/markdoc](https://stripe.com/blog/markdoc))
- **Stripe, personalised samples:** the reader's own test API key is put into code samples, so copied code works with their account. — [stripe.dev/blog/markdoc](https://stripe.dev/blog/markdoc)
- **Stripe, interactive diagrams:** Markdoc tags (`diagram`, `node`, `edge`) render with React Flow. "Unlike static images, the diagrams that are built with React Flow can easily incorporate interactivity and clickable links." — [stripe.dev/blog/markdoc](https://stripe.dev/blog/markdoc)
- **Stripe, why it moved off ERB:** on the legacy stack, "Content authoring effectively became software development". Plain Markdown "isn't designed to express complex structure or hierarchy". — [stripe.dev/blog/markdoc](https://stripe.dev/blog/markdoc)
- **Stripe, reputation [opinion / vendor]:** Mintlify says that if you ask which company has the best docs, "you'll hear the same answer: Stripe". It praises the tailoring to where a user is in onboarding. — [Mintlify, How Stripe creates the best documentation in the industry](https://mintlify.com/blog/stripe-docs). A developer post is titled "Whoever does Stripe's technical writing deserves an award". — [DEV](https://dev.to/thomasstep/whoever-does-stripe-s-technical-writing-deserves-an-award-9f0). Roundups cite its UI/UX, interactive examples, depth and copy-pasteable snippets. — [Nordic APIs](https://nordicapis.com/5-examples-of-excellent-api-documentation/); [Apidog](https://apidog.com/blog/stripe-docs/)
- **react.dev (launched 16 March 2023):** "over 600!" sandboxes "everywhere throughout the site". Any sandbox can be edited, or forked into a new tab. Learn pages end with challenges that have solutions, and diagrams and illustrations appear where code and words weren't enough. Every API page is split into *Reference* (the signature) and *Usage* (why and how), and some add Troubleshooting and Alternatives sections. — [Introducing react.dev](https://react.dev/blog/2023/03/16/introducing-react-dev)
- **react.dev, rationale:** the old Hooks docs "assumed the reader is familiar with class components… New readers had to learn React twice". The new docs "teach React with Hooks from the beginning". The aim is for "everyone in the world to have an equal opportunity to learn React for free". Old docs were archived at legacy.reactjs.org. Writing the examples surfaced "mistakes in some of our own explanations, bugs in React, and even gaps in the React design". — [Introducing react.dev](https://react.dev/blog/2023/03/16/introducing-react-dev)
- **Twilio [opinion]:** praised for quickstarts and fast time to hello world, which one developer gave as the reason they chose Twilio. Also praised for its use-case guides and clean design. — [Nordic APIs](https://nordicapis.com/5-examples-of-excellent-api-documentation/); [ReadMe, Why these API docs are better than yours](https://readme.com/resources/why-these-api-docs-are-better-than-yours-and-what-you-can-do-about-it)
- **Tailwind [opinion, anecdotal]:** praised (in a Hacker News answer) because each class has its own section and every variant is explained, often with illustrations. Laravel's docs were called "straightforward and detailed" with practical examples in a DEV comment. — via the search roundup ([draft.dev, 12 documentation examples](https://draft.dev/learn/12-documentation-examples-every-developer-tool-can-learn-from); [DEV, best documentation you've used](https://dev.to/medusajs/whats-the-best-documentation-youve-used-83d/comments))
- **Svelte:** the Svelte tutorial uses Rollup in a web worker, which the repo calls faster and more reliable. The SvelteKit tutorial uses WebContainers "to boot up a Node runtime in the browser". — [sveltejs/svelte.dev PR #207](https://github.com/sveltejs/svelte.dev/pull/207/files). StackBlitz says Svelte's creator asked about using WebContainers outside stackblitz.com, which led to TutorialKit. — [StackBlitz, Announcing TutorialKit](https://blog.stackblitz.com/posts/announcing-tutorialkit/) **[vendor]**

### Inferences
- Across the praised sites, the common thread is not visual design. It is that **the reader can do something on the page**: run code, see their own key, follow a path matched to their goal. Stripe and react.dev both rebuilt their authoring tooling (Markdoc; Sandpack-in-MDX) so writers could add interactivity without becoming software developers.
- react.dev's split into Reference and Usage, with Troubleshooting sections, is a cheap structural pattern that Lamina's API pages could copy directly.

### Gaps
- This pass did not research site-specific praise for Vue, MDN, Django, Supabase, Prisma, Vercel, Cloudflare, the Rust book or docs.rs, PostgreSQL, SQLite, Turso, Astro, Bun, Deno, Clerk, Resend, Linear or Fly.io. A follow-up pass should cover them, or the writer should not claim specifics.
- Stripe publishes no metrics on its interactive docs. The Markdoc post gives no page counts, contributor counts or usage figures.
- No primary Tom Johnson (I'd Rather Be Writing) or Write the Docs source was retrieved in this pass.

## Interactive patterns and how they are built

### Takeaway
There are four main building blocks. A tag-based content language (Markdoc, or MDX) whose custom tags render components. An in-page sandbox: Sandpack, a hosted bundler in an iframe; WebContainers, Node in the browser, which needs cross-origin isolation; or a framework's own worker-based REPL. Compiler-checked static samples (Twoslash). And keyboard-first search (DocSearch, or Pagefind for static sites), now often paired with an AI "ask" panel and llms.txt or Markdown twins for agents.

### Cited Findings
- **Markdoc, how it is built:** markdown-it with a peg.js grammar for `{% tag %}` syntax. It parses to an AST that can be serialised to JSON for caching, then runs in phases: variables, schema-driven transform, then render (React dynamic or static, or HTML). Schemas set the allowed attributes and children. Validation runs in CI and in a VS Code extension as authors type. There are deliberately no loops and no variable assignment, "to ensure that the content is fully stateless". It is open source and used across Stripe. — [stripe.dev/blog/markdoc](https://stripe.dev/blog/markdoc); [markdoc.dev](https://markdoc.dev/)
- **Sandpack (CodeSandbox):** the React docs use Sandpack "for its examples and for the challenges", fully restyled by the React team. — [CodeSandbox, Sandpack announcement](https://codesandbox.io/blog/sandpack-announcement) **[vendor]**. The bundler can be self-hosted through `bundlerURL`, and transpilers load on demand. — [Sandpack docs, bundlers](https://sandpack.codesandbox.io/docs/advanced-usage/bundlers)
- **WebContainers (StackBlitz):** run Node in the browser. They need SharedArrayBuffer, so the page must be cross-origin isolated (COOP `same-origin` with COEP `require-corp`, or COEP `credentialless` to embed arbitrary resources). — [StackBlitz, Bringing WebContainers to all browsers](https://blog.stackblitz.com/posts/bringing-webcontainers-to-all-browsers/); [web.dev, Why COOP/COEP](https://web.dev/articles/why-coop-coep)
- **TutorialKit:** StackBlitz's framework for step-by-step interactive tutorials on WebContainers. It grew out of the Svelte tutorial. — [StackBlitz blog](https://blog.stackblitz.com/posts/announcing-tutorialkit/) **[vendor]**
- **Shiki Twoslash:** Shiki highlighting (the VS Code grammars) plus TypeScript compiler information. Samples "backed by the TypeScript compiler" get hover types, `// ^?` queries that show a type inline, and the compiler's errors. It is opt-in per block. Types resolve at build time from the app's node_modules; ATA fetches them from a CDN in the browser. There are integrations for VitePress, Fumadocs and Astro. — [Shiki Twoslash docs](https://shikijs.github.io/twoslash); [shiki.style/packages/twoslash](https://shiki.style/packages/twoslash); [Fumadocs Twoslash](https://fumadocs.dev/docs/ui/markdown/twoslash); [Typed snippets in Astro (DEV)](https://dev.to/alexanderop/typescript-snippets-in-astro-show-dont-tell-2a4l)
- **Algolia DocSearch:** free "for open-source and technical docs". It is a Ctrl+K command-palette modal, with adapters for Docusaurus and VitePress, headless React, and vanilla JS. "Ask AI" gives "Instant AI answers straight from your own docs" with sources, powered by Algolia Agent Studio. Setup is `npx @docsearch/cli setup`. — [docsearch.algolia.com](https://docsearch.algolia.com/) **[vendor]**
- **Pagefind:** static search that splits the index into ordered chunks, so a query loads only the chunks it needs. Full text and metadata for results are stored separately. It ships a drop-in UI themed with CSS variables. — [CloudCannon, Introducing Pagefind](https://cloudcannon.com/blog/introducing-pagefind/) (Starlight, which Lamina's docs use, ships Pagefind by default; this is from background knowledge, not checked in this pass).
- **AI "ask the docs" (Kapa.ai):** a "support form deflector" answers a question before the ticket is created. Analytics "surface the questions your docs don't answer yet". — [Kapa docs, Support Form Deflector](https://docs.kapa.ai/integrations/support-form-deflector); [Kapa, ticket deflector](https://www.kapa.ai/use-cases/support-ticket-deflector) **[vendor]**
- **llms.txt and Markdown for agents:** Mintlify added llms.txt to every docs site it hosts in November 2024, Anthropic's and Cursor's included. — [Mintlify, What is llms.txt](https://www.mintlify.com/blog/what-is-llms-txt) **[vendor]**. The proposal's v2 adds a `rel="alternate" type="text/markdown"` link per page. One review found only Cloudflare and Vercel advertise Markdown alternates that way. — [AnswerDotAI/llms-txt](https://github.com/AnswerDotAI/llms-txt); [TechWyse, llms.txt v2](https://www.techwyse.com/news/search-seo/llms-txt-v2-markdown-links-ai-agents). The examples listed include docs.stripe.com/llms.txt and developers.cloudflare.com/llms.txt. — [Codersera guide, May 2026](https://codersera.com/blog/llms-txt-complete-guide-2026/)
- **Playgrounds in vendor PDF SDK docs:** the Nutrient (formerly PSPDFKit) Web SDK README links an interactive in-browser playground, "without installing anything", plus a demo hub for viewing, annotation, forms, signatures, editing and generation. — [Nutrient Web SDK README](https://unpkg.com/@nutrient-sdk/viewer@1.17.0/README.md); [nutrient.io guides](https://nutrient.io/guides/web/viewer/pdf/)

### Inferences
- For Lamina, the SDK is plain TypeScript over WASM with no bundler or Node needed. That means a **lightweight custom runner beats both Sandpack and WebContainers**. Svelte made the same choice: its Svelte tutorial uses a worker, and only SvelteKit, which needs Node, uses WebContainers. An editor such as CodeMirror can send code to `@laminapdf/sdk/worker` and render the result with the existing `@laminapdf/viewer` or `render-pdfium`. That avoids WebContainers' cross-origin isolation, which ADR 0009/0011's "no COOP/COEP in v0.1" rules out anyway, and their Chromium-centred support.
- Twoslash suits a typed SDK well: build-time hover types on the existing tested example files would also fail the build when an example stops type-checking.
- Lamina's docs already have llms.txt and Markdown twins (L5a), which puts them among the leaders described here. A `rel="alternate" type="text/markdown"` link per page would match llms.txt v2.

### Gaps
- No source was found on persistent language or runtime switchers (e.g. Stripe's language tabs remembered across pages) or on "was this helpful" widgets, version switchers or copy buttons. These need a dedicated pass.
- No Inkeep or Mintlify AI-search case study was retrieved.

## Evidence: published metrics and performance costs

### Takeaway
Hard, independent metrics on what docs features achieve are scarce. Most numbers are vendor case studies: AI-assistant deflection of 20–70%, and Pagefind's under-300 KB for an MDN-sized search. The concrete cost numbers are that Sandpack's code cost react.dev about 150 KB until lazy-loaded, that WebContainers require cross-origin isolation and are fully supported only in Chromium, and that a DuckDB-Wasm shell can pull megabytes of WASM.

### Cited Findings
- **Kapa.ai deflection [vendor]:** Appian 70% fewer support cases; ClickHouse 37% deflection; Mapbox 20% fewer monthly tickets; CircleCI 28% faster responses. Kapa calls 30–50% "realistic" for a well-tuned assistant, and says its Support Form Agent resolves about 40% on average, up to 70%. — [Kapa, AI platforms and support costs](https://www.kapa.ai/library/how-ai-platforms-reduce-support-costs-and-how-to-compare-them-(2026)); [Kapa, CircleCI](https://www.kapa.ai/customer-examples/circleci); [Mapbox case study listing](https://www.casestudies.com/company/kapaai/case-study/mapbox-reduces-monthly-support-tickets-by-20-with-kapaai); [Kapa, Support Form Agent](https://www.kapa.ai/solutions/support-form-agent). None of these figures is audited.
- **llms.txt use [mixed]:** one SE Ranking study of 300,000 domains found 10.13% adoption. Redocly reports "no model we tested read llms.txt on its own"; coding agents such as Cursor and Claude Code are the clearest consumers. — [Codersera summary](https://codersera.com/blog/llms-txt-complete-guide-2026/); [Redocly, llms.txt still overhyped](https://redocly.com/blog/llms-txt-still-overhyped) **[opinion, vendor]**
- **Sandpack cost [measured, one site]:** a react.dev PR splitting Sandpack and code blocks out of the main bundle saved about 9.55 KB with Suspense around Sandpack, and about 150 KB with a lazy wrapper around code blocks and Sandpack. — [reactjs/react.dev PR #5164](https://github.com/reactjs/react.dev/pull/5164). Sandpack-react 0.15.0 had a 626 kB unpacked npm size, an old release and not the transfer size. — [npm](https://npmjs.com/package/@codesandbox/sandpack-react/v/0.15.0)
- **WebContainers browser support (page updated October 2026):** Chromium desktop browsers are supported, though Brave and Chrome with third-party cookies blocked may need configuration. Safari 16.4 TP and later is beta. Firefox is alpha: it lacks full support for the required isolation mode, and third-party assets may be blocked in the preview. "Credentialless" isolation is enabled only in Chromium. — [webcontainers.io/guides/browser-support](https://webcontainers.io/guides/browser-support). StackBlitz's February 2023 post said embedding worked only in Chromium. — [StackBlitz blog](https://blog.stackblitz.com/posts/bringing-webcontainers-to-all-browsers/). Cross-origin isolation can break popup flows such as payment and login SDKs. — [JavaScript in Plain English](https://javascript.plainenglish.io/running-webcontainer-in-the-browser-without-breaking-firebase-or-razorpay-63d49a56d7c8)
- **Pagefind [project's own claim]:** "full-text search on the entirety of MDN in under 300KB total, including the Pagefind library itself"; "For most sites, this will be closer to 100KB". — [CloudCannon, Introducing Pagefind](https://cloudcannon.com/blog/introducing-pagefind/). A user report on a 489-article site found an index of about 5 MB in total, but only the needed chunks download. — [raymii.org](https://raymii.org/s/blog/Site_update_self_hosted_search_via_pagefind.html)
- **DocSearch scale [vendor, inconsistent]:** the same page says both "12,000+ Projects" and "Join 9,000+ projects", and claims "<20 ms" latency. — [docsearch.algolia.com](https://docsearch.algolia.com/)

### Inferences
- Lamina already measures its own costs: the SDK's 20 KB JS budget and per-module WASM budgets. Any live-example feature on the docs site should load **on interaction** (click "Run", or when the example scrolls into view), so pages that never run code pay nothing. react.dev's lazy wrapper is the precedent.
- If an AI "ask" panel is added, its claimed value is deflection. For an open-source project with no support desk, the analytics on unanswered questions may be worth more than deflection.

### Gaps
- No published usage numbers were found for react.dev sandbox runs, Svelte tutorial completion rates or Stripe's interactive features.
- No current gzipped size was found for `@codesandbox/sandpack-react`, and none for the WebContainer API client.

## In-browser WASM-powered docs and how they load WASM lazily

### Takeaway
The WASM-heavy docs and playgrounds found (DuckDB-Wasm, Typst ports) choose between several WASM builds at runtime and run the engine in a Web Worker. Optional parts load only when first used. This matches what Lamina already has: three module variants, the `@laminapdf/sdk/worker` client, and the hayro and PDFium backends.

### Cited Findings
- **DuckDB-Wasm bundles:** `mvp`, `eh` (exception handling) and `coi` (threaded, which needs cross-origin isolation). `selectBundle` picks `coi` only on an isolated page in a capable browser and otherwise falls back, so one code path runs everywhere. `getJsDelivrBundles()` returns only mvp and eh. — [DuckDB docs, Deploying DuckDB-Wasm](https://www.duckdb.org/docs/current/clients/wasm/deploying_duckdb_wasm); [instantiation](https://www.duckdb.org/docs/1.5/clients/wasm/instantiation.html)
- **DuckDB-Wasm lazy extensions:** `INSTALL x FROM 'url'` defers the fetch until the first `LOAD x`. In the shell experiment no extensions are preloaded, so `LOAD parquet;` must run first. — [duckdb-wasm-wip](https://www.github.com/duckdb/duckdb-wasm-wip). A third-party page says the shell's extension loading transfers about 3.2 MB of compressed WASM on a first visit. — (third-party, via search; unverified)
- **Typst in the browser:** typst.ts "can run the entire typst directly in browser, like typst.app", with React and Angular bindings and WASM modules from jsDelivr. — [typst.ts](https://awesome.ecosyste.ms/projects/github.com%2Fmyriad-dreamin%2Ftypst.ts). typst_live compiles to PDF locally after page load. — [data-niklas/typst_live](https://github.com/data-niklas/typst_live). typst-wasm targets browsers, Node and serverless runtimes. — [Typst forum](https://forum.typst.app/t/typst-wasm-compile-typst-in-browsers-node-and-serverless-runtimes/9399). @vedivad/typst-web-service compiles in a Web Worker (`compilePdf`). — [pkgstats](https://www.pkgstats.com/pkg:@vedivad/typst-web-service)

### Inferences
- A Lamina live example could follow the same steps. Load nothing at page load. On "Run", start the docs worker, and load the text or extract module only if the example uses `Font` or `extract`. Then render page 1 through the backend already shipped to the viewer. Because the SDK and examples are byte-identical to the Rust goldens, the docs could also show the expected hash or golden next to a live run.

### Gaps
- The Ruff and Biome playgrounds, Shiki's playground, sql.js, Pyodide/JupyterLite, ffmpeg.wasm and the PDF.js demo viewer were not examined, so their loading strategies are unrecorded here.

## Patterns specific to PDF and document libraries' docs

### Takeaway
PDF libraries' own docs mostly **link out** for live output instead of showing it beside the code. pdf-lib sends readers to JSFiddle. react-pdf has a separate playground page, and Nutrient has a separate playground and demo hub. No PDF library found in this pass shows code and its rendered page side by side inline on the reference page, which is the gap Lamina could fill.

### Cited Findings
- **pdf-lib:** examples are code snippets, nearly each with a "Try the JSFiddle demo" link to an external JSFiddle. The metadata example shows console output as text ("This script outputs the following:"). The quick start says "Save this snippet as an HTML file and load it in your browser". — [pdf-lib.js.org](https://pdf-lib.js.org/)
- **react-pdf:** has a separate playground page at /repl ("Playground | react-pdf"). Its client-rendered page could not be read, so whether output appears beside the code was not confirmed. — [react-pdf.org/repl](https://react-pdf.org/repl)
- **Nutrient (PSPDFKit):** a separate in-browser playground and a demo hub organised by capability. — [Nutrient Web SDK README](https://unpkg.com/@nutrient-sdk/viewer@1.17.0/README.md)
- **Typst:** typst.app is itself an editor with a live preview, and community ports reproduce it with WASM. — [typst.ts](https://awesome.ecosyste.ms/projects/github.com%2Fmyriad-dreamin%2Ftypst.ts); [typst_live](https://github.com/data-niklas/typst_live)

### Inferences
- **Recommendation:** put an inline code and rendered-page pair on each generation example, using the existing tested example files, so the code shown is the code that ran. Pre-render a static PNG of the golden at build time so the page shows output with no JS and no WASM. Upgrade it to a live, editable run on click. This combines react.dev's editable sandbox with static-first performance.

### Gaps
- Apryse, PDF.js's examples, Prawn, ReportLab and iText docs were not examined in this pass.

## Accessibility and performance practices of top docs sites

### Takeaway
This pass found no published Core Web Vitals or page-weight numbers for the top docs sites. The only accessibility statement found was react.dev's aim to make sandboxes "accessible not just for everyone but everywhere", in a CodeSandbox contributor's account.

### Cited Findings
- react.dev's sandboxes: the React team "challenged us to provide a browser-based playground… and ensure these sandboxes are accessible not just for everyone but everywhere". — [Danilo Woznica, Sandpack self-reflection](https://danilowoz.vercel.app/blog/sandpack) **[contributor's account]**
- The react.dev launch post lists accessibility guides only as future work ("Creating the updated performance, testing, and accessibility guides"). — [Introducing react.dev](https://react.dev/blog/2023/03/16/introducing-react-dev)

### Inferences
- The lazy-loading evidence above (react.dev's 150 KB, Pagefind's chunked index) suggests the default for static docs should be zero JS for interactive features until a reader uses them.

### Gaps
- No CrUX, HTTP Archive or Lighthouse figures were gathered for Stripe, MDN, react.dev and the others. A follow-up should pull CrUX data.
- No sources were found on retired docs features (e.g. sites that dropped sandboxes or AI search).
