# Plan: make Lamina PDF easy to find in search engines, answer engines, coding agents and registries (Round W3)

- **Status:** Draft
- **Date:** 2026-11-11
- **Asked for by:** the project owner (2026-11-11): add a task for search engine and LLM search optimisation so that the library and the app are discovered easily; research it, make a plan and todos, and work on them
- **Builds on:**
  - the [discoverability research](../research/discoverability/search-and-answer-engines.md): what each engine reads, the evidence for llms.txt and "generative engine optimisation", and an audit of the three sites and the packages on 2026-11-11
  - the [website plan](website.md), whose round this work joins, and [ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md), whose rules it keeps
  - the agent docs that [L5a](../todos/done/L5a-agent-docs.md) built (llms.txt, Markdown twins, docs in the packages)
- **Todos:** Round W3 on the [todo board](../todos/README.md); §5 lists them.

## 1. Goal

A developer who asks a search engine, an AI answer engine or a coding agent for a PDF library for TypeScript or Rust should meet Lamina PDF when it fits their job. Someone who asks for a PDF reader that runs in the browser should meet the web reader. Today they mostly can't. The research found that every engine, AI answer engines included, starts from the same place: pages that are crawlable, indexed once on one host, and plain about what they answer. Lamina PDF's sites do most of that already, but the audit found gaps that cost more than any missing extra:

- the reader's staging site is open to indexing, and the docs' production pages are probably served on a second host;
- the repository went public, but the docs, READMEs and error links still treat it as private;
- package descriptions name ADRs and todo IDs instead of what the package does;
- nothing tells an engine what the project is (no structured data), and nobody measures whether engines find it.

Round W3 closes those gaps in order of reach, then adds the pages and the measurement that tell the project whether the work helped.

## 2. Rules for this round

- **Facts, never tricks.** No hidden text, no keyword stuffing, no invented ratings or reviews, no page written for crawlers that people don't see. Google treats invented ratings as spam, and the GEO study found keyword stuffing does nothing ([research §4, §6](../research/discoverability/search-and-answer-engines.md#4-what-raises-the-chance-of-being-cited)).
- **ADR 0011 stands.** No analytics, beacon or third-party script on any page. Engines are verified by DNS records, IndexNow is pinged from CI, and JSON-LD is a data block in the page, which no CSP governs.
- **Claims need the owner.** Any page that compares Lamina PDF with another library, or makes a new claim, goes to the owner before merge, as the website's copy does.
- **Measure before and after.** The answer-engine check (W3i) records a baseline before the bigger changes ship, so the round can show what changed.

## 3. What Round W3 doesn't do

- **No llms.txt changes for ranking's sake.** The files stay as L5a built them; W3f only makes them easier to find and lists them with Context7.
- **No paid placement, link buying or posting campaigns.** Posts off the site stay with the stories skill ([W2a](../todos/ongoing/W2a-story-skill-and-coverage-map.md), [website plan §9](website.md#9-stories-off-the-site)), and the [trust and proof plan](trust-and-proof.md) already ruled out leading with "local and private".
- **No Worker code.** Content negotiation (`Accept: text/markdown`) would need it or a Cloudflare zone setting; W3f puts the zone setting to the owner as an option.

## 4. Decisions for the owner

1. **Training crawlers** (`GPTBot`, `ClaudeBot`, `Google-Extended`, `CCBot`): allow or block? Search crawlers are allowed either way. Recommendation: allow. The code and docs are public and permissively licensed, and a model that learned the API writes correct code for it without a search. W3a writes whichever the owner chooses into every robots.txt.
2. **Index the reader?** Recommendation: index `app.laminapdf.com/` only, as the page that answers "open a PDF in the browser", with a canonical URL and share tags; the reader has no other pages to index.
3. **Repository description and topics** (a GitHub setting only the owner can change; the homepage is already `https://laminapdf.com`). Recommendation: "Create, read, edit and render PDFs from Rust and TypeScript, and a PDF reader for the browser", with the topics `pdf`, `pdf-generation`, `pdf-parser`, `pdf-editor`, `pdf-viewer`, `rust`, `typescript`, `webassembly`, `pdf-extraction`, `mcp`.
4. **Cloudflare's Markdown for Agents** on the docs zone: optional, since the docs already publish Markdown twins (W3f).

## 5. Todos

| ID | Todo | Depends on |
| --- | --- | --- |
| [W3a](../todos/ready/W3a-crawl-and-index-rules.md) | Crawl and index rules on every host: robots.txt for the docs site and the reader, `noindex` on the reader's staging and previews and on the docs' `workers.dev` copy, one robots policy, a test that holds the hosts to it | — |
| [W3b](../todos/ready/W3b-public-repository-links.md) | The repository is public: GitHub links on the docs site, `docsUrl` on the docs site's code pages (ADR 0026 §6), no "private" left in READMEs and plans, a README that names every part | — |
| [W3c](../todos/ready/W3c-package-metadata.md) | Package metadata written for users: npm and crates.io descriptions without internal IDs, keywords for the jobs people search for, `bugs` fields, READMEs that open with what the package does, and a check that keeps internal IDs out | — |
| [W3d](../todos/ready/W3d-structured-data-and-share-tags.md) | Structured data and share tags: JSON-LD on the website and the docs, share tags on the docs and the reader, links to the Markdown twins, `lastmod` in the sitemaps | W3a, W3i |
| [W3e](../todos/ready/W3e-search-engine-registration.md) | Search engines told about every change: Search Console and Bing Webmaster by DNS, sitemaps submitted, IndexNow pinged from the production deploys | W3d |
| [W3f](../todos/ready/W3f-agent-discovery.md) | Coding agents find the docs: a Context7 listing claimed with `context7.json`, llms.txt linked from the website and the README, L5a's production check ticked | W3b |
| [W3g](../todos/ready/W3g-choosing-a-pdf-library.md) | A page that answers "which PDF library should I use?": what Lamina PDF does and doesn't do beside the common libraries, with sources, approved by the owner | W3c |
| [W3h](../todos/ready/W3h-docs-titles-for-questions.md) | Docs titles and descriptions that match the questions people ask, checked against the page each one leads to | W3e, W3g |
| [W3i](../todos/ready/W3i-answer-engine-check.md) | The answer-engine check: a fixed set of questions asked to answer engines by hand, with Lamina PDF's mentions and citations recorded, a baseline now and a run after each step | — |

## 6. Sequence

W3a, W3b, W3c and W3i can start at once. W3a goes first: an indexable staging site and a duplicate docs host can rank above the real pages, and every later step invites crawlers in. W3d waits for W3a and for W3i's baseline, so the comparison is fair. W3e follows W3d, whose `lastmod` dates tell the IndexNow pings which pages changed. W3f follows W3b, because a Context7 listing reads the repository; W3g follows W3c, so the comparison links to the packages' improved pages; W3h comes last, once Search Console (W3e) has query data and W3g's page exists.

## 7. To-dos

- [ ] Set crawl and index rules on every host ([W3a](../todos/ready/W3a-crawl-and-index-rules.md))
- [ ] Treat the repository as public everywhere a reader or engine looks ([W3b](../todos/ready/W3b-public-repository-links.md))
- [ ] Write package metadata for users, and check it ([W3c](../todos/ready/W3c-package-metadata.md))
- [ ] Add structured data and share tags ([W3d](../todos/ready/W3d-structured-data-and-share-tags.md))
- [ ] Register with Google and Bing, and ping IndexNow from the deploys ([W3e](../todos/ready/W3e-search-engine-registration.md))
- [ ] List the docs with Context7 and link llms.txt where agents look ([W3f](../todos/ready/W3f-agent-discovery.md))
- [ ] Publish a page that helps a developer choose a PDF library ([W3g](../todos/ready/W3g-choosing-a-pdf-library.md))
- [ ] Title the docs pages for the questions people ask ([W3h](../todos/ready/W3h-docs-titles-for-questions.md))
- [ ] Ask answer engines a fixed set of questions, and record what they say ([W3i](../todos/ready/W3i-answer-engine-check.md))

With these done, each host serves its pages once and says what they are, the packages describe themselves in their users' words, the engines hear about every change, coding agents can fetch the current docs, and the project can see whether answer engines recommend it.
