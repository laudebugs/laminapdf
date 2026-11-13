# W3e — Search engines told about every change

| Field | Value |
| --- | --- |
| ID | `W3e` |
| Status | `ready` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | [W3d](../ongoing/W3d-structured-data-and-share-tags.md) |
| ADRs | Follows [0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (no page change for verification; no ping from a browser) |
| Branch | `feat/w3e-indexnow` |
| PR title | `feat(sites): IndexNow pings from the production deploys` |
| Size | S (a key file per site, one script, two workflow steps, tests) plus the owner's setup |
| PR | — |

## Goal

Google and Bing learn about the sites from their sitemaps and, for Bing and the engines that share its IndexNow submissions, from pings. Nobody has registered the sites with either engine, so nobody can see what they index, which queries show the pages, or the Copilot citations Bing now reports. This todo gives the owner the setup steps (DNS verification, which touches no page) and makes each production deploy ping IndexNow with the URLs it changed.

## Context

- [Discoverability research §1](../../research/discoverability/search-and-answer-engines.md#1-search-engines-are-still-the-entry-point-for-ai-answers): Search Console's AI traffic, Bing's AI Performance report, IndexNow's key file and endpoint ([IndexNow documentation](https://www.indexnow.org/documentation)).
- `.github/workflows/site.yml` (production on every push to `main` and every six hours) and `.github/workflows/docs.yml` (production from each release).

## Scope

**In scope**

- `apps/site/README.md` and `apps/docs/README.md`: the owner's steps for Search Console (a Domain property for `laminapdf.com`, verified by DNS TXT, which covers every subdomain) and Bing Webmaster Tools (import from Search Console), and submitting each sitemap.
- An IndexNow key: a key file in each site's `public/` (the key is public by design) and a script that compares the deployed sitemap with the new one and posts the URLs that are new or whose `lastmod` moved; run after each production deploy, never on previews.
- A dry-run mode for the script and a test of the URL comparison.

**Out of scope**

- Any analytics: ADR 0011 forbids it.

## Tasks

- [ ] `tools/indexnow/` or an xtask subcommand (`cargo xtask indexnow --site <url> --sitemap <old> <new> [--dry-run]`): the changed URLs, one POST to `api.indexnow.org` with `host`, `key`, `keyLocation`, `urlList`; a failure warns and never fails the deploy.
- [ ] Key files in `apps/site/public/` and `apps/docs/public/`; the site kit's checks accept them.
- [ ] Workflow steps after the production deploys; `tools/xtask/src/workflows.rs` tests that they run only on production and only after a deploy succeeded.
- [ ] README steps for the owner, including where to read the AI Performance report.

## Acceptance criteria

- [ ] A dry run against two sitemap fixtures prints exactly the added and changed URLs.
- [ ] The workflows call it after production deploys only.
- [ ] Neither site's pages change, apart from the key file.

## Post-merge checks

- [ ] The owner has verified `laminapdf.com` in Search Console and Bing Webmaster Tools and submitted the two sitemaps (`laminapdf.com/sitemap.xml` and the docs' `sitemap-index.xml`; the reader has none); note the date in the plan.
- [ ] The first production deploy after merge logs an accepted IndexNow submission (HTTP 200 or 202).

## Tests

Unit tests of the sitemap comparison and the request body; `workflows.rs` tests.

## Docs to update

The two site READMEs, CLAUDE.md's workflow paragraphs, and the [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- The previous sitemap: fetch the live one before deploying, or keep the last one as an artefact. Fetching is simpler and needs no state.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
