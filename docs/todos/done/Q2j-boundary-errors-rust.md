# Q2j — One error object at every Rust boundary

| Field | Value |
| --- | --- |
| ID | `Q2j` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2k](./Q2k-boundary-errors-ts.md) |
| ADRs | Follows 0026 |
| Branch | `refactor/q2j-boundary-errors-rust` |
| PR title | `refactor(errors): every Rust boundary error is the ADR 0026 object from one serialiser` |
| Size | M |
| PR | [#151](https://github.com/laudebugs/lamina/pull/151) |

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

- Did-you-mean ([Q2v](./Q2v-did-you-mean.md)); the TypeScript side ([Q2k](../done/Q2k-boundary-errors-ts.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Add `Diagnostic::binding` and switch lamina-wasm to it (replaced by `Diagnostic::error` and the `with_` methods, see Outcome); delete `BindingDiagnostic`.
- [x] Add `RenderError::diagnostic()`; switch lamina-render-wasm's throw.
- [x] Write the contract test for both modules; run `cargo xtask wasm`.
- [x] Check `@laminapdf/render-hayro`, which reads `.code` from the thrown value: the object still has `code`.

## Acceptance criteria

- [x] Every error thrown by either WASM module validates against `lamina.error/1`.

## Outcome

Every error that crosses into JavaScript is now built as one `Diagnostic` and serialised by one implementation. The record lives somewhere the plan didn't expect, and five details add to the plan.

- **Where `Diagnostic` lives:** in `lamina-syntax`, behind a new `json` feature, not in the façade. `RenderError` is in `lamina-render`, which depends on `lamina-cos` and `lamina-syntax` only, so it couldn't reach a record or a code table in the façade. The generated `ErrorCode` and `WarningCode` tables moved down with the record (`crates/lamina-syntax/src/generated/codes.rs`), and the façade re-exports all three, so `lamina::Diagnostic` and `lamina::ErrorCode` still work. The façade keeps the `explain` pages in its own generated file. ADR 0026 has an implementation note on what this means for the "no hint text in core crates" rule.
- **The constructor:** `Diagnostic::error(code, message)` and `Diagnostic::warning(code)` read the kind, hint and `docsUrl` from the table, and `with_hint`, `with_did_you_mean`, `with_location` and `with_data` add the rest. The façade, `lamina-wasm` and `lamina-render` all build it this way, which replaces the planned `Diagnostic::binding(code, message, did_you_mean, data)`. `BindingDiagnostic` is gone.
- **Render errors:** `RenderError::error_code()` maps each variant to the generated code, so `code()` and `kind()` are read from the table, and `diagnostic()` adds the `data` fields each code's page lists. `lamina-render-wasm` throws that object as a plain object instead of an `Error`, and a `data-pending` carries its ranges in `data.ranges`. `@laminapdf/render-hayro` reads `code`, `message` and `data.ranges` from it. The TypeScript side of the shape is [Q2k](../done/Q2k-boundary-errors-ts.md)'s.
- **Tests:** `packages/wasm/test/smoke.test.ts` provokes seven errors: `not-pdf`, `unknown-option` and `invalid-argument` from the base module, and `render-failed`, `invalid-argument`, `page-index-out-of-range` and `render-too-large` from the hayro module. Ajv checks each one against `lamina.error/1`. `lamina-render`'s unit tests validate every variant's object with `boon` and pin `data-pending`'s ranges as `[offset, length]` pairs under `data.ranges`. The hayro module never raises `data-pending` itself: it gets the whole input.
- **Size:** every module stays within its budget (`cargo xtask size`). The base module is 404,378 bytes gzipped against 410,000, and the hayro module 1,778,573 against 1,855,000. The hayro module grew because it now links serde_json and the hint table.

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
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
