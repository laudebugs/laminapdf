# How developers will find Lamina PDF: search engines, answer engines, coding agents and registries

- **Date:** 2026-11-11
- **Asked for by:** the project owner: research search engine and LLM search optimisation so that the library and the app are easy to discover
- **Feeds:** the [discoverability plan](../../plans/discoverability.md)

A developer looking for a PDF library in 2026 starts in one of four places: a search engine, an AI answer engine (Google's AI Overviews and AI Mode, ChatGPT search, Claude, Perplexity, Copilot), a coding agent that already has a task in hand, or a package registry. Each place reads different signals, but the official documentation of every one of them points to the same foundation. A site gets found when its pages are crawlable, indexed and say plainly what they answer. No special file or markup buys a place in AI answers. This note sets out what each place reads, what the evidence says about the extras people sell (llms.txt, "generative engine optimisation"), and what Lamina PDF's sites and packages expose today. It ends with recommendations, labelled as such.

## 1. Search engines are still the entry point for AI answers

Google's AI features draw on the ordinary index. Its documentation for site owners says "there are no additional requirements to appear in AI Overviews or AI Mode", and that a site needs no AI text files, llms.txt or special schema; a page must be indexed and eligible to show with a snippet ([Google Search Central: AI features and your website](https://developers.google.com/search/docs/appearance/ai-features)). The controls are the ordinary ones: `nosnippet`, `data-nosnippet`, `max-snippet` and `noindex`, with `Google-Extended` governing only Gemini's training and grounding, not Search. Search Console counts traffic from AI Overviews and AI Mode under the Web search type of the Performance report.

Bing feeds Microsoft Copilot the same way. Bing Webmaster Tools added an AI Performance report covering Copilot citations in public preview on 10 February 2026 and extended it on 16 June 2026 ([Bing Webmaster blog](https://blogs.bing.com/webmaster/)). Bing's recommended workflow is a sitemap plus IndexNow, a protocol in which a site posts changed URLs to one endpoint and every participating engine shares them. The site proves ownership with a key file at its root, `https://<host>/<key>.txt` ([IndexNow documentation](https://www.indexnow.org/documentation)). ChatGPT search also leans on Bing's index, so Bing matters more than its share of searches suggests.

**What this means:** technical SEO (crawlable pages, a sitemap, canonical URLs, one host per page, titles and descriptions that match what people search) is the work that reaches every answer engine at once.

## 2. Answer engines crawl with their own bots, and separate search from training

Each AI company runs separate user agents for search, for user-initiated fetches and for training. Blocking a training crawler doesn't remove a site from that company's search answers, and blocking a search crawler removes it from them even if training is allowed.

| Company | Search index (allow to be cited) | Fetch for a user's question | Training | Source |
| --- | --- | --- | --- | --- |
| OpenAI | `OAI-SearchBot` | `ChatGPT-User` (robots.txt may not apply; not used for search) | `GPTBot` | [OpenAI crawlers](https://developers.openai.com/api/docs/bots) |
| Anthropic | `Claude-SearchBot` | `Claude-User` | `ClaudeBot` | [Anthropic support article 8896518](https://support.claude.com/en/articles/8896518) |
| Perplexity | `PerplexityBot` (not used for model training) | `Perplexity-User` ("generally ignores robots.txt") | none listed | [Perplexity crawlers](https://docs.perplexity.ai/guides/bots) |
| Google | `Googlebot` (Search, AI Overviews, AI Mode) | none separate | `Google-Extended` token (Gemini) | [Google AI features](https://developers.google.com/search/docs/appearance/ai-features) |

OpenAI says each setting is independent; Anthropic says its three bots honour robots.txt and `Crawl-delay`. A `User-agent: *` / `Allow: /` file, as the website has today, already lets every search crawler in.

**Opinion:** for an open-source library, training crawlers help too. A model that learned Lamina PDF's API writes correct code for it without a search, and the code and docs are public and permissively licensed. Allowing training is the owner's choice, though, and the plan asks for it.

## 3. llms.txt helps coding agents, not search rankings

[llms.txt](https://llmstxt.org/) is a Markdown index of a site's docs for language models. Adoption is rising (SE Ranking counted about 10% of domains in May 2026), but crawlers rarely fetch it: it drew about 0.1% of AI-bot visits in the same study, and Ahrefs found 97% of the files it checked received no traffic. Google says it doesn't use it, and no study shows it raises citations.

The audience that does use it is coding agents and documentation tools. [Context7](https://context7.com/), which many coding agents query through MCP for current library docs, can ingest a library from its llms.txt (its add-library page, or the `/v2/add/llmstxt` API) and lets the owner claim the listing with a `context7.json` file in the repository. Lamina PDF already builds llms.txt, llms-full.txt and a Markdown twin of every docs page (todo L5a), and ships versioned docs inside `@laminapdf/sdk` and the `laminapdf` crate. The [libraries research](../libraries/notes/llm-api-and-ai-extraction.md) reached the same view: cheap to keep, unproven for ranking, useful for agents.

Cloudflare's "Markdown for Agents" (changelog 2026-02-12) converts HTML to Markdown at the edge for requests that send `Accept: text/markdown`, with `Vary: accept`. Lamina PDF already has hand-made twins at `.md` paths. A `<link rel="alternate" type="text/markdown">` on each page would tell agents where they are; nothing advertises them today but one docs page.

## 4. What raises the chance of being cited

The one controlled study of "generative engine optimisation" (Aggarwal et al., [GEO: Generative Engine Optimization](https://arxiv.org/abs/2311.09735), KDD 2024) rewrote pages in nine ways and measured how often answer engines cited them. Adding citations to sources, quotations and statistics raised visibility by up to 40%, and by up to 37% on Perplexity. Keyword stuffing did nothing. The finding matches what the project already does on its How we test page: specific numbers with their sources.

Vendor studies of which sources AI answers cite for developer tools (for example devtune.ai's) find the same few sites again and again: Reddit, GitHub, Stack Overflow, Hacker News and blog posts. These studies come from companies that sell the service, so the ranking is indicative, not measured. They agree with each other, and with the project's own [Hacker News research](../hacker-news/pdf-users-on-hacker-news.md), that a library gets recommended when people discuss it where developers ask. A site can't optimise its way into those threads; it can only be worth linking to.

## 5. Package registries rank on metadata and READMEs

- **npm:** since December 2024 search sorts by relevance, downloads, dependents or recency. Relevance reads the name, description and keywords, so a description written for users and keywords for the jobs people search for ("pdf generation", "pdf parser", "extract text from pdf") count.
- **crates.io and lib.rs:** crates.io matches keywords (five at most) and categories; lib.rs ranks with its own signals and infers categories when they are missing. Both show the README, so its first paragraph and a working example are what a visitor judges.
- **docs.rs** builds every published crate. The `documentation` field decides where crates.io's "Documentation" link goes; Lamina PDF points it at the docs site.

## 6. Structured data and versioned docs

Schema.org's `SoftwareApplication` rich result needs `offers` plus `aggregateRating` or `review`; Google treats invented ratings as spam and can issue a manual action ([Google: software app structured data](https://developers.google.com/search/docs/appearance/structured-data/software-app)). Lamina PDF has no ratings, so it can't earn that rich result. `Organization`, `WebSite` and `SoftwareSourceCode` markup still tells engines what the site, the publisher and the code repository are, and costs one inline script.

Sites with several doc versions mark non-production builds `noindex` (an `X-Robots-Tag` header) and point older versions' canonical links at the current one. Lamina PDF does the first for next.docs and previews.

## 7. What Lamina PDF exposes today (audit, 2026-11-11)

The audit read the three sites, the package manifests and the repository at `origin/main` 706689a8.

**In place:**

- The website and the docs site have titles, descriptions, canonical URLs, Open Graph tags and sitemaps. The website's build fails a page without a title, description or canonical URL (`@laminapdf/site-kit`).
- Every non-production website and docs build sends `X-Robots-Tag: noindex`; the released docs build fails if it does.
- The website's robots.txt allows every crawler in production and names the sitemap.
- The docs site builds llms.txt, llms-full.txt and a Markdown twin per page; the SDK and the crate ship versioned docs and an AGENTS.md snippet.
- Thirteen crates have descriptions, keywords and categories; the six npm packages have descriptions and keywords.

**Gaps:**

1. **The repository is public, but the project still says it is private.** GitHub reports `"visibility": "public"`, the homepage `https://laminapdf.com`, no topics and the description "My mom asked for a pdf reader. So here." The docs site hides its GitHub links (`LAMINA_PDF_DOCS_GITHUB` off "while it is private"), error `docs_url`s point at GitHub, and the website plan and several READMEs describe the repository as private.
2. **The reader's staging and previews are indexable.** ADR 0011 §1 requires `X-Robots-Tag: noindex` on them; nothing in `apps/web` or `web-deploy.yml` adds it. The reader's single-page fallback also answers `/robots.txt` and every unknown path with `200` and the app's HTML (soft 404s).
3. **The docs site has no robots.txt**, and its production Worker keeps `workers_dev = true` for previews, so production pages are probably also served on a `workers.dev` host without `noindex`.
4. **No structured data** on any site, no `og:image` on the docs, and no Open Graph or canonical tags on the reader.
5. **Agent docs are hard to find:** no `rel="alternate"` link to the Markdown twins, no link to llms.txt from the website or the README, and no Context7 listing (L5a deferred it to after the first registry prerelease, which has shipped).
6. **Package metadata speaks the project's internal language.** The render and viewer packages' descriptions cite ADR numbers and todo IDs, and so do the `laminapdf-extract`, `laminapdf-render` and `laminapdf-render-pdfium` crates' descriptions; four READMEs (`@laminapdf/wasm`, `viewer`, `render-hayro`, `render-pdfium`) say the package "is private" although it is published; no package has a `bugs` field. READMEs link files by repository-relative paths, which npm and crates.io resolve through the `repository` field now that the repository is public.
7. **Nobody measures discovery.** No Search Console or Bing Webmaster property is mentioned anywhere, and no one has asked an answer engine "what PDF library should I use for TypeScript?" and recorded the answer.

ADR 0011 limits how the gaps can be closed. The sites load nothing from other origins and send no beacon, so analytics scripts and verification tags that load code are out. DNS TXT records verify Search Console and Bing without touching a page; an IndexNow key file is a static asset; IndexNow pings go from CI after a deploy, never from a browser; and JSON-LD is a data block (`<script type="application/ld+json">`), which the browser never runs, so the CSP doesn't apply to it. The site kit hashes every inline `<script>` today whatever its type, and should skip data blocks.

## 8. Recommendations

These are recommendations, in the order of cost against reach:

1. **Fix crawling and indexing on all three hosts first:** robots.txt everywhere, `noindex` on every non-production host (the reader's staging and previews, the docs' `workers.dev` copy), a real robots.txt on the reader, and one robots policy for search and training crawlers, decided by the owner. A host that must leave the index has to stay crawlable until it does: Google reads `noindex` only on pages robots.txt lets it fetch ([Google: block indexing with noindex](https://developers.google.com/search/docs/crawling-indexing/block-indexing)).
2. **Finish the public transition in what search engines and visitors read:** repository topics and description, GitHub links on the docs site, error links on the docs site, and no "private" left in READMEs.
3. **Write package metadata for users:** descriptions without ADR or todo IDs, keywords for the jobs people search for, `bugs` fields, and READMEs that open with what the package does.
4. **Add the metadata engines read:** JSON-LD for the organisation, the website and the code; share tags on the docs and the reader; `rel="alternate"` links to the Markdown twins; `lastmod` in sitemaps.
5. **Register with the engines:** Search Console and Bing Webmaster by DNS, sitemaps submitted, IndexNow pings from the deploy workflows, and a Context7 listing claimed with `context7.json`.
6. **Write pages that answer the questions developers ask,** with numbers and sources: choosing a PDF library for TypeScript or Rust, extracting text for RAG, generating accessible PDFs. Compare honestly and cite the comparison's sources.
7. **Measure:** Search Console and Bing's AI Performance report, registry downloads, and a fixed set of questions asked to answer engines each month, recorded the way the agent eval records its runs.

Being discussed on Hacker News, Reddit and Stack Overflow matters as much as any of this, but it follows from the work being worth discussing. The [trust and proof plan](../../plans/trust-and-proof.md) already decided not to lead with "local and private"; the stories skill (W2a) covers posts off the site.
