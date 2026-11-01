# Q1e — Runnable quick-start apps for every runtime

| Field | Value |
| --- | --- |
| ID | `Q1e` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1c](./Q1c-examples-check.md) |
| Blocks | [Q1h](./Q1h-docs-story.md) |
| ADRs | None |
| Branch | `feat/q1e-quick-start-apps` |
| PR title | `feat(docs): runnable quick-start examples for Node, Deno, Bun and the browser` |
| Size | M |
| PR | — |

## Goal

Three of the six runtime pages show CI test runners and the browser page shows a test app that builds nothing a reader can see. Write one small, real app per runtime (Node, Deno and Bun write `hello.pdf`; the browser offers it as a download), have the existing test runners import and run them, take every `test/` path out of the Snippet glob, and rewrite the six runtime pages around them.

## Context

- [Quality plan §2.1](../../plans/quality.md#21-docs-prose-and-navigation), [examples guideline](../../guidelines/examples.md) rule 1.
- `apps/docs/src/content/docs/quick-start/*.mdx`, `apps/docs/src/components/Snippet.astro`, `packages/sdk/test/{deno,bun}/examples.test.ts`, `packages/sdk/test/examples.test.ts`, `packages/sdk/test/vite-app/main.ts`, `cargo xtask pdf-runtimes` and `verify-install` (they run these).

## Scope

**In scope**

- `packages/sdk/examples/quick-start/{node,deno,bun,browser}.ts`, about 10–20 lines each: `init`, build hello, write or download; plus a Rust quick start if `crates/lamina/examples/hello.rs` isn't already one.
- The runners import them (the Vite app imports the browser one); output still byte-compared.
- Snippet's glob loses every `test/` path.
- Rewrite the six runtime pages: what's different in this runtime, then the app, then how to run it.

**Out of scope** (and which todo covers it)

- The concepts and "Your first PDF" pages ([Q1h](./Q1h-docs-story.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write the four apps; wire them into the runners; update the glob; rewrite the pages; run `cargo xtask pdf-runtimes` and the docs build.

## Acceptance criteria

- [ ] No reader page shows a file under `test/` (the Q1c check reports none).
- [ ] Each quick-start app runs in its runtime in CI and produces the hello reference PDF.
- [ ] `docs-prose` and `docs-snippets` report nothing for the quick-start pages.

## Post-merge checks

None.

## Tests

The existing runtime runners (Node, Deno, Bun, Chromium via Vite, workerd) now run the quick-start apps; `verify-install` uses them.

## Docs to update

The six runtime pages, `packages/sdk/README.md` if it shows a quick start.

## Risks and open questions

Deno and Bun need file-write permissions in CI; keep the write behind a parameter so the runner can capture bytes instead.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
