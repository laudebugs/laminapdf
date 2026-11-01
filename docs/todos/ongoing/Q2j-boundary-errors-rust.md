# Q2j — One error object at every Rust boundary

| Field | Value |
| --- | --- |
| ID | `Q2j` |
| Status | `ongoing` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2k](../ready/Q2k-boundary-errors-ts.md) |
| ADRs | Follows 0026 |
| Branch | `refactor/q2j-boundary-errors-rust` |
| PR title | `refactor(errors): every Rust boundary error is the ADR 0026 object from one serialiser` |
| Size | M |
| PR | — |

## Goal

Errors cross the Rust and JavaScript boundary in two shapes. lamina-wasm throws the ADR 0026 object through its own `BindingDiagnostic`, a copy of the façade's `Diagnostic`; lamina-render-wasm throws a JavaScript `Error` with only `code` and `kind`; and `RenderError` hand-codes its kinds instead of using the generated `ErrorCode`, so it has no hint or docs URL. This todo builds every boundary error with one serialiser.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); ADR 0026; `lamina/src/diagnostic.rs:19`, `lamina-wasm/src/error.rs:33`, `lamina-render/src/error.rs:107-137`, `lamina-render-wasm/src/lib.rs:249`; `crates/lamina/tests/error_contract.rs` (the `boon` schema check to reuse).

## Scope

**In scope**

- `Diagnostic::binding(code, message, did_you_mean, data)` in the façade; `BindingDiagnostic` deleted.
- `RenderError::diagnostic()` built from `ErrorCode`; lamina-render-wasm throws the ADR 0026 object.
- A contract test: errors thrown by both WASM modules validate against `lamina.error/1`.

**Out of scope** (and which todo covers it)

- Did-you-mean ([Q2v](../ready/Q2v-did-you-mean.md)); the TypeScript side ([Q2k](../ready/Q2k-boundary-errors-ts.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add `Diagnostic::binding` and switch lamina-wasm to it; delete `BindingDiagnostic`.
- [ ] Add `RenderError::diagnostic()`; switch lamina-render-wasm's throw.
- [ ] Write the contract test for both modules; run `cargo xtask wasm`.
- [ ] Check `@laminapdf/render-hayro`, which reads `.code` from the thrown value: the object still has `code`.

## Acceptance criteria

- [ ] Every error thrown by either WASM module validates against `lamina.error/1`.

## Post-merge checks

None.

## Tests

The contract test; the WASM and render suites.

## Docs to update

ADR 0026's implementation note, plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
