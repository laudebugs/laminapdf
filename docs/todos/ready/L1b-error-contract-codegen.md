# L1b — Error and diagnostic contract: codegen from `fixtures/codes.json` into Rust, TypeScript and JSON Schema, with `explain` text

| Field | Value |
| --- | --- |
| ID | `L1b` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L1](../../plans/shareable-libraries.md#l1-contracts), [§4 principle 2](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00240029) |
| Round / milestone | Round L1 (shareable libraries) |
| Depends on | [B6b](./B6b-lamina-pdf-package.md), [L1a](./L1a-contract-adrs.md) |
| Blocks | [L2a](./L2a-worker-client.md), [L2b](./L2b-disposal-and-memory.md), [L4a](./L4a-lamina-cli.md), [L5a](./L5a-agent-docs.md) |
| ADRs | Implements 0024 (from [L1a](./L1a-contract-adrs.md)); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) §2 and §4 (codes listed once, Rust and TS tests), [0002](../../decisions/0002-binding-strategy.md) §1 (errors map 1:1 to a JS `Error` with `code`), [0018](../../decisions/0018-test-strategy.md) §5 (shared expectations), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (new dev dependencies) |
| Branch | `feat/l1b-error-contract` |
| PR title | `feat(codes): generate the error contract into Rust, TypeScript and JSON Schema with explain text` |
| Size | L (≈ 500–800 lines of generator, tests and explain text; generated files don't count) |
| PR | — |

## Goal

Make ADR 0024's error shape real on every surface, from one file. `fixtures/codes.json` grows from a list of names into the contract: each code gets a `kind`, a `hint` and an `explain` page. `cargo xtask gen codes` generates the Rust code table, the TypeScript `ErrorCode`/`WarningCode` unions with an exhaustively narrowable `LaminaError`, and a JSON Schema for the error object. CI fails when a generated file is stale. Models and people can then self-correct from a code, a hint and a link instead of a free-text message ([report: errors and schemas as a single contract](../../research/libraries/shareable-libraries.md#errors-and-schemas-as-a-single-contract)).

## Context

- ADR 0024 from [L1a](./L1a-contract-adrs.md): fields, `kind` list, code and hint rules, no document text in messages, `docsUrl`, where `explain` text lives, TS class name.
- [ADR 0003 §2](../../decisions/0003-reader-tolerance-and-limits.md#2-warnings-stable-enumerated-codes) and [§4](../../decisions/0003-reader-tolerance-and-limits.md#4-enforcement): codes in `fixtures/codes.json`, checked by `crates/lamina-cos/tests/codes.rs` and (from B6b) a TS test.
- [B6b](./B6b-lamina-pdf-package.md): introduces `cargo xtask gen codes` → `packages/pdf/src/codes.generated.ts` and `PdfError`; this todo extends both.
- [Report: strict types](../../research/libraries/shareable-libraries.md#strict-types-branded-units-discriminated-options-codes-generated-from-one-file) and the [TypeScript note §6](../../research/libraries/notes/typescript-web-dx.md#6-strict-typescript-types-errors-tooling): `LaminaError<C>`, `isLaminaError(e, code)`.
- [Code organisation §9](../../guidelines/code-organization.md#9-generated-code): `@generated` header, dedicated location, `linguist-generated`, a CI check that regeneration gives no diff.
- [Rust guideline: errors](../../guidelines/rust.md#errors) and [TypeScript guideline: errors](../../guidelines/typescript.md#errors).
- [CLI note §4](../../research/libraries/notes/cli-spec.md#4-agent-readiness-schemas-mcp-security-what-agents-need): `lamina explain <code>` ([L4a](./L4a-lamina-cli.md)) prints the same text.

## Scope

**In scope**

- **`fixtures/codes.json` v2.** Each warning and error becomes an object: `{ "code", "kind", "summary", "hint", "since" }`; `limits` stay as they are. Retired codes are kept with `"retired": "<version>"`. A JSON Schema for the file itself (`fixtures/codes.schema.json`) is validated in the test.
- **`explain` text** per code, in the location ADR 0024 picks. Recommendation: `docs/errors/<code>.md`, with a fixed set of headings (What happened, Why, How to fix, Example). Every code in `codes.json` has a page, and every page has a code.
- **Rust (generated):**
  - The `lamina` façade gets `src/generated/codes.rs` with `ErrorCode`/`WarningCode` enums (`as_str`, `kind`, `hint`, `docs_url`).
  - `lamina::Error` and `Warning` gain `kind()`, `hint()`, `docs_url()` and `serde::Serialize` into the ADR 0024 shape.
  - `lamina::explain(code) -> Option<&'static str>` behind an `explain` feature that is off in the wasm base build, so the text costs no wasm size.
- **TypeScript (generated):**
  - `packages/pdf/src/generated/codes.ts`: the `ErrorCode` and `WarningCode` unions, plus `kind`/`hint`/`docsUrl` tables.
  - `LaminaError<C extends ErrorCode>` (or `PdfError`, as ADR 0024 decides) with `code`, `kind`, `hint`, `location`, `docsUrl` and `toJSON()`.
  - `isLaminaError(e, code?)` that narrows; warnings typed by `WarningCode`.
  - `explain` text as an opt-in subpath (`@lamina/pdf/explain`) so it stays out of the main bundle.
- **JSON Schema (generated):** a 2020-12 schema `lamina.error/1` for the error and warning objects, with `code` as an `enum`. It lives in the location ADR 0024 picks (recommendation: `packages/pdf/schemas/` and the façade crate's package, served by the docs site later). Recommendation: generate it with `schemars` from the Rust types, if its licence and wasm-free use pass `cargo deny`; otherwise emit it from xtask directly.
- **Staleness check:** `cargo xtask gen codes --check` regenerates into memory and fails on any diff. Wired into `cargo xtask check docs` (fast, no build needed). Generated files carry the `@generated` header and are marked `linguist-generated=true`.
- **Codes the plan already needs**, added through this process: `disposed` ([L2b](./L2b-disposal-and-memory.md)), `cancelled` (ADR 0026), `unknown-option` (the "did you mean" hint), plus any B2b added (`wrong-password`).

**Out of scope** (and which todo covers it)

- The CLI `explain` command and exit-code mapping: [L4a](./L4a-lamina-cli.md).
- The docs-site code table and error pages: [B7a](./B7a-docs-site.md) (it can consume this generator's `--md` output if L1b merges first; B7a does not depend on L1b, see the order note under Risks); shipping the pages inside packages: [L5a](./L5a-agent-docs.md).
- Swift and Kotlin error types: [L6a](./L6a-lamina-ffi-spike.md).
- `lamina.extract/1` and other output schemas: [L3a](./L3a-lamina-extract-reading-order.md), [L4b](./L4b-cli-batch-render-schema.md).

## Tasks

- [ ] Load the `/rust-best-practices` skill and follow [docs/guidelines/rust.md](../../guidelines/rust.md); re-read ADR 0024 and B6b's generator on `main`
- [ ] Convert `fixtures/codes.json` to v2. Write `fixtures/codes.schema.json`. Fill `kind`, `summary` and `hint` for every current code (11 warnings, 16 errors on today's `main`, plus later additions). Keep hints free of document text
- [ ] Update `crates/lamina-cos/tests/codes.rs` to read v2. It still asserts the Rust enums expose exactly the listed codes, and also that none is missing a `kind`, `hint` or `explain` page
- [ ] Write `docs/errors/<code>.md` (or the ADR's location) for each code; add the folder to `docs/README.md`
- [ ] Extend `tools/xtask` `gen codes`:
  - [ ] Read v2 and validate it against its schema
  - [ ] Emit the Rust, TS and JSON Schema files, plus `--md` for B7a
  - [ ] `--check` mode; header `// @generated by cargo xtask gen codes. DO NOT EDIT.`
  - [ ] Add a `.gitattributes` entry
- [ ] Façade (`crates/lamina`):
  - [ ] Include the generated module
  - [ ] Implement `kind()`/`hint()`/`docs_url()`/`Serialize` on `Error` and `Warning` by mapping each variant's `code()`. No new `match` arms written by hand per code beyond `code()`
  - [ ] The `explain` feature
- [ ] `lamina-wasm`: errors cross as a JS `Error` carrying the ADR 0024 fields (ADR 0002 §1), not just `code`
- [ ] `packages/pdf`:
  - [ ] Replace `codes.generated.ts` with `src/generated/codes.ts`
  - [ ] `LaminaError` (rename from `PdfError` if ADR 0024 says so, keeping a deprecated alias `PdfError` until the first published release)
  - [ ] `isLaminaError`, `toJSON()`
  - [ ] The `./explain` export in `package.json`, plus publint/attw still clean
- [ ] Add `kind` and `hint` to warnings in `doc.warnings` (Rust and TS)
- [ ] Wire `gen codes --check` into `cargo xtask check docs`; document the command in `CLAUDE.md`

## Acceptance criteria

- [ ] Editing `fixtures/codes.json` (adding, removing or renaming a code, or changing a hint) without running `cargo xtask gen codes` makes `cargo xtask check` fail with a message naming the stale file
- [ ] A TS type test switches over `LaminaError["code"]` with a `never` default and compiles. Adding a code to `codes.json` and regenerating makes it fail until the case is handled (checked with `expectTypeOf`/`@ts-expect-error` fixtures)
- [ ] `isLaminaError(e, "wrong-password")` narrows `e.code` to `"wrong-password"` (type test)
- [ ] A Rust error and the same error from TS serialise to JSON that validates against `lamina.error/1` and is field-for-field equal for a shared fixture case (wrong password, limit exceeded, malformed file)
- [ ] Every code has non-empty `kind`, `summary`, `hint` and an `explain` page; no `explain` page lacks a code
- [ ] No error `message` or `hint` contains document text: a test opens a fixture whose names, strings and `/Info` values carry a marker string, triggers each reachable error and warning, and asserts the marker never appears in `message` or `hint`
- [ ] The wasm base module size does not grow by more than the generated tables (no `explain` text in it); measured by `cargo xtask size` against ADR 0016's budget

## Post-merge checks

None.

## Tests

- Rust: `crates/lamina-cos/tests/codes.rs` (v2, completeness); `crates/lamina/tests/error_contract.rs` (serialisation, schema validation with a JSON Schema dev dependency such as `jsonschema`, after a licence check; the no-document-text test over a new fixture `fixtures/corpus/marker-text.pdf` with provenance in `fixtures/README.md`).
- TS: `packages/pdf/test/codes.test.ts` (codes equal `codes.json`, exhaustive narrowing type tests, `toJSON()` validated with `ajv` or the same schema validator, shared JSON expectations per ADR 0018 §5).
- xtask: unit tests for the generator (golden output for a small `codes.json`) and for `--check` failing on a diff.

## Docs to update

- [ ] [shareable-libraries §9 L1](../../plans/shareable-libraries.md#9-to-dos): tick the codegen to-do
- [ ] `CLAUDE.md` (the codes paragraph and the `cargo xtask gen codes` command), `fixtures/README.md` (v2 format, new fixture), `docs/README.md` (`docs/errors/`)
- [ ] [TypeScript guideline: errors](../../guidelines/typescript.md#errors) and [code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer), if L1a left the class rename to this PR
- [ ] `packages/pdf/README.md`: error handling section with `isLaminaError`

## Risks and open questions

- **Order with B7a:** B7a also plans `gen codes --md`. Whichever merges second extends the other's generator rather than adding a second one.
- **Rename churn:** if B6b already shipped `PdfError` in examples and docs, keep the alias and record the rename for [L5a](./L5a-agent-docs.md)'s rename log.
- **`didYouMean`** needs the set of valid option names per call. Recommendation: generate it from the tsify option types in [B6b](./B6b-lamina-pdf-package.md)'s runtime unknown-key check, and leave it empty where no list exists.
- **`schemars` vs hand-written schema:** schemars keeps the schema next to the Rust types but adds a dependency to the façade. Decide in the PR, behind a non-default feature if kept.
- **Hint quality** is a writing task as much as a code task. Ask the reviewer to read every hint once.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
