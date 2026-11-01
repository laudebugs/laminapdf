# Q1g — Worker examples that work outside this repository

| Field | Value |
| --- | --- |
| ID | `Q1g` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1c](./Q1c-examples-check.md) |
| Blocks | [Q1j](./Q1j-recipes-and-troubleshooting.md) |
| ADRs | None |
| Branch | `refactor/q1g-worker-examples` |
| PR title | `refactor(examples): Cloudflare Worker and Web Worker examples without repo internals` |
| Size | S |
| PR | — |

## Goal

The Cloudflare Worker example imports fonts from `../../../../../fixtures/` and the Web Worker client example returns a shape made for its test. Make the Cloudflare Worker self-contained (fonts from its own folder, no cast) and give `worker-client` the API a reader needs (`onProgress`, `signal`, returns the saved bytes); its browser test counts the calls instead.

## Context

- [Quality plan §2.2](../../plans/quality.md#22-code-examples); `packages/sdk/examples/worker/src/index.ts`, `packages/sdk/examples/worker-client.ts`, `packages/sdk/test/browser/worker.browser.test.ts`, the workerd runner.

## Scope

**In scope**

- Worker example: a local `fonts/` folder (copied by the test setup or committed if small and OFL), no repo-relative imports, no cast (after [Q1d](./Q1d-example-api-cleanups.md) the cast isn't needed; if Q1d hasn't landed, keep it with a comment).
- `worker-client.ts(bytes, { onProgress, signal })` → `Uint8Array`; the test supplies the callbacks.

**Out of scope** (and which todo covers it)

- Other examples ([Q1f](./Q1f-examples-readable.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Rewrite both; update their runners; run workerd and the browser tests.

## Acceptance criteria

- [ ] `docs-snippets` reports nothing for these files.
- [ ] The workerd and browser runs pass.

## Post-merge checks

None.

## Tests

The workerd runner and `worker.browser.test.ts`.

## Docs to update

"Keep the page responsive" and the Cloudflare Workers page captions.

## Risks and open questions

Committing font files into `examples/` adds bytes: prefer copying them in the test setup.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
