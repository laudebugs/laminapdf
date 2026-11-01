# Q1e — Runnable quick-start apps for every runtime

| Field | Value |
| --- | --- |
| ID | `Q1e` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1c](../done/Q1c-examples-checks.md) |
| Blocks | [Q1h](./Q1h-docs-story.md) |
| ADRs | None |
| Branch | `feat/q1e-quick-start-apps` |
| PR title | `feat(docs): runnable quick-start examples for Node, Deno, Bun and the browser` |
| Size | M |
| PR | — |

## Goal

Three of the six runtime pages show CI test runners, and the browser page shows a test app that builds nothing a reader can see. This todo writes one small, real app per runtime, has the existing test runners import and run them, removes every `test/` path from the Snippet glob, and rewrites the runtime pages around the apps.

## Context

- [Quality plan §2.1](../../plans/quality.md#21-docs-prose-and-navigation); the [examples guideline](../../guidelines/examples.md) ("What an example is").
- `apps/docs/src/content/docs/quick-start/*.mdx`, `apps/docs/src/components/Snippet.astro`, `packages/sdk/test/{deno,bun}/examples.test.ts`, `packages/sdk/test/examples.test.ts`, `packages/sdk/test/vite-app/main.ts`; `cargo xtask pdf-runtimes` and `verify-install` run them.

## Scope

**In scope**

- `packages/sdk/examples/quick-start/{node,deno,bun,browser}.ts`, about 10–20 lines each: `init`, build hello, then write `hello.pdf` (Node, Deno, Bun) or offer it as a download (browser).
- The runners import the apps (the Vite app imports the browser one); the output is still byte-compared.
- No `test/` path in Snippet's glob.
- The six runtime pages: what's different in this runtime, the app, how to run it.

**Out of scope** (and which todo covers it)

- The first-PDF and concept pages ([Q1h](./Q1h-docs-story.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write the four apps, each taking the output path or a byte sink as a parameter so a runner can capture the bytes.
- [ ] Change the Node, Deno, Bun and Vite runners to import the apps; run `cargo xtask pdf-runtimes`.
- [ ] Remove the `test/**` globs from `Snippet.astro`.
- [ ] Rewrite the six runtime pages; run `docs-prose` and `docs-snippets` on them.
- [ ] Check that `cargo xtask verify-install` still passes with the new apps.

## Acceptance criteria

- [ ] No reader page shows a file under `test/`.
- [ ] Each quick-start app runs in its runtime in CI and produces the hello reference PDF.
- [ ] `docs-prose` and `docs-snippets` report nothing for the quick-start pages and apps.

## Post-merge checks

None.

## Tests

The runtime runners (Node, Deno, Bun, Chromium through Vite, workerd) now run the apps; `verify-install` uses them.

## Docs to update

The six runtime pages; `packages/sdk/README.md` if it shows a quick start; plan §9.

## Risks and open questions

Deno and Bun need write permissions in CI: keep the write behind the parameter so the runner can capture bytes instead.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
