# W3d — Structured data and share tags

| Field | Value |
| --- | --- |
| ID | `W3d` |
| Status | `done` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | [W3a](./W3a-crawl-and-index-rules.md), [W3i](./W3i-answer-engine-check.md) |
| ADRs | Follows [0011 §4](../../decisions/0011-cloudflare-topology-and-privacy.md#4-security-headers-appswebpublic_headers-for-) (JSON-LD is a data block the browser never runs, so the CSP doesn't apply to it) |
| Branch | `feat/w3d-structured-data` |
| PR title | `feat(sites): structured data, share tags and links to the Markdown twins` |
| Size | M |
| PR | [#575](https://github.com/laudebugs/laminapdf/pull/575) |

## Goal

Search and answer engines read structured data to learn what a site is, who publishes it and where the code lives, and read share tags when a link is posted. The website has share tags but no structured data; the docs site has neither a share image nor structured data; the reader has a title and a description and nothing else. Agents that want Markdown have no link from a docs page to its twin. This todo adds each of these, from one shared description of the project.

## Context

- [Discoverability research §6](../../research/discoverability/search-and-answer-engines.md#6-structured-data-cant-earn-a-rich-result-here-but-still-names-the-project): which schema.org types fit (no `SoftwareApplication` rich result without ratings, and no invented ratings) and §3 on Markdown twins.
- `apps/site/src/layouts/Base.astro` (the website's head), `apps/docs/astro.config.mjs` (Starlight's `head`), `apps/web/index.html`.
- `packages/site-kit/src/integrations.mjs`: `inlineScripts` hashes every inline `<script>` into `_headers` whatever its `type`, and the docs site's CSP line has a 1,900-character limit. A JSON-LD data block needs no hash.

## Scope

**In scope**

- One module in `@laminapdf/site-kit` that writes the JSON-LD: `Organization` (name, URL, logo, `sameAs` the GitHub repository and the registries), `WebSite`, and `SoftwareSourceCode` (repository, licence, languages, runtime platforms). Website home page and docs home page.
- `inlineScripts` skips data blocks (a `type` that isn't JavaScript or a module), so JSON-LD adds nothing to the CSP.
- The docs site: `og:image` (the website's share image or one of its own) and `twitter:card` image.
- The reader: canonical URL, description, Open Graph and X card tags on `/` in production (per plan §4 decision 2). The single-page fallback serves the same `index.html` on every path, so its canonical link to `https://app.laminapdf.com/` also keeps stray paths from being indexed as copies of `/` (found in W3a's review).
- `<link rel="alternate" type="text/markdown" href="….md">` on every docs page that has a twin.
- `lastmod` in both sitemaps from each page's last commit (the docs' through `@astrojs/sitemap`'s `serialize` option, or a sitemap of the docs' own if Starlight's can't take it). W3e's IndexNow pings read these dates to tell which pages changed.

**Out of scope** (and which todo covers it)

- Page titles and descriptions: [W3h](../ready/W3h-docs-titles-for-questions.md).

## Tasks

- [x] `packages/site-kit/src/structured-data.mjs`: the objects above from one set of facts (name, URLs, licence from `Cargo.toml`), with tests that they validate against schema.org's types (shape checks; no network).
- [x] Teach `inlineScripts` to skip data blocks, with a test; add the JSON-LD to both sites' heads; check that the CSP lines don't change.
- [x] Docs share image and tags; reader tags (the reader's CSP and ADR 0011's reader rules allow meta tags; check the e2e header test).
- [x] The alternate links: `plugins/agent-docs.mjs` knows each twin's path; add the link in the same pass, and fail the build when a page's link and twin disagree.
- [x] Sitemap `lastmod` (both sites): from `git log -1 --format=%cI -- <source>`; fail the production build when the history is shallow rather than write today's date.
- [ ] Check every page in Google's Rich Results Test and the Schema Markup Validator by hand on the PR preview; note results in the PR.

## Acceptance criteria

- [x] The website's and the docs' home pages carry valid JSON-LD with no rating, review or offer.
- [x] Every docs page with a twin links it with `rel="alternate"`; the build fails if one is missing or broken.
- [x] The reader's production `/` has canonical and share tags; staging has none of the canonical tags and keeps `noindex`.
- [x] The website's checks (`siteChecks`) and every CSP still pass.

## Post-merge checks

- [ ] Search Console's enhancement reports (W3e) show no structured-data errors after the next crawl.

## Tests

Site-kit unit tests; the docs plugin tests for alternate links; the reader's e2e head test.

## Docs to update

`packages/site-kit/README.md`, `apps/docs/README.md`, `apps/web/README.md`, CLAUDE.md, and the [discoverability plan](../../plans/discoverability.md).

## Notes from the work

- **The Markdown twin links were already there.** L5a had added `<link rel="alternate" type="text/markdown">` to every docs page (`src/components/Head.astro`), and `plugins/agent-docs.mjs` already fails the build on a page without it. This todo changed neither.
- **The docs' sitemap is rewritten after the build.** Starlight writes it through `@astrojs/sitemap` and takes no `serialize` option, so `plugins/sitemap.mjs` adds the dates to its files. The one-page views have no source of their own and get no date.
- **The released docs need the whole history.** `docs.yml`'s production job must check out with `fetch-depth: 0`. An agent's token can't change workflow files, so the PR gives that change, and a `tools/xtask/src/workflows.rs` test that holds the job to it, as a patch for the owner to commit before merging.
- **The reader shares the website's image** by its full address: a meta tag loads nothing, so the reader's CSP is not involved.

## Risks and open questions

- Other pages may want their own JSON-LD later (`TechArticle` on guides); with data blocks out of the CSP, that costs nothing but the markup.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
