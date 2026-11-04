# Q1g — Worker examples that work outside this repository

| Field | Value |
| --- | --- |
| ID | `Q1g` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1c](./Q1c-examples-checks.md) |
| Blocks | [Q1j](./Q1j-recipes-and-troubleshooting.md) |
| ADRs | None |
| Branch | `claude/working-session-9g3nst` |
| PR title | `refactor(examples): Cloudflare Worker and Web Worker examples without repo internals` |
| Size | S |
| PR | [#216](https://github.com/laudebugs/laminapdf/pull/216) |

## Goal

The Cloudflare Worker example imports fonts from `../../../../../fixtures/`, and the Web Worker client example returns a shape made for its test. This todo makes the Cloudflare Worker self-contained and gives `worker-client` the API a reader needs: `onProgress` and `signal` in, the saved bytes out.

## Context

- [Quality plan §2.2](../../plans/quality.md#22-code-examples); `packages/sdk/examples/worker/src/index.ts`, `packages/sdk/examples/worker-client.ts`, `packages/sdk/test/browser/worker.browser.test.ts`, the workerd runner.

## Scope

**In scope**

- Cloudflare Worker example: fonts from its own folder (copied by the test setup), no repository-relative imports, no cast (after [Q1d](../done/Q1d-example-api-cleanups.md); keep it with a comment if Q1d hasn't landed).
- `worker-client.ts`: `(bytes, { onProgress, signal })` returning `Uint8Array`; its browser test supplies the callbacks and counts the calls.

**Out of scope** (and which todo covers it)

- Other examples ([Q1f](../done/Q1f-examples-readable.md)).

## Tasks

- [x] Claim by opening the work PR (#216) off an up-to-date `main`. The cloud session that did the work had a plain clone and an assigned branch, so it worked on that branch instead of a worktree.
- [x] Change the workerd runner's setup to copy the fonts into the example's folder; change the imports. The Worker also imported the report from a sibling example, which Biome's examples rule refuses too, so its `/report` route became `/embedded`: the same letter in a font bundled from its own `src/fonts/`, which the runner fills with one font.
- [x] Rewrite `worker-client.ts` and update `worker.browser.test.ts`: `saveInWorker(bytes, { onProgress, signal })`; the test counts progress on a save of 21,000 blank pages (more objects than one save step) and checks that a signal aborted at once rejects with `cancelled`. After review, the signal reaches `connect` and `open` too.
- [x] Run the workerd runner and the browser tests; run `docs-snippets` on both files. workerd ran in full; the browser test ran in Chromium (Firefox and WebKit in CI).

## Acceptance criteria

- [x] `docs-snippets` reports nothing for these files.
- [x] The workerd and browser runs pass.

## Post-merge checks

None.

## Tests

The workerd runner and `worker.browser.test.ts`.

## Docs to update

Captions on the Cloudflare Workers and "Keep the page responsive" pages, plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: the work PR moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] Work PR opened; **PR** field filled in. It came from the session's assigned branch `claude/working-session-9g3nst` in a plain clone, not from a worktree or a `refactor/…` branch (see Tasks)
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
