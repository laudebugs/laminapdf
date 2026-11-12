# Q4l — A Markdown alternate link and "Copy as Markdown" on every page

| Field | Value |
| --- | --- |
| ID | `Q4l` |
| Status | `ongoing` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | None |
| Branch | `feat/q4l-markdown-alternate` |
| PR title | `feat(docs): link each page's Markdown twin and offer to copy it` |
| Size | S |
| PR | [#460](https://github.com/laudebugs/laminapdf/pull/460) |

## Goal

Coding agents are among the few readers that fetch llms.txt at all; in Ahrefs' study, Claude Code was the second most active named tool. The docs already build a Markdown twin of every page, but nothing in a page's HTML points to it. This todo adds a `rel="alternate" type="text/markdown"` link to each page and a "Copy as Markdown" button, so agents and people pasting into a chat get the clean text.

## Context

- [Docs research: recommendation 10](../../research/docs-site/developer-docs.md#recommendations-in-order).
- `plugins/agent-docs.mjs`, `plugins/markdown.mjs` (the twins), the `Header` override.

## Scope

**In scope**

- The alternate link in every page's head.
- A "Copy as Markdown" button near the title, loading the twin from the same origin.

**Out of scope**

- A chat assistant on the site (out of scope for the round).

## Tasks

- [ ] Add the link through Starlight's head config or the override.
- [ ] Add the button as a small island; it fetches the twin only when pressed.
- [ ] Check the CSP allows it (same origin) and the e2e tests cover it.

## Acceptance criteria

- [ ] Every page links its twin.
- [ ] The button copies the twin's text.

## Post-merge checks

None.

## Tests

The docs e2e tests; a build test that every page has the link.

## Docs to update

The coding agents page (`start/coding-agents`).

## Risks and open questions

- None.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
