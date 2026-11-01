# Q2n — Shared code for the render adapters in `@laminapdf/render`

| Field | Value |
| --- | --- |
| ID | `Q2n` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2m](./Q2m-ts-test-utils.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2n-render-adapters` |
| PR title | `refactor(render): shared adapter helpers, one WasmSource type, memory() on RenderBackend` |
| Size | M |
| PR | — |

## Goal

The hayro and PDFium adapters repeat the page-index check, the "bytes could not be read" handling, the input-size limit and the disposed flag; only PDFium merges pending ranges; five packages define their own WASM-source type, and PDFium hand-writes an instantiate that the glue already provides. Move the shared pieces into `@laminapdf/render` and give both adapters the same behaviour.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/render-hayro/src/index.ts:91,119-142,185,193`, `packages/render-pdfium/src/index.ts:46-60,101,127,169-196,243,268`, `packages/render/src/index.ts:11-22,244`, `packages/sdk/src/init.ts:20,216`, `packages/sdk/src/worker/client.ts:43`, `packages/viewer/src/protocol.ts:17`.

## Scope

**In scope**

- `checkPageIndex`, `readFailure`, `readRange` (with range merging), the input limit constant and a disposed guard in `@laminapdf/render`; one `WasmSource` type there, reused by both adapters and the viewer; `memory()` on `RenderBackend`; PDFium uses the glue's init where possible.

**Out of scope** (and which todo covers it)

- Splitting PDFium's `render` closure ([Q2p](./Q2p-ts-complexity-splits.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Move, adapt both adapters, run the contract suite in Node and browsers.

## Acceptance criteria

- [ ] Both adapters pass the shared contract unchanged; both report merged pending ranges (new contract case).

## Post-merge checks

None.

## Tests

Contract suite with a merged-ranges case.

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
