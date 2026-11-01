# Q1j — Recipes (one page per example) and a Troubleshooting page

| Field | Value |
| --- | --- |
| ID | `Q1j` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1f](./Q1f-examples-readable.md), [Q1g](./Q1g-worker-examples.md), [Q1h](./Q1h-docs-story.md) |
| Blocks | [Q1l](./Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `docs/q1j-recipes-and-troubleshooting` |
| PR title | `docs(site): recipes, one per example, and troubleshooting` |
| Size | M |
| PR | — |

## Goal

Split the 88-line Examples page into Recipes, one page per example (what you'll make, the code, how it works, what to change), give the Cloudflare Worker a real recipe, and add Troubleshooting: symptom → cause → fix for the problems readers hit.

## Context

- [Quality plan §3](../../plans/quality.md#3-the-docs-as-a-story); `apps/docs/src/content/docs/examples/index.mdx`; the examples after Q1f/Q1g.
- Troubleshooting entries from the review: `init` not called; text or extract module not loaded; CSP needs `wasm-unsafe-eval`; Vite pre-bundling; Workers can't compile WASM at runtime; wrong password; a signed file won't save; memory grows (documents not freed).

## Scope

**In scope**

- `recipes/*.mdx`, one per example; `troubleshooting.mdx`; sidebar entries; a redirect from `/examples/`.

**Out of scope** (and which todo covers it)

- Quick starts ([Q1e](./Q1e-quick-start-apps.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write the recipe pages from the rewritten examples; write Troubleshooting from the error codes and the guides' asides; link each entry to its error-code page.

## Acceptance criteria

- [ ] Every example has exactly one recipe page; `docs-prose --strict` passes on them.
- [ ] Each Troubleshooting entry links to the guide or code page that explains it.

## Post-merge checks

None.

## Tests

`docs-prose`, `docs-snippets` (every example used once), the docs build.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
