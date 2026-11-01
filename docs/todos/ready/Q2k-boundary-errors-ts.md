# Q2k — One error wire shape in TypeScript (fixes lost render ranges)

| Field | Value |
| --- | --- |
| ID | `Q2k` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2j](../done/Q2j-boundary-errors-rust.md) |
| Blocks | None |
| ADRs | None |
| Branch | `fix/q2k-boundary-errors-ts` |
| PR title | `fix(sdk,render,viewer): one error wire shape; render data-pending keeps its ranges across the worker` |
| Size | M |
| PR | — |

## Goal

The worker turns a render error's `ranges` into `{offset, length}` objects, but `LaminaError.ranges` only accepts `[offset, length]` tuples (what Rust emits), so a render `data-pending` that crosses the worker arrives with no ranges. Render errors also have an unused `toJSON`/`reviveRenderError` pair, the worker duck-types instead, and the viewer throws plain objects. This todo leaves one shape (tuples) and one path.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/sdk/src/worker/server.ts:116-136`, `packages/sdk/src/errors.ts:97`, `packages/render/src/index.ts:199-222`, `packages/viewer/src/worker/document.ts:39,93,118`; typescript.md ("never throw plain objects").

## Scope

**In scope**

- `RenderError.toDiagnostic()` returning the ADR 0026 object; the worker's `diagnostic()` calls it; `reviveRenderError` and `SerializedRenderError` deleted.
- Ranges as tuples everywhere.
- The viewer throws `RenderError("cancelled" | "disposed")` and maps its bare `Error`.

**Out of scope** (and which todo covers it)

- Generated render codes ([Q2l](./Q2l-boundary-lists-generated.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write a test where a render `data-pending` crosses the worker; watch it lose its ranges.
- [ ] Add `toDiagnostic`; switch the worker; delete the unused pair.
- [ ] Change the viewer's throws; remove its entries from [Q2b](../done/Q2b-ts-lint-rules.md)'s `useThrowOnlyError` override.
- [ ] Run the Node and browser suites.

## Acceptance criteria

- [ ] The crossing test passes.
- [ ] No plain-object throw in `packages/*/src`.

## Post-merge checks

None.

## Tests

The new worker test; existing contract suites.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
