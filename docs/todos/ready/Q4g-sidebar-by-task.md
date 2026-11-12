# Q4g — The sidebar grouped by task, with question titles

| Field | Value |
| --- | --- |
| ID | `Q4g` |
| Status | `ready` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | [W3h](./W3h-docs-titles-for-questions.md), [Q4f](../done/Q4f-every-page-in-the-sidebar.md), [Q4m](./Q4m-watch-readers-use-the-docs.md) |
| ADRs | None |
| Branch | `docs/q4g-sidebar-by-task` |
| PR title | `docs: group the sidebar by what readers want to do` |
| Size | M |
| PR | — |

## Goal

The sidebar's top level mixes tasks ("Create PDFs") with kinds of page ("Concepts", "Recipes", "Troubleshooting"). In the one study that watched developers read API docs, a top level split by kind of page confused them, and its authors recommend grouping by what the API does. This todo regroups the sidebar into task areas (proposed: create, read and extract, edit and save, render and view, command line; Q4m confirms them), with each area's tutorial, guides, recipes, concepts and troubleshooting inside it as labelled page types. It waits for Q4m, so the change answers what readers actually got lost on.

## Context

- [Docs research: recommendation 5](../../research/docs-site/developer-docs.md#recommendations-in-order); Meng, Steinhardt and Schubert (2019), eleven developers.
- Q4m's findings: where readers stopped or searched.
- W3h's question titles, which change the same front matter.
- `plugins/sidebar.mjs`, `plugins/agent-docs.mjs`, the redirects list in `astro.config.mjs`.

## Scope

**In scope**

- The new grouping in `sidebar.mjs`, and llms.txt in the same order.
- Page paths moved where the grouping needs it, each with a redirect.
- Page-type labels inside each group (for example a "Recipe" badge).

**Out of scope**

- Rewriting the pages themselves.

## Tasks

- [ ] Read Q4m's notes and decide the areas with the docs track.
- [ ] Change the sidebar and the paths; add the redirects.
- [ ] Check the Markdown twins, llms.txt and the packages' shipped docs still build in order.

## Acceptance criteria

- [ ] Every old path redirects to its new one.
- [ ] The sidebar's top level names tasks only.
- [ ] Q4m's reasons for the change are written in the plan.

## Post-merge checks

- [ ] Q4m's next round of sessions, if one runs, checks that readers find the pages they looked for before.

## Tests

The docs build, the link validator, the redirects.

## Docs to update

The docs redesign plan; the quality plan's §3 (the docs as a story).

## Risks and open questions

- Moving pages breaks inbound links and search results; redirect every one.
- If Q4m finds readers don't get lost at the top level, close this todo as not needed rather than regroup on the study alone.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
