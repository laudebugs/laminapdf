# Q1g — Worker examples that work outside this repository

| Field | Value |
| --- | --- |
| ID | `Q1g` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1c](../ongoing/Q1c-examples-checks.md) |
| Blocks | [Q1j](./Q1j-recipes-and-troubleshooting.md) |
| ADRs | None |
| Branch | `refactor/q1g-worker-examples` |
| PR title | `refactor(examples): Cloudflare Worker and Web Worker examples without repo internals` |
| Size | S |
| PR | — |

## Goal

The Cloudflare Worker example imports fonts from `../../../../../fixtures/`, and the Web Worker client example returns a shape made for its test. This todo makes the Cloudflare Worker self-contained and gives `worker-client` the API a reader needs: `onProgress` and `signal` in, the saved bytes out.

## Context

- [Quality plan §2.2](../../plans/quality.md#22-code-examples); `packages/sdk/examples/worker/src/index.ts`, `packages/sdk/examples/worker-client.ts`, `packages/sdk/test/browser/worker.browser.test.ts`, the workerd runner.

## Scope

**In scope**

- Cloudflare Worker example: fonts from its own folder (copied by the test setup), no repository-relative imports, no cast (after [Q1d](./Q1d-example-api-cleanups.md); keep it with a comment if Q1d hasn't landed).
- `worker-client.ts`: `(bytes, { onProgress, signal })` returning `Uint8Array`; its browser test supplies the callbacks and counts the calls.

**Out of scope** (and which todo covers it)

- Other examples ([Q1f](./Q1f-examples-readable.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Change the workerd runner's setup to copy the fonts into the example's folder; change the imports.
- [ ] Rewrite `worker-client.ts` and update `worker.browser.test.ts`.
- [ ] Run the workerd runner and the browser tests; run `docs-snippets` on both files.

## Acceptance criteria

- [ ] `docs-snippets` reports nothing for these files.
- [ ] The workerd and browser runs pass.

## Post-merge checks

None.

## Tests

The workerd runner and `worker.browser.test.ts`.

## Docs to update

Captions on the Cloudflare Workers and "Keep the page responsive" pages, plan §9.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
