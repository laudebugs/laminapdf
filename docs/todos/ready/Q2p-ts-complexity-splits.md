# Q2p — Split the TypeScript functions over the complexity limit

| Field | Value |
| --- | --- |
| ID | `Q2p` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2b](./Q2b-maintainability-checks-ts.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2p-ts-complexity` |
| PR title | `refactor(ts): split the eight functions over the complexity limit` |
| Size | M |
| PR | — |

## Goal

Eight functions are over Biome's cognitive-complexity limit of 15 (the PDFium `render` closure scores 39). Split each into named steps and remove them from [Q2b](./Q2b-maintainability-checks-ts.md)'s override list, so the limit holds everywhere.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/render-pdfium/src/index.ts:169,315`, `packages/sdk/src/worker/server.ts:240,339`, `packages/render/src/index.ts:259`, `packages/sdk/src/source/http.ts:59,124`, `packages/render-hayro/src/index.ts:119`, `packages/sdk/src/worker/client.ts:356`.

## Scope

**In scope**

- PDFium `render` → background fill, draw with widgets, draw with matrix; `execute` → run and reply; `openUrl` option splitting; `planTile` checks as functions; `http.ts` request and response handling; `PdfiumDocument.open`; hayro `fill`; `WorkerClient.call`.

**Out of scope** (and which todo covers it)

- New behaviour.

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Split one function per commit with its tests green.

## Acceptance criteria

- [ ] Biome's override list for complexity and function length is empty.

## Post-merge checks

None.

## Tests

Existing suites (Node and browsers).

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
