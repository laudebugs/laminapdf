# The current Lamina PDF docs site (apps/docs): a baseline for a redesign

Audited on 2026-11-11 against `origin/main` at `5abe81d` ("release: v0.2.0-alpha.6 (#427)"), checked out as a temporary detached worktree, since removed (the main checkout was 84 commits behind). Sources below are repository paths; "local build" means `pnpm --filter @laminapdf/docs build` run there on 2026-11-11. Nothing tracked was modified.

## Structure and flow: sections, sidebar order, page counts, the path to a first PDF, Diátaxis coverage

### Takeaway
The site is a Starlight site with 60 hand-written pages in 9 sidebar groups plus about 320 generated reference pages (149 TypeDoc, 171 error-code pages). The order follows an explicit "docs as a story" sidebar: Start here → runtime → concepts → tasks → CLI → production → recipes → troubleshooting → reference. The home page gets a reader to a working PDF in two clicks, through a Node-only five-minute tutorial. Reference and how-to content are strong. There is exactly one tutorial, and some areas have no guide: text extraction from TypeScript or Rust beyond one recipe, the viewer and render packages, and redaction/reveal outside the CLI.

### Cited Findings
- The sidebar is an explicit list in `plugins/sidebar.mjs`, called from `astro.config.mjs`. The comment there says "the order is a decision (the docs as a story, quality plan §3), so every page is listed", and llms.txt follows the same order — [apps/docs/plugins/sidebar.mjs](../../../../apps/docs/plugins/sidebar.mjs)
- Sidebar groups and item counts, in order — [apps/docs/plugins/sidebar.mjs](../../../../apps/docs/plugins/sidebar.mjs):
  1. **Start here** (4): What is Lamina PDF (`/`), Install, Your first PDF, Use with coding agents.
  2. **Your runtime** (6): Browser and bundlers, Node, Deno, Bun, Cloudflare Workers, Rust.
  3. **Concepts** (5): How Lamina PDF thinks about a PDF, Pages and coordinates, Errors and warnings, What Lamina PDF writes and what it borrows, How a page is drawn.
  4. **Create PDFs** (6): Text and fonts, Shapes colour and images, Links outlines and metadata, Pages that flow, Accessible PDFs, Archival PDFs.
  5. **Read and edit PDFs** (5): Edit a PDF, Save a PDF, Open password-protected files, Open files you don't trust, Open large files by ranges.
  6. **Command line** (7): overview, Install the tool, Inspect and check, Extract text and render pages, Make and change PDFs, Scripts and batch jobs, Connect an AI agent (MCP).
  7. **Production** (2): Keep the page responsive, Test the PDFs you generate.
  8. **Recipes** (16 incl. "All recipes").
  9. **Troubleshooting** (1 page).
  10. **Reference**: TypeDoc group (collapsed), Rust API (a page linking docs.rs), Error and warning codes, Stability policy, Benchmarks, Changelog, Glossary.
- `src/content/docs/` holds 60 `.mdx` files. By folder: cli 7, concepts 5, guides 13, quick-start 6, recipes 17, reference 7, start 2; root 3 (`index`, `install`, `troubleshooting`) — [apps/docs/src/content/docs/](../../../../apps/docs/src/content/docs/)
- **Two recipe pages are not in the sidebar:** `recipes/invoice-from-json.mdx` and `recipes/markdown-to-pdf.mdx`. Only the recipes index and other pages link to them. llms.txt's builder puts unlisted pages in a trailing "More" section. This breaks the house rule "MUST: add every new page to the explicit sidebar" — [apps/docs/plugins/sidebar.mjs](../../../../apps/docs/plugins/sidebar.mjs), [apps/docs/src/content/docs/recipes/index.mdx](../../../../apps/docs/src/content/docs/recipes/index.mdx) (lines 19–20), [apps/docs/plugins/agent-docs.mjs](../../../../apps/docs/plugins/agent-docs.mjs) (lines 347–353), [docs/guidelines/docs-writing.md](../../../../docs/guidelines/docs-writing.md)
- Page lengths (source lines, including frontmatter and imports) — `wc -l` over [apps/docs/src/content/docs/](../../../../apps/docs/src/content/docs/):
  - 2,754 lines in total, so pages are short.
  - Quick starts: 18–29 lines.
  - Recipes: mostly 30–33 lines; longest is invoice-from-json at 56.
  - Guides: 35–61 lines, except flowing-pages at 129.
  - CLI pages are the longest hand-written ones: 46–112 lines.
  - Glossary: 123 lines.
  - Rendered words in `<main>` (Playwright, local build): home 349, first-pdf 461, save-a-pdf 715, recipes/invoice 898, `/reference/codes/wrong-password/` 117, TypeDoc `Document` class 2,278.
- The local build produced 380 pages; Pagefind indexed 386 HTML files. `dist/` held 385 `index.html` files: 149 under `reference/sdk/` (TypeDoc) and 171 under `reference/codes/` (one per error or warning code). `docs/errors/` holds 171 entries (170 code pages plus README.md) — local build log; [docs/errors/](../../../../docs/errors/)
- **The home page** (`index.mdx`) uses Starlight's `splash` template:
  - Hero actions: "Your first PDF" (primary) and "See the recipes".
  - A "What works today" table of five areas (Generate, Navigate, Edit, Read safely, Errors), each linking its guide.
  - It ends: "Start with your first PDF, then read how Lamina PDF thinks about a PDF. When you're ready for your own setup, pick your runtime."
  - It claims: "Every code block on this site is a tested example file, so the code you see is the code that runs."
  — [apps/docs/src/content/docs/index.mdx](../../../../apps/docs/src/content/docs/index.mdx)
- **Path to a first PDF:** home → "Your first PDF" (`start/first-pdf.mdx`, "Write hello.pdf from Node in five minutes"), which runs `mkdir`, `npm init -y && npm pkg set type=module`, `npm install @laminapdf/sdk@{{version}}`, then shows `packages/sdk/examples/quick-start/node.ts` with a Result tab and `node hello.ts`. It needs "Node 24.2 or newer, which runs `.ts` files directly". It then sends the reader to the concepts page and the runtime pages. There is no Rust or CLI equivalent of this tutorial: Rust readers land on `quick-start/rust.mdx` (18 lines) — [apps/docs/src/content/docs/start/first-pdf.mdx](../../../../apps/docs/src/content/docs/start/first-pdf.mdx), [apps/docs/src/content/docs/quick-start/rust.mdx](../../../../apps/docs/src/content/docs/quick-start/rust.mdx)
- **Version pinning is a first-run hurdle:** Install warns "A plain `npm install` or `cargo add` picks a `0.0.0` placeholder that only reserves the name", so every install command names `{{version}}` (filled at build time by `versionInPages`) — [apps/docs/src/content/docs/install.mdx](../../../../apps/docs/src/content/docs/install.mdx), [apps/docs/astro.config.mjs](../../../../apps/docs/astro.config.mjs)
- Moved pages keep their URLs through static redirects in `astro.config.mjs`. Six are listed, for example `/guides/open-edit-and-save/` → `/guides/edit-a-pdf/` and `/examples/` → `/recipes/` — [apps/docs/astro.config.mjs](../../../../apps/docs/astro.config.mjs)
- No guide page mentions `@laminapdf/viewer`, `@laminapdf/render-pdfium` or `createViewer`. Only `install.mdx`, `concepts/dependencies.mdx` and `reference/rust.mdx` name them. `reveal` and redaction appear only in CLI pages (`cli/inspect`, `cli/scripts`, `cli/mcp`) — grep over [apps/docs/src/content/docs/](../../../../apps/docs/src/content/docs/)
- Text extraction for SDK and Rust users has one recipe (`recipes/extract-markdown.mdx`, "Extract text for an LLM", with an Output tab) and a CLI page. "Read and edit PDFs" has no extraction guide — [apps/docs/plugins/sidebar.mjs](../../../../apps/docs/plugins/sidebar.mjs)

### Inferences
- Diátaxis mapping (my classification, not a label the site uses):
  - **Tutorial:** one page, `start/first-pdf` (Node). The six quick starts are tutorial/how-to hybrids of 18–29 lines.
  - **How-to:** 13 guides (6 create, 5 read/edit, 2 production), 7 CLI pages, 16 recipes and troubleshooting. This is the bulk.
  - **Explanation:** 5 concept pages, plus "Details" asides inside guides.
  - **Reference:** TypeDoc (149 pages), 171 code pages, glossary, stability, benchmarks and changelog; Rust reference is off-site on docs.rs.
- Thin or missing:
  - Tutorials beyond hello world: no multi-step "build a real app" lesson, no Rust or CLI first-run tutorial, no reading or extraction tutorial.
  - Guides for extraction, the viewer and rendering packages, and redaction/reveal from code.
  - Explanation of extraction concepts beyond diagrams on the recipe page.
- The story order is strong for a generator user. A reader whose job is "read or extract from PDFs" has no entry on the home page's table and no tutorial: the "What works today" table lists Generate, Navigate, Edit, Read safely and Errors, but not Extract.

### Gaps
- No analytics exist (ADR 0011: opt-in-only telemetry), so there is no data on how readers actually move through the site. The diagrams plan says this outright: "No usage data backs it yet; the docs collect none" ([docs/plans/docs-diagrams.md](../../../../docs/plans/docs-diagrams.md)).
- I did not open every one of the 60 pages; content-quality judgements below rest on a sample of about 14.

## Content quality: openings, tested code, language coverage, errors, troubleshooting, what's missing

### Takeaway
Content is tightly governed. Every TypeScript and Rust code block on guide and recipe pages comes from a tested example file through `Snippet`/`Example`, and the build fails otherwise. Twenty pages show TypeScript and Rust side by side. Pages open with a short "You'll make X; you need it when Y" paragraph, and every error code has its own page. The weak spots:
- The CLI pages' commands and outputs are hand-copied into `console` blocks with no check.
- Troubleshooting is one page of 10 entries, mostly about JS setup.
- The writing rules are enforced mechanically, which keeps pages short, sometimes terse.

### Cited Findings
- **House rules** (`docs/guidelines/docs-writing.md`, 76 lines):
  - The "Tell a story" sidebar order is a MUST.
  - "MUST: open every guide and recipe with one paragraph of at most 50 words that says what you'll make and when you'd need it. Show the code next, then the details."
  - Sentences of 40 words or fewer (30 SHOULD); paragraphs of 100 words or fewer.
  - No ADR or todo IDs, "golden", CI or repository paths in reader text.
  - Spec terms explained, linked to the glossary or allow-listed.
  - Banned words: "simply", "just", "easy", "note that", "via"…
  - "MUST: show code only from a tested example file."
  - Diagrams only through the kit, each with a text description.
  
  — [docs/guidelines/docs-writing.md](../../../../docs/guidelines/docs-writing.md)
- **Examples rules** (`docs/guidelines/examples.md`, 54 lines):
  - An example "does one thing and reads top to bottom… could paste it into an app and run it unchanged".
  - Public imports only; "no test scaffolding": fixed IDs and dates are parameters.
  - `using` frees what a TypeScript example creates.
  - Named layout numbers; functions ≤ 40 lines, files ≤ 100.
  - TS and Rust twins share a name and first header sentence.
  
  — [docs/guidelines/examples.md](../../../../docs/guidelines/examples.md)
- **Enforcement:** `cargo xtask check docs-prose` checks the plain-English MUSTs and the opening paragraph; `check docs-snippets`, a Biome override and `clippy-examples` check the example rules. `cargo xtask check docs` runs them strict on every file. "The sidebar order, the SHOULD rules, the reviewer checklist and the 'pleasure to read' advice stay with reviewers" — [docs/guidelines/docs-writing.md](../../../../docs/guidelines/docs-writing.md), [docs/guidelines/examples.md](../../../../docs/guidelines/examples.md)
- **`Snippet.astro`** globs `packages/sdk/examples/**/*.{ts,toml,md}` and `crates/laminapdf/examples/*.rs` at build time. It throws "Snippet: … is not one of the example files apps/docs may show" for any other path — [apps/docs/src/components/Snippet.astro](../../../../apps/docs/src/components/Snippet.astro)
- **`Example.astro`** puts TypeScript, Rust and a Result (or Output) tab in one Starlight `<Tabs syncKey="lang">` row:
  - Result shows lazy-loaded PNGs of every page from `public/examples/preview/` and a link "Open <stem>.pdf (N pages, size)".
  - The build fails if an example has no pictures ("run `cargo xtask docs-previews`").
  - A script keeps the remembered tab on a language, so no page opens on Result.
  
  — [apps/docs/src/components/Example.astro](../../../../apps/docs/src/components/Example.astro)
- **Example inventory:**
  - 22 entries in `packages/sdk/examples/`: TS files plus `AGENTS.md`, `quick-start/` and `worker/`.
  - 17 entries in `crates/laminapdf/examples/`.
  
  — [packages/sdk/examples/](../../../../packages/sdk/examples/), [crates/laminapdf/examples/](../../../../crates/laminapdf/examples/)
- **Language coverage** (grep over the `.mdx` pages):
  - 20 pages use `<Example>` with both `ts=` and `rust=`.
  - 7 `<Example>` pages are TypeScript-only (no `rust=`; the JS quick starts and first-pdf among them).
  - 2 are Rust-only (no `ts=`): `quick-start/rust` and `recipes/invoice-from-json`. TypeScript readers of the JSON-template recipe therefore see Rust alone (T4j would add the TS side).
  - The 7 CLI pages, install, home, dependencies, glossary, stability, benchmarks, changelog, codes and troubleshooting have no Snippet/Example.
  - Hand-written fenced code: 14 pages contain `sh` blocks, and the CLI pages use `console` blocks.
  
  — [apps/docs/src/content/docs/](../../../../apps/docs/src/content/docs/)
- **CLI pages are not checked like the examples.** The L4f todo says "every command and output copied from a run of the CLI on `main`". Its only build check compares the download table with `dist-workspace.toml`. Example from `cli/inspect.mdx`: `$ laminapdf info invoice.pdf` followed by hand-pasted output — [docs/todos/done/L4f-command-line-docs.md](../../../../docs/todos/done/L4f-command-line-docs.md), [apps/docs/src/content/docs/cli/inspect.mdx](../../../../apps/docs/src/content/docs/cli/inspect.mdx)
- **Sampled openings follow the rule.** Edit a PDF: "You'll open a PDF, change its title and author, and save the edit after the original bytes. You need this whenever your app changes a file someone gave it…". Text and fonts: "You'll draw text on a page… You need this whenever a PDF holds words…". Both show the `<Example>` straight after — [apps/docs/src/content/docs/guides/edit-a-pdf.mdx](../../../../apps/docs/src/content/docs/guides/edit-a-pdf.mdx), [apps/docs/src/content/docs/guides/fonts-and-text.mdx](../../../../apps/docs/src/content/docs/guides/fonts-and-text.mdx)
- **Concept pages state the model plainly.** "How Lamina PDF thinks" has six ideas (bytes in/out; same input, same bytes; the opened file never changes; two ways to save; you free what you create; three JS modules), with two diagrams and one example — [apps/docs/src/content/docs/concepts/how-lamina-thinks.mdx](../../../../apps/docs/src/content/docs/concepts/how-lamina-thinks.mdx)
- **Guides use `:::note[Details]` asides for spec detail**, for example the fonts guide's Type 0, `/W`, ToUnicode, UAX #14 and #9 aside — [apps/docs/src/content/docs/guides/fonts-and-text.mdx](../../../../apps/docs/src/content/docs/guides/fonts-and-text.mdx)
- **Errors:**
  - `concepts/errors-and-warnings.mdx` explains the one error shape (`code`, `kind`, `message`, `hint`, `location`, `data`, `docsUrl`, `didYouMean`) and says "Branch on its `code`, never on its `message`".
  - It also says document text appears only in `data`.
  - `reference/codes.mdx` (11 lines) renders the table from `fixtures/codes.json`.
  - Each code page comes from `docs/errors/<code>.md`, for example `wrong-password.md`, with sections "What happened / Why / How to fix / Example / Data fields".
  - Some code pages still carry spec shorthand, for example "owner first for R5/R6", "in TS also its NFKC form".
  
  — [apps/docs/src/content/docs/concepts/errors-and-warnings.mdx](../../../../apps/docs/src/content/docs/concepts/errors-and-warnings.mdx), [docs/errors/wrong-password.md](../../../../docs/errors/wrong-password.md)
- **Troubleshooting** is one 68-line page with 10 symptom-first entries, each giving Cause, Fix and links:
  - `not-initialized`, `text-module-not-loaded`, `extract-module-not-loaded`, a CSP blocking WebAssembly, `init()` in Vite, `init()` in a Cloudflare Worker, passwords, signed file won't save, memory growth, `out-of-memory`/`wasm-trap`.
  - None covers Rust, the CLI, extraction quality or rendering.
  
  — [apps/docs/src/content/docs/troubleshooting.mdx](../../../../apps/docs/src/content/docs/troubleshooting.mdx)
- **Recipe example with real-world framing:** `recipes/invoice-from-json.mdx` shows a JSON template excerpt, then has "How it works", "What to change" (including the failure modes `invalid-document` with `didYouMean`, and `template-data-missing`) and "Build it without code" with the CLI command — [apps/docs/src/content/docs/recipes/invoice-from-json.mdx](../../../../apps/docs/src/content/docs/recipes/invoice-from-json.mdx)
- **API reference split:**
  - TypeScript: TypeDoc via `starlight-typedoc` into `/reference/sdk/`, a collapsed sidebar group, generated at build time and gitignored.
  - Rust: `reference/rust.mdx` (30 lines) links each crate to docs.rs; no Rust reference on the site itself.
  - "Defined in" source links and "Edit page" are off unless `LAMINA_PDF_DOCS_GITHUB=1`, "while the repository is private they would 404".
  
  — [apps/docs/astro.config.mjs](../../../../apps/docs/astro.config.mjs)
- **The Round Q review of 2026-10-31** recorded the problems the current structure fixed:
  - Runtime pages showing CI test runners.
  - "No first-PDF path and no concepts page".
  - Spec jargon; "45 of the 117 error-code pages" with ADR or todo IDs.
  - An autogenerated sidebar.
  - Test scaffolding in examples.
  
  The Q1a–Q1m todos are all in `done/` — [docs/plans/quality.md](../../../../docs/plans/quality.md) §2.1–2.2, [docs/todos/done/](../../../../docs/todos/done/)

### Inferences
- The "every code block is tested" promise holds for TS and Rust, but not for CLI transcripts or the `sh` install blocks. A redesign that adds more CLI or terminal content should bring those under a check too, for example by generating transcripts from runs.
- The 50-word opening and the sentence and paragraph caps produce consistent, scannable pages. The pages are short (most guides are 35–60 source lines), so depth often lives in the TypeDoc reference or is missing. Examples: Edit a PDF covers transactions, undo and JSON replay in three short sections, with no example of `rotatePage` or of handling `partial` undo.
- What a reader can't find on the site:
  - A guide for extracting text or tables from code.
  - Anything on the viewer or render packages, although Install lists them.
  - Rust-specific troubleshooting.
  - A Rust API reference hosted on the site.
  - Searchable "how do I…" titles. W3h plans this; current titles are feature names.

### Gaps
- No user research, support tickets or search-query logs exist to show which questions readers fail to answer.
- I did not run `cargo xtask check docs-prose` or `docs-snippets` (they need a Rust build of xtask), so I can't confirm the current pass/fail state beyond the build passing.

## Interactivity today: copy buttons, tabs, search, diagrams, live examples, the Paper theme

### Takeaway
More interactive than a stock Starlight site:
- Synced TS/Rust/Result tabs.
- Page pictures of every generated PDF, with a download link.
- 13 inline-SVG diagrams that follow the theme, each with a text description.
- "Edit and run" (CodeMirror with a sandboxed Web Worker) on six TypeScript examples across nine pages.
- Pagefind search, which runs locally.

The Paper theme is a CSS-variable remap of Starlight plus a shared site header; the layout is Starlight's.

### Cited Findings
- **Copy buttons** come from Expressive Code, Starlight's default. Playwright counted copy buttons equal to code blocks (first-pdf 3/3, save-a-pdf 2/2, TypeDoc Document 5/5) — local build, Playwright Chromium 1243 run on 2026-11-11.
- **Tabs:** `<Tabs syncKey="lang">` syncs the TypeScript/Rust choice across the site through localStorage. The Result/Output tab is deliberately never remembered — [apps/docs/src/components/Example.astro](../../../../apps/docs/src/components/Example.astro)
- **Result pictures:**
  - 38 PNGs in `public/examples/preview/` (3.5 MB in dist).
  - The example PDFs themselves in `/examples/pdf/` (296 KB).
  - The plan: rendered "at 144 dpi… by the render tooling in `tools/render` (hayro)", committed with a manifest of each golden's SHA-256 that `cargo xtask check docs` compares.
  
  — local `dist/`; [docs/plans/docs-examples.md](../../../../docs/plans/docs-examples.md) §2
- **Edit and run** (B7e, done):
  - Enabled for six examples in `runner/examples.ts`'s `RUNNABLE`: hello, invoice, images-and-vectors, links-and-outlines, element-flow, pdf-ua-report.
  - They appear on 9 pages: 6 recipes and 3 guides (accessible-pdfs, images-and-vectors, links-outlines-and-metadata).
  - The Node quick-start file isn't runnable, so "Your first PDF" has no Edit and run.
  - Pressing the button lazy-loads `runner/editor.ts`: CodeMirror 6, with Sucrase stripping types and no type check.
  - The code runs in a module worker at `/run/worker.js`, with Run, Reset and "Download PDF" buttons.
  - The runner is built only when `cargo xtask wasm` output exists. Otherwise "the pages show their examples without 'Edit and run'".
  
  — [apps/docs/runner/examples.ts](../../../../apps/docs/runner/examples.ts), [apps/docs/src/components/Example.astro](../../../../apps/docs/src/components/Example.astro), [apps/docs/scripts/build-runner.mjs](../../../../apps/docs/scripts/build-runner.mjs), [docs/todos/done/B7e-docs-edit-and-run.md](../../../../docs/todos/done/B7e-docs-edit-and-run.md)
- **Diagrams** (Round Q3, Q3a–Q3g all done):
  - 13 diagram names, each used once on 10 pages: open-edit-save, save-modes, save-decision, package-map, package-map-modules, page-space, extract-pipeline, reading-order, worker-calls, errors-and-warnings, undo-timeline, drawing-pages, range-loading.
  - Each is inline SVG written by an Astro component from a kit (`Arrow`, `Box`, `Lane`, `PageSheet`, `Strip`…), coloured only through `--paper-*` tokens, with caption and text in `registry.mjs`. The Markdown twins read the same text.
  - The plan's goal line ("The docs site has no images at all") describes the state before Q3.
  
  — [apps/docs/src/components/diagrams/registry.mjs](../../../../apps/docs/src/components/diagrams/registry.mjs), [docs/plans/docs-diagrams.md](../../../../docs/plans/docs-diagrams.md)
- **Search** is Starlight's Pagefind, built at build time. The build log says "Found 386 HTML files", and the index is 2.6 MB under `dist/pagefind/`, loaded in fragments. The CSP's `connect-src 'self'` notes "search fetches its own index; nothing remote" — local build log; [apps/docs/public/_headers](../../../../apps/docs/public/_headers)
- **Header override** (`Header.astro`, 42 lines):
  - The shared `@laminapdf/site-kit` SiteHeader (wordmark to laminapdf.com, one nav list with "Docs" marked).
  - A version link: the release's version linked to the changelog, or "Unreleased" linked to the released docs.
  - Starlight's Search, SocialIcons (only with `LAMINA_PDF_DOCS_GITHUB=1`), ThemeSelect (a Paper select) and LanguageSelect.
  - Other overrides: `Banner` (the unreleased-changes banner), `MobileMenuFooter`, `ThemeSelect`.
  
  — [apps/docs/src/components/Header.astro](../../../../apps/docs/src/components/Header.astro), [apps/docs/astro.config.mjs](../../../../apps/docs/astro.config.mjs)
- **Paper theme:**
  - `paper.css` (301 lines) maps Starlight's `--sl-color-*` variables onto `--paper-*` tokens from `@laminapdf/tokens`: "warm paper grounds, ink text, hairline rules, one deep teal accent, and one sans-serif face for all text, titles included… Lamina Sans, which is Source Sans 3 renamed".
  - Aside colours map to accent, Plum, warning and danger, with WCAG ratios noted in comments.
  - `diagrams.css` is 217 lines.
  - Fonts are self-hosted through `@laminapdf/fonts`; one 30.6 KB font loads on a normal page.
  
  — [apps/docs/src/styles/paper.css](../../../../apps/docs/src/styles/paper.css); Playwright network log
- **No feedback widget, analytics, "copy page as Markdown" button, AI chat or version switcher** were found in the components or config. The only version distinction is docs.laminapdf.com (release) vs next.docs.laminapdf.com (main) — [apps/docs/src/components/](../../../../apps/docs/src/components/), [apps/docs/astro.config.mjs](../../../../apps/docs/astro.config.mjs)
- **Tests of the interactive parts:** `e2e/runner.e2e.test.ts` and `e2e/theme.e2e.test.ts` (Vitest + Playwright); `test/browser/diagrams.browser.test.ts`; `test/diagrams.test.mjs` (no diagram before the opening paragraph) — [apps/docs/e2e/](../../../../apps/docs/e2e/), [apps/docs/test/](../../../../apps/docs/test/)

### Inferences
- The site already has the "see the result" and "edit and run" patterns that the best docs (MDN live samples, Svelte/Vue playgrounds) use. The redesign's gap is less about adding interactivity and more about:
  - Coverage: Edit and run on 6 of about 22 TS examples, none in the first tutorial.
  - Discoverability: Result sits behind a tab, and Edit and run is inside the Result tab.
- With layout and components stock Starlight, the redesign's room is in information architecture, the home page, and the components around examples; the colours and fonts are already Paper.

### Gaps
- I did not test Edit and run in the browser: the local build lacked `packages/wasm/dist`, so the runner wasn't built. Its behaviour comes from source and the B7e todo only.

## Plans already on record about the docs site

### Takeaway
Nearly all docs-site work on record is finished:
- B7a–e: site, domain, Result tabs, Edit and run.
- Q1a–m: story, prose checks, examples.
- Q3a–g: 12+ diagrams.
- L5a: agent docs.
- P4: release vs next docs.
- L4f: CLI section.
- U6g: Paper.
- T1b–d: proof pages.

What's still open is about discoverability and agents (W3a–W3i, L5b), plus T4j, which would add TypeScript flow documents. No open todo proposes a redesign of the site's IA or look.

### Cited Findings
- **Done todos naming the docs site** include:
  - B7a (docs site), B7c (custom domain), B7d (example results), B7e (edit and run).
  - L4f (command-line docs), L5a (agent docs), P4 (docs match the release).
  - Q1a (Source Sans font), Q1k (internal IDs out of reader text), U6g (docs site in Paper).
  - T1b (how we test page), T1c (publish performance numbers), T1d (snapshot-testing guide).
  - X1 (link checker), W1d (one site header).
  
  — [docs/todos/done/](../../../../docs/todos/done/)
- **Q1a–Q1m and Q3a–Q3g are all in `done/`.** The Q1 todos: docs-font-source-sans, docs-prose-check, examples-checks, example-api-cleanups, quick-start-apps, examples-readable, worker-examples, docs-story, guides-create, recipes-and-troubleshooting, internal-ids-out-of-reader-text, docs-checks-strict, guides-read-and-edit. The Q3 todos: diagram-kit-and-first-diagram, save-diagrams, coordinates-diagram, package-map-diagram, extraction-diagrams, worker-errors-undo-diagrams, drawing-and-ranges-pages — `ls docs/todos/done` ([docs/todos/done/](../../../../docs/todos/done/))
- **Plans** (all marked "Status: Draft" in their headers, although their todos shipped):
  - `docs/plans/quality.md` (Round Q: "docs people enjoy reading, and code that stays easy to change"; §3 is the story sidebar).
  - `docs/plans/docs-examples.md` (2026-11-08: Result tabs for every example that makes a document, Edit and run for "the few TypeScript examples worth editing", with a per-example table).
  - `docs/plans/docs-diagrams.md` (Round Q3: twelve diagrams, inline SVG through tokens, works in forced colours).
  
  — [docs/plans/quality.md](../../../../docs/plans/quality.md), [docs/plans/docs-examples.md](../../../../docs/plans/docs-examples.md), [docs/plans/docs-diagrams.md](../../../../docs/plans/docs-diagrams.md)
- **Docs research:** `docs/research/docs-site/live-examples.md` compared Android-style pictures, MDN live samples, hosted sandboxes (TanStack, StackBlitz), a hosted bundler (React, Sandpack) and a playground of one's own (Svelte, Vue). It recommended "a Result tab with page pictures now and a runner of our own for a few TypeScript examples later", and both shipped — [docs/research/docs-site/README.md](../../../../docs/research/docs-site/README.md)
- **Open (ready) todos touching the docs:**
  - **W3h:** "Docs titles and descriptions that match the questions people ask". Many titles are feature names ("Save a PDF") rather than questions ("Edit a PDF in TypeScript without breaking signatures"). Depends on W3e and W3g.
  - **W3g:** a "Choosing a PDF library" docs page with sourced comparison tables per language.
  - **W3f:** a Context7 listing (`context7.json`) and llms.txt linked from the website and README. "Nothing on the website or in the README links llms.txt".
  - **W3a:** crawl and index rules. **W3d:** structured data and share tags. **W3b:** public repository links (would turn on GitHub links). **W3e, W3i:** search-engine registration, answer-engine check.
  - **L5b:** run the agent eval with and without the shipped docs.
  - **T4j:** typed flow documents and `builder.flow` in TypeScript (would let the flowing-pages and invoice-from-json content gain TS twins).
  
  — [docs/todos/ready/W3h-docs-titles-for-questions.md](../../../../docs/todos/ready/W3h-docs-titles-for-questions.md), [docs/todos/ready/W3g-choosing-a-pdf-library.md](../../../../docs/todos/ready/W3g-choosing-a-pdf-library.md), [docs/todos/ready/W3f-agent-discovery.md](../../../../docs/todos/ready/W3f-agent-discovery.md), [docs/todos/ready/L5b-agent-eval-baseline.md](../../../../docs/todos/ready/L5b-agent-eval-baseline.md), [docs/todos/done/T4j-documents-in-typescript.md](../../../../docs/todos/done/T4j-documents-in-typescript.md)
- **Ongoing:** only W2a, the `lamina-story` social-posts skill, which isn't docs-site work — [docs/todos/ongoing/W2a-story-skill-and-coverage-map.md](../../../../docs/todos/ongoing/W2a-story-skill-and-coverage-map.md)
- **Recent git history:**
  - `git log origin/main -- apps/docs` shows 35 commits, dated 2026-11-06 to 2026-11-11, 21 of them on 2026-11-08. The repository's visible history appears to start recently, so this may not be the full history.
  - Recent docs changes: "a Command line section on the docs site (#415)"; "a Paper select for the sites, and the theme switch on it (#361)"; "the Edit and run editor in the site's Paper tokens (#350)"; "an example's tabs in one row (#343)"; diagrams (#331, #335, #337, #339, #341, #346); "edit and run the TypeScript examples on their pages (#332)"; "show each example's result beside its code (B7d) (#330)"; "a benchmarks page… (#310)"; "a guide and examples for testing the PDFs you generate… (#295)".
  
  — `git log --oneline origin/main -- apps/docs`

### Inferences
- A redesign would be the first docs work since Rounds Q and B7 that isn't driven by an existing todo. Per CLAUDE.md, it would need a new todo, and probably a plan in `docs/plans/`.
- The W3h retitling and the W3g comparison page overlap with any IA redesign and should be folded in or sequenced with it.

### Gaps
- The three plans still say "Draft" although their todos shipped. I found no record of whether that is intentional.

## Constraints a redesign must respect

### Takeaway
Hard constraints:
- A strict CSP with no third-party origins: no CDN fonts, scripts, analytics or embeds; `connect-src 'self'`.
- Static Cloudflare Workers assets with no server code.
- Separate release and next deploys.
- Agent docs (Markdown twins, llms.txt, llms-full.txt, the package docs folder) generated from the same pages and the sidebar.
- Prose, snippet and diagram checks that fail the build or PR.
- An explicit sidebar.

### Cited Findings
- **Docs CSP** (`public/_headers`): `default-src 'none'; script-src 'self' 'wasm-unsafe-eval' 'inline-script-hashes'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; worker-src 'self'; … frame-ancestors 'self'; base-uri 'none'; form-action 'none'; object-src 'none'`.
  - `'inline-script-hashes'` is replaced after the build by `@laminapdf/site-kit`.
  - `style-src` allows `'unsafe-inline'` only "because Expressive Code colours code tokens with style attributes".
  - `/run/*` gets its own CSP with `'unsafe-eval'` for the example runner.
  - Other headers: `Referrer-Policy: no-referrer`, a restrictive `Permissions-Policy`, HSTS, `X-Frame-Options: SAMEORIGIN` and `Cross-Origin-Resource-Policy: same-origin`.
  
  — [apps/docs/public/_headers](../../../../apps/docs/public/_headers)
- **ADR 0011:**
  - `docs.laminapdf.com` serves "the docs site… for the latest release" (Worker `lamina-pdf-docs`, deployed on each release, with a preview per PR).
  - `next.docs.laminapdf.com` serves main's docs, "never indexed".
  - Its implementation note of 2026-11-08 records the runner's separate CSP: "The code it runs is one of the docs' own examples, edited in the reader's own tab; nothing is sent anywhere."
  - ADR 0011 is also the source of the opt-in-only telemetry rule (CLAUDE.md summary).
  
  — [docs/decisions/0011-cloudflare-topology-and-privacy.md](../../../../docs/decisions/0011-cloudflare-topology-and-privacy.md)
- **ADR 0015** sets the monorepo layout (apps under `apps/`, packages as `@laminapdf/*`, the docs app is `@laminapdf/docs`) and Astro + Starlight for the docs site, per the CLAUDE.md summary — [docs/decisions/0015-monorepo-layout-and-tooling.md](../../../../docs/decisions/0015-monorepo-layout-and-tooling.md)
- **Deploy** (`wrangler.toml`):
  - Worker `lamina-pdf-docs` on custom domain `docs.laminapdf.com`, with `workers_dev` and `preview_urls` on for per-PR previews.
  - `[assets] directory = "./dist"`, `not_found_handling = "404-page"`, `html_handling = "auto-trailing-slash"`, `[observability] enabled = false`.
  - `[env.next]` is `lamina-pdf-docs-next` on `next.docs.laminapdf.com`.
  - "no Worker script, every request answered by the asset server". Redirects must be static pages, because "Workers static assets ignore _redirects".
  
  — [apps/docs/wrangler.toml](../../../../apps/docs/wrangler.toml), [apps/docs/astro.config.mjs](../../../../apps/docs/astro.config.mjs)
- **Workflow:** `.github/workflows/docs.yml` has three jobs:
  - "Docs preview (build)" and "Docs preview (deploy)" for PRs. Deploy checks out the default branch's deploy configuration, runs `wrangler versions upload --preview-alias pr-<n>` and comments the URL.
  - "Docs next" on pushes to main.
  - Production from a release tag via `workflow_call`/`workflow_dispatch` with a `dry-run` input.
  - Each build first runs `cargo xtask web docs-runner` ("Build the example runner's WebAssembly"), then `pnpm --filter @laminapdf/docs build`.
  
  — [.github/workflows/docs.yml](../../../../.github/workflows/docs.yml)
- **Build-time behaviour** (`astro.config.mjs` and its plugins):
  - `LAMINA_PDF_ENV=production` names the version in the header. Other builds show the "unreleased changes" banner and are noindex (`plugins/release.mjs`, `previewNoindex`).
  - `starlight-links-validator` runs with `errorOnLocalLinks: true`, plus `checkedLinks` for example PDFs and code pages.
  - `cliDownloadChecks` and `examplePdfs` run.
  - `agentDocs(sidebarRoutes)` writes a Markdown twin per page, `llms.txt` and `llms-full.txt` in sidebar order.
  - `releaseChecks` runs last.
  
  — [apps/docs/astro.config.mjs](../../../../apps/docs/astro.config.mjs)
- **Agent docs output** (local build): `llms.txt` is 423 lines (52 KB) and `llms-full.txt` 748 KB, with 379 `.md` twins in dist. The `llms.txt` header reads "These docs describe version 0.2.0-alpha.6. Every link is a Markdown page". Diagram text is shared with the twins through `registry.mjs`, so "the picture, its description for screen readers and the twin that agents read never drift apart" — local `dist/`; [apps/docs/src/components/diagrams/registry.mjs](../../../../apps/docs/src/components/diagrams/registry.mjs)
- **Package-shipped docs:** `node apps/docs/scripts/package-docs.mjs <out-dir> [--api]` writes the `docs/` folder that `@laminapdf/sdk` (prepack) and the `laminapdf` crate ship: pages, error pages, `AGENTS.md`, `llms.txt`, `renames.json` and schemas. The coding-agents page tells agents to prefer this copy — [apps/docs/src/content/docs/start/coding-agents.mdx](../../../../apps/docs/src/content/docs/start/coding-agents.mdx), [CLAUDE.md](../../../../CLAUDE.md)
- **Checks:**
  - `cargo xtask check docs` runs docs-prose and docs-snippets with `--all-strict`, plus links, todos, codes and renames.
  - The docs `test` script runs `plugins/`, `scripts/`, `runner/` and `test/` node tests.
  - `check:api-docs` and the e2e/browser suites are separate.
  - The word lists live in `tools/xtask/docs-prose.toml`; the TS/Rust pairs and test-example exceptions in `tools/xtask/docs-snippets.toml`.
  
  — [docs/guidelines/docs-writing.md](../../../../docs/guidelines/docs-writing.md), [docs/guidelines/examples.md](../../../../docs/guidelines/examples.md), [apps/docs/package.json](../../../../apps/docs/package.json)
- **Pinned dependency versions:** `@astrojs/starlight` 0.42.8, `astro` 7.4.5, `starlight-typedoc` 0.23.2, `typedoc` 0.28.20, `starlight-links-validator` 0.27.0, `codemirror` 6.0.2, `sucrase` 3.35.1 — [apps/docs/package.json](../../../../apps/docs/package.json)

### Inferences
- Common best-practice features from hosted docs platforms are ruled out by the CSP and the privacy guarantees as configured today:
  - Hosted search such as Algolia DocSearch (needs a remote `connect-src`).
  - Hosted AI chat widgets.
  - Third-party analytics or feedback widgets.
  - Embedded StackBlitz or CodeSandbox iframes.
  
  Equivalents would have to be self-hosted and same-origin, as Pagefind and the runner already are.
- Any new page type or IA change has to keep llms.txt, the twins and the package docs coherent, since all three derive from the sidebar and the `.mdx` sources.

### Gaps
- I did not read ADR 0015 in full; the Starlight choice is taken from CLAUDE.md's summary of it.

## Measurements from a local build

### Takeaway
The site builds without the WebAssembly in about 15 s (380 pages; 39 MB `dist`, of which 24 MB is `reference/`). A typical page loads about 250–280 KB uncompressed, about 60–75 KB gzip, before images:
- JS: about 103–110 KB raw.
- CSS: about 100–120 KB raw.
- One 30.6 KB font.

The HTML is heavy on example pages (129–180 KB raw) because the highlighted code for both languages is inlined. The Edit and run editor bundle is 505 KB (166 KB gzip) but loads only on demand. Pages render quickly locally, with no horizontal scroll at phone width.

### Cited Findings
- **Build:**
  - `pnpm install --frozen-lockfile` took 17 s.
  - `pnpm --filter @laminapdf/docs build` took 15.5 s wall time ("380 page(s) built in 12.60s").
  - It ran on Node 22.23.4 without `cargo xtask wasm`. The runner step printed "docs: no example runner (missing packages/wasm/dist/lamina_pdf_text.wasm, packages/sdk/dist/index.js, …; run `cargo xtask wasm` first)".
  - TypeDoc ran; there was one Vite chunk-size warning; "All internal links are valid."
  
  — local build log, 2026-11-11
- **`dist/` sizes:** 39 MB in total. `reference/` 24 MB, `examples/` 3.8 MB (preview PNGs 3.5 MB), `pagefind/` 2.6 MB, `recipes/` 2.6 MB, `guides/` 2.2 MB, `_astro/` 1.1 MB, `llms-full.txt` 748 KB, `cli/` 632 KB — `du -sh` on local `dist/`
- **Per-page transfer** (Playwright on Chromium 1243, served uncompressed by `python3 -m http.server`, so figures are raw bytes):

  | Page | HTML | CSS (files) | JS (files) | Fonts |
  | --- | --- | --- | --- | --- |
  | `/` (home, splash) | 17.8 KB | 101.4 KB (2) | 103.1 KB (4) | 30.6 KB |
  | `/start/first-pdf/` | 90.9 KB | 119.7 KB (3) | 109.6 KB (9) | 30.6 KB |
  | `/guides/save-a-pdf/` | 128.8 KB | 119.7 KB (3) | 109.6 KB (9) | 30.6 KB |
  | `/recipes/invoice/` | 179.7 KB | 119.7 KB (3) | 109.6 KB (9) | 30.6 KB |
  | `/reference/codes/wrong-password/` | 63.8 KB | 119.7 KB (3) | 108.2 KB (8) | 30.6 KB |
  | `/reference/sdk/classes/document/` (TypeDoc) | 239.4 KB | 119.7 KB (3) | 108.2 KB (8) | 61.2 KB (2) |

  Same figures at 1280×800 and 390×844 — measure script in the scratchpad; local `dist/`
- **Gzip sizes** of the main assets (`gzip -9`):
  - `middleware.*.css` 98.1 KB → 17.4 KB (Starlight + Paper + tokens + fonts CSS).
  - `ec.*.css` 18.3 KB → 3.9 KB (Expressive Code).
  - `ui-core.*.js` 97.2 KB → 26.6 KB, imported by the Search component on every page.
  - `editor.*.js` 504.7 KB → 165.8 KB (Edit and run, dynamic import).
  - Every other chunk is under 2.6 KB.
  - Home HTML 17.8 KB → 6.0 KB; save-a-pdf HTML 128.8 KB → 19.2 KB.
  
  — local `dist/_astro/`
- **Timing and layout:**
  - First contentful paint was 36–136 ms over localhost for every sampled page, and no sampled page scrolled horizontally at 390 px.
  - DOM counts: first-pdf has 3 code blocks, 1 tab row and 1 lazy image; save-a-pdf has 2 code blocks, 1 tab row and 4 SVG figures; invoice has 2 code blocks and 1 tab row.
  
  — Playwright run, 2026-11-11

### Inferences
- About 100 KB of JS raw (about 27 KB gzip) on every page is mostly Pagefind's search UI. It is the biggest fixed cost, and it is in line with stock Starlight.
- Inlining both languages' highlighted code makes example pages' HTML large (up to 180 KB raw, about 23 KB gzip for the invoice). That is acceptable gzipped, but the HTML grows with each tab added.
- Localhost timings say nothing about real network performance; they only show that no page does heavy client work at load.

### Gaps
- Lighthouse was not run: it is not installed, and I didn't add it. The Playwright metrics above are a partial substitute; no CLS, LCP or TBT, and no axe accessibility scan.
- No measurement with the runner built: the `cargo xtask wasm` build was skipped. The runner's WebAssembly and inputs under `/run/assets/` are therefore unmeasured. They load only after "Edit and run".
- No production (Cloudflare, Brotli or gzip) transfer sizes were measured; the live site wasn't fetched.
