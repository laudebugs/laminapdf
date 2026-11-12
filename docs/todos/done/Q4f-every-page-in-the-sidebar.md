# Q4f — Every docs page in the sidebar, kept there by a check

| Field | Value |
| --- | --- |
| ID | `Q4f` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | None |
| Branch | `docs/q4f-sidebar-complete` |
| PR title | `docs: put every recipe in the sidebar and check that every page is listed` |
| Size | S |
| PR | [laudebugs/lamina#440](https://github.com/laudebugs/laminapdf/pull/440) |

## Goal

The docs' house rule is that every page is in the sidebar, in the story's order. Two recipes, "Invoice from JSON" and "Markdown to PDF", are not, so readers find them only by search, and llms.txt puts them in a trailing "More" section. This todo lists them and adds a check so a new page can't be left out again.

## Context

- [Docs research: recommendation 5](../../research/docs-site/developer-docs.md#recommendations-in-order) and the [audit](../../research/docs-site/developer-docs/current_docs_site.md).
- `apps/docs/plugins/sidebar.mjs` (the explicit list), `plugins/agent-docs.mjs` (llms.txt's order).

## Scope

**In scope**

- Both recipes added to the Recipes group.
- A build or test check that every hand-written page under `src/content/docs/` is in the sidebar.

**Out of scope**

- Regrouping the sidebar (Q4g).

## Tasks

- [x] Add the two recipes in a sensible place in the Recipes group.
- [x] Add the check (a test beside the plugins, or a build step) and make it fail on a missing page.

## Acceptance criteria

- [x] Every hand-written page is in the sidebar.
- [x] Removing a page from `sidebar.mjs` fails the check.

## Post-merge checks

None.

## Tests

The new check, in `apps/docs`'s tests.

## Docs to update

None beyond the sidebar.

## Risks and open questions

- None.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
