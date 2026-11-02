# Q2p — Split the TypeScript functions over the complexity limit

| Field | Value |
| --- | --- |
| ID | `Q2p` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2b](./Q2b-ts-lint-rules.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2p-ts-complexity-splits` |
| PR title | `refactor(ts): split the eight functions over the complexity limit` |
| Size | M |
| PR | [#191](https://github.com/laudebugs/laminapdf/pull/191) |

## Goal

Eight functions are over Biome's cognitive-complexity limit of 15; the PDFium `render` closure scores 39. This todo splits each into named steps and empties [Q2b](../done/Q2b-ts-lint-rules.md)'s override, so the limit holds everywhere.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/render-pdfium/src/index.ts:169,315`, `packages/sdk/src/worker/server.ts:240,339`, `packages/render/src/index.ts:259`, `packages/sdk/src/source/http.ts:59,124`, `packages/render-hayro/src/index.ts:119`, `packages/sdk/src/worker/client.ts:356`.

## Scope

**In scope**

- PDFium `render` into background fill, drawing with widgets and drawing with the matrix; `execute` into run and reply; `openUrl`'s option splitting; `planTile`'s checks as functions; `http.ts` request and response handling; `PdfiumDocument.open`; hayro `fill`; `WorkerClient.call`.

**Out of scope** (and which todo covers it)

- New behaviour.

## Tasks

- [x] Claim by opening the work PR (#191) from a fresh worktree off `main`.
- [x] Remove the overrides and list what still fails. Six functions did: PDFium's `render` (39), the worker's `execute` (25), the HTTP test server's `send` in `test/source.test.ts` (23), `planTile` (21), `httpRangeSource`'s `fetchRange` (21) and `httpRangeSource` itself (16, and 103 lines). `PdfiumDocument.open`, hayro's `fill` and `WorkerClient.call` were already under the limits after Q2n.
- [x] Split one function per commit, running its package's suite after each (Node; the worker's browser tests in Chromium).
- [x] Remove each package's override as its functions land; no package's `biome.json` turns the rules off.

## Acceptance criteria

- [x] Biome's override for complexity and function length is empty.

## Post-merge checks

None.

## Tests

Existing suites in Node and the three browsers.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
