# W3a — Crawl and index rules on every host

| Field | Value |
| --- | --- |
| ID | `W3a` |
| Status | `ready` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | None |
| ADRs | Implements [0011 §1](../../decisions/0011-cloudflare-topology-and-privacy.md#1-topology)'s `noindex` rule for the reader, which C4d left out; follows 0011 §4 (headers change only by adding `X-Robots-Tag` outside production) |
| Branch | `feat/w3a-crawl-and-index-rules` |
| PR title | `feat(sites): robots.txt and noindex on every host, one crawler policy` |
| Size | M (three sites' robots files, the reader's build input, a docs host rule, tests) |
| PR | — |

## Goal

Each of the project's hosts tells crawlers the same thing: production pages may be indexed, once, on their own host; every other page may be crawled but never indexed. Today two hosts can put a copy above the real page in a search index: the reader's staging site and PR previews are open to indexing (ADR 0011 §1 forbids it), and the docs' production pages are probably served again on a `workers.dev` address without `noindex`. Two robots files are also wrong: the reader answers `/robots.txt` with its HTML page, and the docs site has none. The todos after this one (W3d–W3h) invite crawlers in, so this one goes first.

## Context

- [Discoverability research §7](../../research/discoverability/search-and-answer-engines.md#7-what-lamina-pdf-exposes-today-audit-2026-11-11): the audit's gaps 2 and 3. §2 lists the search, user and training crawlers of OpenAI, Anthropic, Perplexity and Google, and what each one honours.
- [Plan §4](../../plans/discoverability.md#4-decisions-for-the-owner), decisions 1 and 2: whether training crawlers are allowed, and whether the reader's start page is indexed. Ask the owner in the PR if they haven't answered.
- `packages/site-kit/src/integrations.mjs`: `markNoindex` and `previewNoindex`, which add `X-Robots-Tag: noindex` to every non-production Astro build. The website's robots.txt is `apps/site/src/pages/robots.txt.ts`.
- `apps/docs/plugins/release.mjs`: `headerProblems` fails a released docs build that sends `noindex` and an unreleased one that doesn't.
- `apps/docs/wrangler.toml`: `workers_dev = true` on the production Worker for the per-PR previews.
- `apps/web/public/_headers`, `apps/web/wrangler.jsonc` (single-page fallback) and `.github/workflows/web-deploy.yml` (preview, staging and production builds, all through `cargo xtask web build`).

## Scope

**In scope**

- One robots policy, written once in `@laminapdf/site-kit` (a function that takes the environment and the sitemap URL and returns the file), used by the website, the docs site and the reader's build.
- robots.txt for the docs site (production: allow, with the sitemap index) and the reader (production: allow `/` with the training-crawler choice). Non-production builds of all three sites serve a robots.txt that doesn't block crawling, and rely on `noindex`: Google reads `noindex` only on pages robots.txt lets it fetch ([Google: block indexing with noindex](https://developers.google.com/search/docs/crawling-indexing/block-indexing)), and the reader's staging host may already be indexed. The website's non-production `Disallow: /` changes to match.
- `X-Robots-Tag: noindex` on the reader's staging and preview builds, from a build input (`LAMINA_PDF_ENV`), as the two Astro sites do.
- `noindex` on the docs' production Worker when it is reached on a `workers.dev` host, or that host turned off if the previews can move.

**Out of scope** (and which todo covers it)

- Canonical URLs and share tags on the reader: [W3d](./W3d-structured-data-and-share-tags.md).
- Submitting sitemaps and pinging IndexNow: [W3e](./W3e-search-engine-registration.md).
- Soft 404s on unknown reader paths other than `/robots.txt`: the single-page fallback serves the app's own routes; leave it.

## Tasks

- [ ] Get the owner's answer to plan §4 decisions 1 and 2 (training crawlers; the reader's start page) and record it in the plan.
- [ ] `packages/site-kit/src/robots.mjs`: `robotsTxt({ production, sitemap, training })` returns the file. Production: `User-agent: *` / `Allow: /`, then, if training is blocked, one group per training crawler (`GPTBot`, `ClaudeBot`, `Google-Extended`, `CCBot`) with `Disallow: /`, then `Sitemap:`. Otherwise `User-agent: *` / `Allow: /` with no sitemap, so crawlers can read the `noindex` header. Export it from the package; unit tests for both cases.
- [ ] `apps/site/src/pages/robots.txt.ts`: use it.
- [ ] `apps/docs`: a `src/pages/robots.txt.ts` endpoint with the same call, the sitemap being Starlight's `sitemap-index.xml` on `docs.laminapdf.com`; the released-docs check in `plugins/release.mjs` also fails a release whose robots.txt disallows everything or names no sitemap.
- [ ] `apps/docs`: keep the production pages off `workers.dev`. First try `workers_dev = false` with `preview_urls = true` on the production Worker, the combination the reader's `lamina-pdf-web-preview` Worker already uses, and check a PR preview still deploys. Failing that, a `_headers` rule that matches the `workers.dev` host (check that Workers static assets honour absolute-URL rules first, and test it with `wrangler dev`), or move the previews to their own Worker as the reader does (`lamina-pdf-web-preview`) and set `workers_dev = false`. Record which in `apps/docs/README.md`.
- [ ] `apps/web`: read `LAMINA_PDF_ENV` in the build (`vite.config.ts` or `cargo xtask web build`), write `dist/robots.txt` from the site kit's function and add `X-Robots-Tag: noindex` to `dist/_headers`' `/*` block unless it is `production`. Reuse `markNoindex` if it fits a Vite build; otherwise move the shared part into the site kit, so there is one implementation.
- [ ] `.github/workflows/web-deploy.yml`: set `LAMINA_PDF_ENV=production` on the production build only, `staging` on staging; previews get neither. `tools/xtask/src/workflows.rs`: a test that only the production job sets `production`.
- [ ] C4d's header comparison (ADR 0011 §1: "C4d compares the two files"): check that staging's `_headers` equals production's plus the `X-Robots-Tag` line, and that test exists or is added.
- [ ] Update ADR 0011 §1's implementation note if the docs' preview host changes.

## Acceptance criteria

- [ ] A production build of each of the three sites serves a robots.txt that allows search crawlers and names its sitemap (the reader has none: no `Sitemap` line), and sends no `X-Robots-Tag`.
- [ ] A staging or preview build of each sends `X-Robots-Tag: noindex` on every response, and its robots.txt lets crawlers fetch the pages so they read it.
- [ ] `curl -sI https://staging-app.laminapdf.com/` would show `X-Robots-Tag: noindex` (checked against `wrangler dev` with the staging build in the reader's e2e suite).
- [ ] `/robots.txt` on the reader answers `text/plain`, not the app's HTML.
- [ ] The docs' production pages carry `noindex` on any host but `docs.laminapdf.com`, or are served nowhere else.
- [ ] The robots file is written by one function, used by all three sites.

## Post-merge checks

- [ ] `https://staging-app.laminapdf.com/` sends `X-Robots-Tag: noindex` after the next push to `main`.
- [ ] `https://docs.laminapdf.com/robots.txt` and `https://next.docs.laminapdf.com/robots.txt` answer as planned after the next release and the next push to `main`.
- [ ] The reader's robots.txt after the next `web-v*` release.
- [ ] Google Search Console's URL inspection (once W3e verifies the domain) shows no staging or `workers.dev` URL indexed; request removal of any that are.

## Tests

- `packages/site-kit/test/robots.test.ts`: both environments, both training choices, the sitemap line.
- The docs site's plugin tests: the release check over robots.txt.
- `apps/web/e2e/`: headers and robots.txt of a staging build and a production build under `wrangler dev`.
- `tools/xtask/src/workflows.rs`: which jobs set `LAMINA_PDF_ENV`.

## Docs to update

- `apps/web/README.md` and `apps/web/DEPLOY.md`: the build input and what it changes.
- `apps/docs/README.md`: robots.txt and the preview host.
- `packages/site-kit/README.md`: `robotsTxt`.
- CLAUDE.md's descriptions of the three apps and the site kit.
- The [discoverability plan](../../plans/discoverability.md): tick W3a, record the owner's decisions.

## Risks and open questions

- Whether Workers static assets' `_headers` can match on host. If not, moving the docs previews to a separate Worker touches `docs.yml`'s preview jobs; keep the change to what the reader's previews already do.
- A cached copy of a staging page may stay in an index for weeks; the post-merge check asks for removal.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
