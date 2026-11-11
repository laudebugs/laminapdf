# W3h — Docs titles and descriptions that match the questions people ask

| Field | Value |
| --- | --- |
| ID | `W3h` |
| Status | `ready` |
| Plan | [Discoverability §5](../../plans/discoverability.md#5-todos) |
| Round / milestone | Round W (website), W3 discoverability |
| Depends on | [W3d](./W3d-structured-data-and-share-tags.md) |
| ADRs | None |
| Branch | `docs/w3h-docs-titles` |
| PR title | `docs: page titles and descriptions for the questions people ask` |
| Size | S–M (front matter across the docs pages) |
| PR | — |

## Goal

Search engines match a page's title and description against the query, and answer engines quote the description. Many docs pages are titled by Lamina PDF's own feature names ("Save a PDF") rather than the question a newcomer types ("Edit a PDF in TypeScript without breaking signatures"). This todo goes through every guide and recipe, writes each title and description for the question the page answers, and keeps them true to the page.

## Context

- [Discoverability research §1 and §4](../../research/discoverability/search-and-answer-engines.md#1-search-engines-are-still-the-entry-point-for-ai-answers).
- Search Console's queries report (W3e), once it has data: the queries that already show the pages.
- [Docs writing guideline](../../guidelines/docs-writing.md).

## Scope

**In scope**

- Titles (the `<title>` and the sidebar label may differ; Starlight's `head` or `title` front matter) and descriptions of guides, recipes and troubleshooting pages.
- Redirects for any page whose path changes (`astro.config.mjs` already keeps a list).

**Out of scope**

- New pages, apart from what W3g adds.

## Tasks

- [ ] List each page with its current title and description, the question it answers, and (when W3e has data) its queries.
- [ ] Rewrite; name the language or runtime where the question does ("in TypeScript", "in Rust", "in the browser").
- [ ] Keep descriptions under about 160 characters and true to the page.

## Acceptance criteria

- [ ] Every guide and recipe has a description that names the task and the language it covers.
- [ ] No path changes without a redirect.

## Post-merge checks

- [ ] Search Console's queries and click-through for the changed pages after four weeks, noted in the plan.

## Tests

The docs build and `check docs-prose`.

## Docs to update

The [discoverability plan](../../plans/discoverability.md).

## Risks and open questions

- Titles written for search can read worse in the sidebar; keep the sidebar labels short.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
