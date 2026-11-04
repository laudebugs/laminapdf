# Q1j — Recipes, one page per example, and troubleshooting

| Field | Value |
| --- | --- |
| ID | `Q1j` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1f](./Q1f-examples-readable.md), [Q1g](./Q1g-worker-examples.md), [Q1h](./Q1h-docs-story.md) |
| Blocks | [Q1l](../ongoing/Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `claude/working-session-9g3nst` |
| PR title | `docs(site): recipes, one per example, and troubleshooting` |
| Size | M |
| PR | [#218](https://github.com/laudebugs/laminapdf/pull/218) |

## Goal

This todo splits the 88-line Examples page into Recipes, one page per example (what you'll make, the code, how it works, what to change), gives the Cloudflare Worker a real recipe, and adds Troubleshooting: symptom, cause and fix for the problems readers hit.

## Context

- [Quality plan §3](../../plans/quality.md#3-the-docs-as-a-story); `apps/docs/src/content/docs/examples/index.mdx`; the examples after Q1f and Q1g.
- Troubleshooting entries from the review: `init` not called; text or extract module not loaded; the CSP needs `wasm-unsafe-eval`; Vite pre-bundling; a Cloudflare Worker can't compile WASM at runtime; wrong password; a signed file won't save; memory grows because documents aren't freed.

## Scope

**In scope**

- `recipes/*.mdx`, one per example; `troubleshooting.mdx`; sidebar entries; a redirect from `/examples/`.

**Out of scope** (and which todo covers it)

- Quick starts ([Q1e](../done/Q1e-quick-start-apps.md)).

## Tasks

- [x] Claim by opening the work PR (#218) off an up-to-date `main`. The cloud session that did the work had a plain clone and an assigned branch, so it worked on that branch instead of a worktree.
- [x] Write one recipe page per example from the rewritten examples, each with a screenshot or description of the result. Thirteen recipes and an index that lists them by task; each links its reference PDF where one exists and describes the result otherwise. Three helper agents drafted twelve of them from the first; every claim was then checked against the example and the source.
- [x] Write Troubleshooting from the error codes and the guides' asides; link each entry to its error page or guide. Nine entries: the review's eight, with the text module and the extract module as two.
- [x] Update the sidebar; redirect `/examples/`. Recipes and Troubleshooting come before Reference; the reference PDFs stay at `/examples/pdf/`, and the guides' anchors into the old page point at their recipes.
- [x] Run `docs-prose` and `docs-snippets`; run `/writing-review` after opening the PR.

## Acceptance criteria

- [x] Every example has exactly one recipe page; `docs-prose` reports nothing on them.
- [x] Each troubleshooting entry links to the page that explains it.

## Post-merge checks

None.

## Tests

`docs-prose`, `docs-snippets` (every example used once), the docs build.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: the work PR moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] Work PR opened; **PR** field filled in. It came from the session's assigned branch `claude/working-session-9g3nst` in a plain clone, not from a worktree or a `docs/…` branch (see Tasks)
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
