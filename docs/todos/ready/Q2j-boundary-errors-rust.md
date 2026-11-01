# Q2j — One error object at every Rust boundary (render, WASM, did-you-mean)

| Field | Value |
| --- | --- |
| ID | `Q2j` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2k](./Q2k-boundary-errors-ts.md) |
| ADRs | Follows 0026 |
| Branch | `refactor/q2j-boundary-errors` |
| PR title | `refactor(errors): every Rust boundary error is the ADR 0026 object from one serialiser` |
| Size | M |
| PR | — |

## Goal

Errors cross the Rust/JS boundary in two shapes: lamina-wasm throws the ADR 0026 §1 object (through its own `BindingDiagnostic`, a copy of the façade's `Diagnostic`), while lamina-render-wasm throws a JS `Error` with only `code`/`kind`; `RenderError` hand-codes its kinds instead of using the generated `ErrorCode`, so it has no hint or docs URL. Did-you-mean is implemented in Rust and TS with different tie-breaking. Build every boundary error with one serialiser and pin did-you-mean with a shared fixture.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); ADR 0026; `lamina/src/diagnostic.rs:19`, `lamina-wasm/src/error.rs:32,128`, `lamina-render/src/error.rs:107-137`, `lamina-render-wasm/src/lib.rs:249`, `packages/sdk/src/errors.ts:150`.

## Scope

**In scope**

- `Diagnostic::binding(code, message, did_you_mean, data)` in the façade; delete `BindingDiagnostic`.
- `RenderError::diagnostic()` from `ErrorCode`; render-wasm throws the §1 object.
- Did-you-mean: one tie rule (distance, then name); `fixtures/did-you-mean.json` run by a Rust test and a Vitest.
- Contract test: thrown errors from both WASM modules validate against `lamina.error/1` (`boon`, as `crates/lamina/tests/error_contract.rs`).

**Out of scope** (and which todo covers it)

- The TS side ([Q2k](./Q2k-boundary-errors-ts.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Implement, add the fixture and contract tests, run the WASM suites.

## Acceptance criteria

- [ ] Every error thrown by either WASM module validates against `lamina.error/1`.
- [ ] Rust and TS give the same suggestion for every fixture case.

## Post-merge checks

None.

## Tests

Contract tests, the shared fixture.

## Docs to update

ADR 0026 implementation note, plan §9.

## Risks and open questions

@laminapdf/render-hayro callers that read `.code` from an `Error` keep working (the object has `code`); check the adapter.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
