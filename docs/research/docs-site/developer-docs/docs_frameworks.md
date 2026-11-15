# Docs frameworks and platforms (as of 11 November 2026): stay on Starlight, customise it, or switch?

Context read first: `apps/docs/README.md`, `apps/docs/astro.config.mjs`, `docs/decisions/0011-cloudflare-topology-and-privacy.md`. The site pins `astro` 7.4.2, `@astrojs/starlight` 0.42.7, `starlight-typedoc` 0.23.1, `starlight-links-validator` 0.27.0, `typedoc` 0.28.20 (`apps/docs/package.json`). It overrides three Starlight components (`Banner`, `Header`, `MobileMenuFooter`), adds Paper CSS through `customCss`, and runs its own Astro integrations: the Markdown twins and llms.txt (`agentDocs`), release checks, link checks and CSP inline-script hashes. The docs README says the docs CSP is **not** fully strict today. `script-src` uses hashes plus `'wasm-unsafe-eval'` (Pagefind), but **`style-src` allows `'unsafe-inline'` because Expressive Code colours tokens with style attributes**. The website (`apps/site`) does have `style-src 'self'`, because it sets `build.inlineStylesheets: "never"` and uses no `style` attributes (ADR 0011 §4).

Method notes: npm downloads and versions come from the npm registry API, fetched on 2026-11-11 for the week 2026-11-04 to 2026-11-10. PyPI numbers are from pypistats.org and crates.io numbers from the crates.io API, both fetched 2026-11-11. GitHub star counts came through the ungh.cc mirror of the GitHub API, because the session's GitHub API access was blocked; treat them as approximate. I measured the JS-per-page figures myself; see the method under Q3.

## Q1. Starlight in late 2026: version, features, limits, adoption, cadence, health under Cloudflare

### Takeaway
Starlight is healthy and widely used: release 0.42.10 (6 Nov 2026), ~2.18 M npm downloads a week (more than Docusaurus or VitePress), and a minor release every one to three months. It is still 0.x, and versioning, OpenAPI, TypeDoc and llms.txt come only from community plugins. Astro's team joined Cloudflare on **16 January 2026, not in 2025**, and Astro stays MIT and platform-neutral.

### Cited Findings
- Latest `@astrojs/starlight` is 0.42.10, published 2026-11-06. The repo pins 0.42.7, three patches behind. — [npm registry](https://registry.npmjs.org/@astrojs/starlight)
- Weekly downloads of `@astrojs/starlight` (2026-11-04 to 11-10): 2,176,426. `astro` itself: 11,745,189. — [npm downloads API](https://api.npmjs.org/downloads/point/last-week/@astrojs/starlight), [astro](https://api.npmjs.org/downloads/point/last-week/astro)
- Latest `astro` is 7.4.11 (2026-11-11). Major release dates: 5.0.0 on 2024-12-03, 6.0.0 on 2026-03-10, 7.0.0 on 2026-06-22, so two majors in 2026. — [npm registry](https://registry.npmjs.org/astro)
- Starlight minor releases (each 0.x minor may break things): 0.31 (2025-01-13), 0.32 (2025-02-15), 0.33 (2025-04-07), 0.34 (2025-04-16), 0.35 (2025-07-16), 0.36 (2025-09-19), 0.37 (2025-11-28), 0.38 (2026-03-11), 0.39 (2026-05-07), 0.40 (2026-06-09), 0.41 (2026-06-23), 0.42 (2026-09-02). There were 43 stable versions in the 12 months to 2026-11-11. First published 2023-05-08. — [npm registry](https://registry.npmjs.org/@astrojs/starlight)
- GitHub: withastro/starlight has about 9.5 k stars and 1.09 k forks, last pushed 2026-11-10. withastro/astro has about 63.9 k stars. — [ungh.cc mirror of GitHub API](https://ungh.cc/repos/withastro/starlight)
- Astro and Cloudflare: Cloudflare announced on **16 January 2026** that The Astro Technology Company team is joining Cloudflare. All full-time Astro employees became Cloudflare employees, Astro stays free, open source and MIT-licensed, and the team will keep supporting all deploy targets, not just Cloudflare. The announcement came with the Astro 6 beta. — [Cloudflare press release](https://www.cloudflare.com/press/press-releases/2026/cloudflare-acquires-astro-to-accelerate-the-future-of-high-performance-web-development/), [Astro blog](https://astro.build/blog/joining-cloudflare/), [The New Stack](https://thenewstack.io/cloudflare-acquires-team-behind-open-source-framework-astro/)
- Plugins for things Starlight doesn't do itself are **all community-maintained**. The official plugins page lists `starlight-versions` (versioning), `starlight-typedoc`, `starlight-openapi`, `starlight-llms-txt`, `starlight-blog`, `starlight-links-validator`, `starlight-image-zoom` and `starlight-sidebar-topics`. Its only official plugin is Algolia DocSearch. The page says nothing about built-in versioning. — [Starlight plugins and integrations](https://starlight.astro.build/resources/plugins/)
- `HiDeoo/starlight-versions` has about 102 stars and was last pushed 2026-10-30. The plugin is small and maintained by one person. — [ungh.cc](https://ungh.cc/repos/HiDeoo/starlight-versions)
- Expressive Code (Starlight's code-block engine) has about 969 stars and was last pushed 2026-10-27. — [ungh.cc](https://ungh.cc/repos/expressive-code/expressive-code)
- Built-in features, from Starlight's docs (I did not re-fetch them this session):
  - [Pagefind](https://starlight.astro.build/guides/site-search/) search;
  - [i18n](https://starlight.astro.build/guides/i18n/);
  - [component overrides](https://starlight.astro.build/guides/overriding-components/);
  - [route-data middleware](https://starlight.astro.build/guides/route-data/), for changing a page's data before it renders;
  - [custom pages wrapped in `<StarlightPage>`](https://starlight.astro.build/guides/pages/), plus the `splash` template;
  - [asides and tabs (`syncKey` syncs tabs across a page and between pages)](https://starlight.astro.build/components/tabs/);
  - [Expressive Code blocks](https://starlight.astro.build/guides/authoring-content/#code-blocks).
  Treat the exact feature names as needing a check against the current docs.
- Opinion, from low-authority aggregators: Starlight is usually picked for a standalone, content-first docs site; its drawbacks are that it is built for docs, not marketing pages; Docusaurus is the stronger pick when versioning is the priority. — [context7 Docusaurus alternatives](https://context7.com/docusaurus-alternatives), [skillbench](https://skillbench.vercel.app/solutions/astro-starlight)

### Inferences
- Health is not a worry. Cloudflare employs the Astro team, Starlight ships monthly, and it is now the most-downloaded docs-specific framework on npm after fumadocs-core. Note that `fumadocs-core` downloads include its use as a library. This site deploys to Cloudflare Workers, which lines up with Astro's new owner. That should make deployment friction lower, never higher.
- The real risk is churn: Starlight is still 0.x and Astro shipped two majors in 2026. Each Starlight minor can break overrides, so three overridden components plus custom integrations mean regular upgrade work.
- Versioning is the biggest feature gap. It matters little here: ADR 0017 already deploys `docs.laminapdf.com` (release) and `next.docs.laminapdf.com` (main) as separate Workers. That two-channel scheme is a form of versioning that needs no plugin.

### Gaps
- I could not fetch Starlight's GitHub releases or issues directly; GitHub access was blocked. Open issues on layout limits and versioning could not be counted.
- I found no official list of notable Starlight users with dates. Cloudflare's own docs, Astro's docs and Biome's docs are commonly cited, but I did not verify any of them in this session.

## Q2. The alternatives, compared on the stated axes

### Takeaway
Only self-hosted static generators can meet ADR 0011's "no third-party origins". Of those, Starlight/Astro, VitePress, Rspress, mdBook, Sphinx and Zensical/MkDocs export plain static files. Fumadocs and Nextra are React/Next.js apps with heavy hydration. Docusaurus is a React SPA (single-page app) whose v4 is still in canaries. Hosted platforms (Mintlify, GitBook, ReadMe) put documents and analytics through a third party, so they fail the constraint as products. Material for MkDocs reached the end of its announced maintenance window on 5 November 2026.

### Cited Findings

**Adoption snapshot (npm downloads per week, 2026-11-04 to 11-10; latest version and date; stars via ungh.cc):**

| Framework | Downloads per week | Latest version (date) | Stars |
| --- | --- | --- | --- |
| Starlight | 2,176,426 | 0.42.10 (2026-11-06) | ~9.5 k |
| Docusaurus (`@docusaurus/core`) | 1,958,403 | 3.10.2 (2026-07-10); canary `4.0.0-canary-7078` (2026-11-07) | ~66.7 k (repo now under `meta/docusaurus`) |
| VitePress | 1,978,351 | 1.6.4 (2025-08-05); `next` 2.0.0-alpha.21 (2026-10-25) | ~18.5 k |
| Nextra | 308,681 | 4.6.1 (2025-12-04) | ~14.0 k; repo last pushed 2026-07-31 |
| Fumadocs (`fumadocs-core`) | 2,980,654 | 16.18.4 (2026-11-09) | ~13.6 k |
| Rspress (`@rspress/core`) | 109,036 | 2.1.2 (2026-11-11) | ~2.4 k |
| Zudoku | 37,327 | 0.93.2 (2026-10-30) | ~0.6 k |
| Scalar (`@scalar/api-reference`) | 2,322,361 | 1.78.0 (2026-11-10) | ~16.4 k |

Sources: [npm downloads API](https://api.npmjs.org/downloads/point/last-week/@docusaurus/core) (same endpoint per package), [npm registry](https://registry.npmjs.org/vitepress), [ungh.cc](https://ungh.cc/repos/facebook/docusaurus).

**Python and Rust generators:**
- mkdocs-material 9.8.1 (2026-11-07), 4.03 M downloads a week. mkdocs 1.6.1, last released **2024-08-30**, 4.44 M a week. sphinx 9.2.0 (2026-10-21), 4.66 M a week. zensical 0.1.1 (2026-11-10), 507 k a week. — [PyPI](https://pypi.org/pypi/mkdocs-material/json), [pypistats](https://pypistats.org/api/packages/mkdocs-material/recent)
- mdBook 0.5.4 (2026-07-06), 2.54 M recent downloads on crates.io, about 22.3 k stars. — [crates.io API](https://crates.io/api/v1/crates/mdbook), [ungh.cc](https://ungh.cc/repos/rust-lang/mdBook)

**Docusaurus:**
- 3.10 is announced as the last v3.x release and adds opt-in v4 future flags. The release-process page says v4 will ship from `main`, with a `docusaurus-v3` branch kept for security fixes for three months after the new stable release. I found no v4 release date. — [Docusaurus 3.10 blog](https://www.docusaurus.io/zh-CN/blog/releases/3.10), [release process](https://docusaurus.io/community/release-process)
- Docusaurus 3.9 added AI search (InfoQ coverage). — [InfoQ](https://www.infoq.com/news/2025/10/docusaurus-3-9-ai-search)
- Docusaurus has built-in docs versioning; aggregator opinion calls it the strongest choice for versioned docs. — [context7](https://context7.com/docusaurus-alternatives)
- The official Astro guide covers migrating from Docusaurus. — [Astro docs](https://docs.astro.build/en/guides/migrate-to-astro/from-docusaurus/)

**VitePress:** 1.x has not had a stable release since August 2025; 2.0 has been in alpha all through 2026 (alpha.19 on 2026-08-02, alpha.20 on 2026-09-04, alpha.21 on 2026-10-25). — [npm registry](https://registry.npmjs.org/vitepress)

**Fumadocs:**
- Runs on Next.js (recommended), React Router, TanStack Start, Waku and Astro, where MDX renders as React islands.
- React Router and TanStack Start in SPA mode can be hosted statically on a CDN.
- llms.txt is mentioned for Fumapress, a separate Waku-based generator.
- Fumadocs' own guide says a migration from Docusaurus needs more changes because the architectures differ.

— [Fumadocs docs](https://fumadocs.dev/docs); the Fumadocs migration guide was read on a Mintlify mirror that has since gone, with no archived copy

**Nextra:** The last npm release is 4.6.1 (2025-12-04) and the repo was last pushed 2026-07-31: nearly a year with no release. — [npm](https://registry.npmjs.org/nextra), [ungh.cc](https://ungh.cc/repos/shuding/nextra)

**Material for MkDocs / Zensical:**
- On 5 November 2025 Material for MkDocs entered maintenance mode, with critical bug and security fixes promised **through 5 November 2026**. That window has now closed.
- Its team builds Zensical instead. Zensical is MIT-licensed, reads `mkdocs.yml` natively and is still closing the feature gap.
- Insiders features became free in 9.7.0.
- MkDocs itself (1.6.1) has had no release since August 2024.

— [mkdocs-material issue #8523](https://github.com/squidfunk/mkdocs-material/issues/8523), [Insiders now free (blog, 2025-11-11)](https://squidfunk.github.io/mkdocs-material/blog/2025/11/11/insiders-now-free-for-everyone/), [PyPI mkdocs](https://pypi.org/pypi/mkdocs/json)

**Mintlify** (SaaS):
- Starter is $0 a month (5 editors; hosting, search, custom domain, Git sync, MCP server, API playground). Pro is $450 a month billed annually ($540 billed monthly); it adds the AI assistant at 25 credits an answer, an agent, preview deployments and 10,000 credits a month. Enterprise adds SSO, an SLA and EU hosting.
- Self-hosting is "available" on Enterprise only.
- Content lives as MDX in your Git repo.

— [Mintlify pricing](https://www.mintlify.com/pricing)

**GitBook** (SaaS):
- Plans per site per month, billed annually: Free; Essential at $65 + $12 a user; Ultimate at $249 + $12 a user; Enterprise at a custom price.
- Every plan gets Markdown versions of pages, llms.txt and llms-full.txt, and an auto-generated MCP server.
- Custom domains start at Essential; a custom subdirectory needs Ultimate.
- Its pricing page says you can self-host published docs by cloning its open-source repo, with no support from GitBook.

— [GitBook pricing](https://www.gitbook.com/pricing)

**CSP with Astro:** Astro 6 made CSP stable. Options include `security.csp.scriptDirective.hashes` and `security.csp.styleDirective.hashes`. Astro sends the policy as a `<meta http-equiv>` element, which works in build and preview but not in dev. — [Report URI's Astro docs](https://docs.report-uri.com/frameworks/astro/), [Astro v5 experimental CSP reference](https://v5.docs.astro.build/en/reference/experimental-flags/csp/)

**Syntax highlighting and CSP:** Highlighters such as Shiki and astro-expressive-code emit `style` attributes on tokens, which a strict `style-src` blocks. The usual workaround keeps `'unsafe-inline'` for styles. — [astro-shield issue (polar.sh)](https://github.com/kindspells/astro-shield/issues/140)

**CSP mechanics:** Inline `style=` attributes fall under `style-src-attr`. Once a hash or nonce is present, browsers ignore `'unsafe-inline'`. CSP Level 3's `'unsafe-hashes'` can allow specific attribute values. — [MDN style-src](https://developer.mozilla.org/docs/Web/HTTP/Headers/Content-Security-Policy/style-src)

### Inferences
- **Viable under ADR 0011:** Starlight; a custom Astro site; VitePress (Vue; stable line stalled, 2.0 in alpha); Rspress (React, Rsbuild); mdBook (Rust, but poor theming and no MDX islands); Sphinx; Zensical (very young). Docusaurus and Fumadocs can also be self-hosted static, but they hydrate a React app on every page; see Q3.
- **Not viable as products:** Mintlify, GitBook and ReadMe are hosted on third-party infrastructure with their own analytics and AI. That breaks ADR 0011's rule that everything comes from our origin. GitBook's and Mintlify's self-host options are unsupported or Enterprise-only.
- **Not recommended:** Nextra (stalled releases) and Material for MkDocs (maintenance window closed). Zensical is too early (0.1.x).
- **What the hosted platforms do well is reproducible here.** GitBook's headline AI features (Markdown twin per page, llms.txt, llms-full.txt) are what this site's `agentDocs` integration already does. An MCP server over the docs is the one thing not yet matched (the CLI has an MCP server, the docs do not).
- **Scalar and Zudoku are OpenAPI tools, not general docs frameworks.** Lamina PDF has no HTTP API, so they are irrelevant except as models.
- **Switching frameworks would not fix the CSP gap.** The `style-src 'unsafe-inline'` is caused by token colouring in style attributes, which Shiki-based highlighters do in every framework. The fix stays inside Starlight. Option one: configure Expressive Code/Shiki to emit class-based colours, as `apps/site`'s highlight.ts already does ("Shiki's tokens as classes coloured by the --paper-code-* tokens, since the CSP blocks inline styles", per CLAUDE.md). Option two: replace the code-block component.

### Gaps
- I did not fetch ReadMe pricing, Rspress, Zudoku or Scalar feature pages, Docusaurus 3.10 notes, or VitePress feature docs; tool budget. Feature claims for these (i18n, versioning, OpenAPI) are not individually sourced here.
- I did not find whether current Expressive Code has a class-based or "no inline styles" output mode. Check its changelog before deciding.
- I did not check whether Starlight's own inline scripts and styles work with Astro 6's stable `security.csp`, which delivers by meta tag, while this site uses `_headers`.

## Q3. Performance: JS per page, Lighthouse, build times

### Takeaway
I found no neutral published benchmark comparing these frameworks. My own measurement of each framework's own docs site on 2026-11-11 shows Starlight shipping about 6 KB gzip of script files. React-SPA frameworks ship roughly 400–700 KB gzip: Docusaurus ~489 KB, Fumadocs ~514 KB, Nextra ~710 KB, Mintlify ~385 KB. Rspress shipped ~1.5 MB. VitePress (~85 KB) and Zudoku (~96 KB) fall in between.

### Cited Findings
- **Method.** On 2026-11-11 I fetched one docs page per framework's own site. I summed the bytes of every `<script src>`, `modulepreload` and `preload` JS file in the initial HTML, then re-compressed each with gzip level 6. Script `src` values that the regex missed were not counted, and neither were chunks loaded later or on interaction. The script is in the session scratchpad and is not committed. Each site's own analytics are included where same-origin and counted separately where third-party.
- **Results** (script-file gzip; inline script characters; third-party script hosts):

| Page | Script files (gzip) | Inline script chars | Third-party script hosts |
| --- | --- | --- | --- |
| [starlight.astro.build/guides/authoring-content/](https://starlight.astro.build/guides/authoring-content/) | 6 files, ~6 KB (15 KB raw) | 7.8 k | Fathom |
| [docusaurus.io/docs/markdown-features](https://docusaurus.io/docs/markdown-features) | 3 files, ~489 KB (1.85 MB raw) | | Google Tag Manager |
| [vitepress.dev/guide/markdown](https://vitepress.dev/guide/markdown) | 6 files, ~85 KB | | Fathom |
| [fumadocs.dev/docs](https://fumadocs.dev/docs) | 29 files, ~514 KB | 231 k (RSC payload) | none |
| [nextra.site/docs](https://nextra.site/docs) | 20 files, ~710 KB | 81 k | none |
| [rspress.rs/guide/start/introduction](https://rspress.rs/guide/start/introduction) | 10 files, ~1.52 MB | | cdnjs, Cloudflare Insights, GTM |
| [mintlify.com/docs/quickstart](https://mintlify.com/docs/quickstart) | 9 files, ~385 KB | 18 k | |
| [mdBook guide](https://rust-lang.github.io/mdBook/guide/creating.html) | 13 files, ~199 KB | | cdnjs (MathJax) |
| [zudoku.dev/docs](https://zudoku.dev/docs) | 1 file, ~96 KB | | |
| [Material for MkDocs getting-started](https://squidfunk.github.io/mkdocs-material/getting-started/) | 0 | 3.2 k | |

- The Material for MkDocs result (no script files) is probably the regex missing its bundle, so treat it as unreliable. The figures are all my own measurement, not published numbers.

### Inferences
- Astro's islands model gives Starlight a large lead in JS shipped: about two orders of magnitude less than the React frameworks. Any move to Docusaurus, Fumadocs, Nextra or Rspress would add hundreds of KB of JS per page.
- A custom Astro site would match Starlight on JS weight, since both are Astro. The question there is maintenance cost, not performance.

### Gaps
- No published Lighthouse comparisons or large-site build-time benchmarks from a neutral source turned up in my searches. Build times for a 1,000-page site per framework remain unknown. Benchmarking them would mean building each framework locally, which I did not do.

## Q4. Interactivity: live code, Twoslash, synced tabs, islands

### Takeaway
Astro islands are the best fit for a light page with a few heavy widgets, such as a WASM PDF playground: only the island hydrates, and it can wait until visible or idle (`client:visible`, `client:idle`). React frameworks make widgets easy to write but hydrate everything. A Sandpack or StackBlitz embed would load from third-party origins and so breaks ADR 0011. A self-hosted WASM island, using `@laminapdf/sdk` from our origin, is the compliant route on any framework.

### Cited Findings
- Fumadocs on Astro renders MDX as React islands. — [Fumadocs docs](https://fumadocs.dev/docs)
- Starlight tabs sync across a page and across pages through `syncKey` (from Starlight's docs; I did not re-fetch it this session). — [Starlight Tabs](https://starlight.astro.build/components/tabs/)
- Code blocks: Starlight uses Expressive Code, a Shiki-based engine with plugins. Its token colours are inline styles, which force `style-src 'unsafe-inline'` on this site (`apps/docs/README.md`, Headers section). — [astro-shield issue](https://github.com/kindspells/astro-shield/issues/140)

### Inferences
- A live PDF playground fits Starlight now: an MDX page imports a framework component and renders it with `client:visible`. The component loads `@laminapdf/sdk` and the WASM from `'self'`. That needs `'wasm-unsafe-eval'`, which the docs CSP already allows for Pagefind, and `worker-src 'self'`.
- Sandpack, StackBlitz and CodeSandbox load from their own origins in iframes. That breaks "no third-party origins", so a self-built island is needed whatever the framework.
- Twoslash: Expressive Code has a Twoslash plugin (`expressive-code-twoslash`, community). I did not verify it this session. Its hover popups would add JS and, likely, inline styles. *Since measured by the [Q4p spike](../../../../spikes/docs-twoslash/README.md) (2026-11-12): 0.6.1 supports Expressive Code 0.41 and TypeScript 5 only; on Lamina PDF's docs it adds 24.5 KB of script and 15.5 KB of CSS to every page and 953 `style=` attributes to the page with hovers, and its hovers work by mouse only.*

### Gaps
- I did not verify the status of the Twoslash plugins for Expressive Code, Fumadocs (`fumadocs-twoslash`) or VitePress (`@shikijs/vitepress-twoslash`) this session.
- I did not verify a Sandpack CSP self-hosting option (Sandpack's bundler can be self-hosted, but I didn't confirm it).

## Q5. Published migrations between frameworks

### Takeaway
The migration evidence is thin and mostly anecdotal. Teams leave Docusaurus for Starlight or Fumadocs mainly over dev-server speed and developer experience. Material for MkDocs users are being pushed toward Zensical by its maintenance mode. I found no well-documented recent case of a team leaving Starlight.

### Cited Findings
- An HN commenter in a Starlight-switch thread called Docusaurus very slow. — [HN thread](https://news.ycombinator.com/item?id=44074626)
- One ADR describes a Docusaurus migration (the sv0 project, ADR 014). — [sv0 ADR 014](https://sv0-docs.pages.dev/docs/architecture/decisions/adr-014-docusaurus-migration) (not read in full)
- A project's issue says it will likely have to change its static site generator within a year because of the Material for MkDocs maintenance mode. — [L5lua website issue #20](https://github.com/L5lua/L5-website/issues/20)
- Official migration guides exist for Docusaurus → Astro/Starlight and Docusaurus → Fumadocs. — [Astro guide](https://docs.astro.build/en/guides/migrate-to-astro/from-docusaurus/); the Fumadocs guide was read on a Mintlify mirror that has since gone, with no archived copy

### Gaps
- I did not find named, high-profile migrations with published reasons that I could verify in this budget, such as GitBook → Mintlify or Docusaurus → Starlight by a well-known project. The report should not cite specific company migrations from this note.

## Q6. Which options are viable here, and the cost of a custom Astro site versus Starlight with overrides

### Takeaway
Stay on Starlight and customise it more, through overrides, route data and `StarlightPage`. Fix the one CSP gap (inline token styles) inside it. Switching to a React framework would add hundreds of KB of JS per page and fix nothing. Hosted platforms break ADR 0011. A custom Astro site without Starlight is the only realistic switch. It would mean rebuilding search UI wiring, sidebar and navigation, table of contents, i18n-ready routing, accessibility work and theme switching, and rewiring `starlight-typedoc` and `starlight-links-validator`. In return it would remove 0.x churn and give full layout freedom. It is worth it only if the Paper look needs layouts that overrides can't reach.

### Cited Findings
- The site already depends on Starlight-specific pieces: the `starlight-typedoc` sidebar group, `starlight-links-validator` and three component overrides. Its own integrations already carry the custom parts: the agent docs, release checks, CSP hashes and redirects. — `apps/docs/astro.config.mjs` (repository)
- `apps/site` already does Shiki highlighting with token classes coloured by `--paper-code-*` tokens, so that it can keep `style-src 'self'`. — `CLAUDE.md` (apps/site description, `src/data/highlight.ts`)
- Starlight's plugin ecosystem (versions, TypeDoc, OpenAPI, llms.txt, blog) is all community-maintained. — [Starlight plugins](https://starlight.astro.build/resources/plugins/)
- Starlight is 0.x and released 12 minors in about 22 months (0.31 to 0.42). — [npm registry](https://registry.npmjs.org/@astrojs/starlight)

### Inferences (opinion, labelled)
- **Stay and customise (recommended).**
  - Starlight's ~6 KB of JS per page, Pagefind search that runs on our own origin, and an owner (Cloudflare, also the host) that is good for the long term all fit ADR 0011.
  - Spend effort on two things: class-based code colouring so `style-src 'unsafe-inline'` can go, and self-hosted WASM islands for interactive examples.
  - Keep overrides few and pin minors. Each 0.x minor should be one small upgrade PR.
- **Custom Astro without Starlight.**
  - Same performance. Content collections, MDX and Pagefind carry over.
  - Rebuild cost: layout, sidebar from config, table of contents, pagination, header, search modal, theme toggle, mobile menu, accessibility audit, plus replacements for `starlight-typedoc` (TypeDoc → Markdown into a content collection) and `starlight-links-validator`. I found no published effort figure; any estimate would be a guess.
  - The ongoing benefit is no upstream breaking changes. The ongoing cost is that every fix Starlight gets for free (accessibility, RTL, i18n, Pagefind updates) becomes ours.
- **VitePress or Rspress:** possible and static, but they are a framework change (Vue or React) with no advantage over Starlight for this site, and VitePress 2 is in a long alpha.
- **Docusaurus, Fumadocs, Nextra:** a big JS increase per page, React hydration everywhere, and Nextra's stalled releases. They are a step back for "light pages with islands".
- **Mintlify, GitBook, ReadMe:** they break ADR 0011 (third-party hosting and analytics, documents through their servers) and cost $65–$540+ a month per site for features the site already builds itself (Markdown twins, llms.txt).

### Gaps
- There is no sourced engineering estimate (person-weeks) for a custom Astro docs shell. Any figure in the report should be labelled as an estimate.
- I did not check Starlight's issue tracker for open layout-customisation limits, such as multiple sidebars or a full-width page without the TOC. GitHub access was blocked.
