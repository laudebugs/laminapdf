# L1b — Error and diagnostic contract: codegen from `fixtures/codes.json` into Rust, TypeScript and JSON Schema, with `explain` text

| Field | Value |
| --- | --- |
| ID | `L1b` |
| Status | `done` |
| Plan | [shareable-libraries §7 L1](../../plans/shareable-libraries.md#l1-contracts), [§4 principle 2](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) |
| Round / milestone | Round L1 (shareable libraries) |
| Depends on | [B6b](./B6b-lamina-pdf-package.md), [L1a](./L1a-contract-adrs.md), [L1c](./L1c-error-contract-core.md) |
| Blocks | [L2a](./L2a-worker-client.md), [L2b](../ongoing/L2b-disposal-and-memory.md), [L4a](../ready/L4a-lamina-cli.md), [L5a](../ready/L5a-agent-docs.md) |
| ADRs | Implements 0026 (from [L1a](../done/L1a-contract-adrs.md)); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) §2 and §4 (codes listed once, Rust and TS tests), [0002](../../decisions/0002-binding-strategy.md) §1 (errors map 1:1 to a JS `Error` with `code`), [0018](../../decisions/0018-test-strategy.md) §5 (shared expectations), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (new dev dependencies) |
| Branch | `feat/l1b-error-contract` |
| PR title | `feat(codes): generate the error contract into Rust, TypeScript and JSON Schema with explain text` |
| Size | L (≈ 500–800 lines of generator, tests and explain text; generated files don't count) |
| PR | [#96](https://github.com/laudebugs/lamina/pull/96) |

## Goal

Make ADR 0026's error shape real on every surface, from one file. `fixtures/codes.json` grows from a list of names into the contract: each code gets a `kind`, a `hint` and an `explain` page. `cargo xtask gen codes` generates the Rust code table, the TypeScript `ErrorCode`/`WarningCode` unions with an exhaustively narrowable `LaminaError`, and a JSON Schema for the error object. CI fails when a generated file is stale. Models and people can then self-correct from a code, a hint and a link instead of a free-text message ([report: errors and schemas as a single contract](../../research/libraries/shareable-libraries.md#errors-and-schemas-as-a-single-contract)).

## Context

- [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md) from [L1a](../done/L1a-contract-adrs.md): fields (incl. `data`, the only place for document-derived strings, and `location.pageIndex`/`id`), the `kind` list with an initial assignment for every current code (§2), code and hint rules, no document text in messages, `docsUrl`, TS class name. Decided there: the class is **`LaminaError`** (B6b ships it; `PdfError` is never used); `explain` pages live in **`docs/errors/<code>.md`**; `docsUrl` uses one generated base constant, the GitHub-rendered page until the domain is chosen.
- Already done before this todo (re-sequenced after L1a's review): [L1c](../done/L1c-error-contract-core.md) converts `fixtures/codes.json` to v2 (kind, summary, hint), adds `kind()`/`location()` with tagged positions to the core errors, and removes document text from `Display`. [B5c](../done/B5c-lamina-facade-and-examples.md) adds `lamina::Error`'s `hint()`/`docs_url()`/`Serialize` and the job codes. [B6b](../done/B6b-lamina-pdf-package.md) ships `LaminaError` with every field. This todo replaces those hand-checked tables with generated ones and adds what is listed below.
- [ADR 0003 §2](../../decisions/0003-reader-tolerance-and-limits.md#2-warnings-stable-enumerated-codes) and [§4](../../decisions/0003-reader-tolerance-and-limits.md#4-enforcement): codes in `fixtures/codes.json`, checked by `crates/lamina-cos/tests/codes.rs` and (from B6b) a TS test.
- [B6b](../done/B6b-lamina-pdf-package.md): introduces `cargo xtask gen codes` → `packages/sdk/src/codes.generated.ts` and `LaminaError`; this todo extends both.
- [Report: strict types](../../research/libraries/shareable-libraries.md#strict-types-branded-units-discriminated-options-codes-generated-from-one-file) and the [TypeScript note §6](../../research/libraries/notes/typescript-web-dx.md#6-strict-typescript-types-errors-tooling): `LaminaError<C>`, `isLaminaError(e, code)`.
- [Code organisation §9](../../guidelines/code-organization.md#9-generated-code): `@generated` header, dedicated location, `linguist-generated`, a CI check that regeneration gives no diff.
- [Rust guideline: errors](../../guidelines/rust.md#errors) and [TypeScript guideline: errors](../../guidelines/typescript.md#errors).
- [CLI note §4](../../research/libraries/notes/cli-spec.md#4-agent-readiness-schemas-mcp-security-what-agents-need): `lamina explain <code>` ([L4a](../ready/L4a-lamina-cli.md)) prints the same text.

## Scope

**In scope**

- **`fixtures/codes.json` v2** exists from [L1c](../done/L1c-error-contract-core.md). This todo adds `fixtures/codes.schema.json` (a JSON Schema for the file itself, validated in the test) and fills in anything later todos left thin.
- **`explain` text** per code in `docs/errors/<code>.md` (ADR 0026 §6). L1a created a short page per code; this todo expands each to the full headings, and adds the check that every code has a page and every page has a code.
- **Rust (generated):**
  - The `lamina` façade gets `src/generated/codes.rs` with `ErrorCode`/`WarningCode` enums (`as_str`, `kind`, `hint`, `docs_url`).
  - `lamina::Error` and `Warning` keep B5c's `kind()`, `hint()`, `docs_url()` and `Serialize`, now backed by the generated tables instead of hand-written ones.
  - `lamina::explain(code) -> Option<&'static str>` behind an `explain` feature that is off in the wasm base build, so the text costs no wasm size.
- **TypeScript (generated):**
  - `packages/sdk/src/generated/codes.ts`: the `ErrorCode` and `WarningCode` unions, plus `kind`/`hint`/`docsUrl` tables.
  - `LaminaError` made generic, `LaminaError<C extends ErrorCode>`, so `code` narrows; `toJSON()` validated against the schema.
  - `isLaminaError(e, code?)` that narrows; warnings typed by `WarningCode`.
  - `explain` text as an opt-in subpath (`@lamina/pdf/explain`) so it stays out of the main bundle.
- **JSON Schema (generated):** a 2020-12 schema `lamina.error/1` for the error and warning objects, with `code` as an `enum`. It lives in the location ADR 0026 picks (recommendation: `packages/sdk/schemas/` and the façade crate's package, served by the docs site later). Recommendation: generate it with `schemars` from the Rust types, if its licence and wasm-free use pass `cargo deny`; otherwise emit it from xtask directly.
- **Staleness check:** `cargo xtask gen codes --check` regenerates into memory and fails on any diff. Wired into `cargo xtask check docs` (fast, no build needed). Generated files carry the `@generated` header and are marked `linguist-generated=true`.
- **Codes the plan already needs**, if not added earlier: `disposed` ([L2b](../ongoing/L2b-disposal-and-memory.md)) and `unknown-option` (the "did you mean" hint). `cancelled`, `job-finished` and `document-changed` come with B5c's jobs.

**Out of scope** (and which todo covers it)

- The CLI `explain` command and exit-code mapping: [L4a](../ready/L4a-lamina-cli.md).
- The docs-site code table and error pages: [B7a](./B7a-docs-site.md) (it can consume this generator's `--md` output if L1b merges first; B7a does not depend on L1b, see the order note under Risks); shipping the pages inside packages: [L5a](../ready/L5a-agent-docs.md).
- Swift and Kotlin error types: [L6a](../done/L6a-lamina-ffi-spike.md).
- `lamina.extract/1` and other output schemas: [L3a](../ready/L3a-lamina-extract-reading-order.md), [L4b](../ready/L4b-cli-batch-render-schema.md).

## Tasks

- [x] Load the `/rust-best-practices` skill and follow [docs/guidelines/rust.md](../../guidelines/rust.md); re-read ADR 0026 and B6b's generator on `main`
- [x] Write `fixtures/codes.schema.json` for the v2 file L1c created; review every `kind`, `summary` and `hint` once more (no document text)
- [x] Extend `crates/lamina-cos/tests/codes.rs` (v2 since L1c) to check that no code is missing an `explain` page and no page lacks a code
- [x] Expand each `docs/errors/<code>.md` page (created short in L1a) to the full ADR 0026 §6 headings
- [x] Extend `tools/xtask` `gen codes`:
  - [x] Read v2 and validate it against its schema
  - [x] Emit the Rust, TS and JSON Schema files, plus `--md` for B7a
  - [x] `--check` mode; header `// @generated by cargo xtask gen codes. DO NOT EDIT.`
  - [x] Add a `.gitattributes` entry
- [x] Façade (`crates/lamina`):
  - [x] Include the generated module
  - [x] Replace B5c's hand-written `kind()`/`hint()`/`docs_url()` tables with the generated module. No `match` arms written by hand per code beyond `code()`
  - [x] The `explain` feature
- [x] `packages/sdk`:
  - [x] Replace `codes.generated.ts` with `src/generated/codes.ts`
  - [x] `LaminaError<C extends ErrorCode>` (generic over the generated union)
  - [x] `isLaminaError`, `toJSON()`
  - [x] The `./explain` export in `package.json`, plus publint/attw still clean
- [x] Wire `gen codes --check` into `cargo xtask check docs`; document the command in `CLAUDE.md`

## Acceptance criteria

- [x] Editing `fixtures/codes.json` (adding, removing or renaming a code, or changing a hint) without running `cargo xtask gen codes` makes `cargo xtask check` fail with a message naming the stale file
- [x] A TS type test switches over `LaminaError["code"]` with a `never` default and compiles. Adding a code to `codes.json` and regenerating makes it fail until the case is handled (checked with `expectTypeOf`/`@ts-expect-error` fixtures)
- [x] `isLaminaError(e, "wrong-password")` narrows `e.code` to `"wrong-password"` (type test)
- [x] A Rust error and the same error from TS serialise to JSON that validates against `lamina.error/1` and is field-for-field equal for a shared fixture case (wrong password, limit exceeded, malformed file)
- [x] Every code has non-empty `kind`, `summary`, `hint` and an `explain` page; no `explain` page lacks a code
- [x] No error `message` or `hint` contains document text: a test opens a fixture whose names, strings and `/Info` values carry a marker string, triggers each reachable error and warning, and asserts the marker never appears in `message` or `hint`
- [x] The wasm base module size does not grow by more than the generated tables (no `explain` text in it); measured by `cargo xtask size` against ADR 0016's budget

## Post-merge checks

None.

## Tests

- Rust: `crates/lamina-cos/tests/codes.rs` (v2, completeness); `crates/lamina/tests/error_contract.rs` (serialisation, schema validation with a JSON Schema dev dependency such as `jsonschema`, after a licence check; the no-document-text test over a new fixture `fixtures/corpus/marker-text.pdf` with provenance in `fixtures/README.md`).
- TS: `packages/sdk/test/codes.test.ts` (codes equal `codes.json`, exhaustive narrowing type tests, `toJSON()` validated with `ajv` or the same schema validator, shared JSON expectations per ADR 0018 §5).
- xtask: unit tests for the generator (golden output for a small `codes.json`) and for `--check` failing on a diff.

## Docs to update

- [x] [shareable-libraries §9 L1](../../plans/shareable-libraries.md#9-to-dos): tick the codegen to-do
- [x] `CLAUDE.md` (the codes paragraph and the `cargo xtask gen codes` command), `fixtures/README.md` (v2 format, new fixture), `docs/README.md` (`docs/errors/`)
- [x] [TypeScript guideline: errors](../../guidelines/typescript.md#errors) and [code organisation §5](../../guidelines/code-organization.md#5-error-handling-by-layer), if L1a left the class rename to this PR
- [x] `packages/sdk/README.md`: error handling section with `isLaminaError`

## Risks and open questions

- **Order with B7a:** B7a also plans `gen codes --md`. Whichever merges second extends the other's generator rather than adding a second one.
- **Hand tables vs generated:** L1c and B5c keep hand-written kind/hint tables checked by test. The generator must reproduce them exactly, so the first generated diff is empty.
- **`didYouMean`** needs the set of valid option names per call. Recommendation: generate it from the tsify option types in [B6b](../done/B6b-lamina-pdf-package.md)'s runtime unknown-key check, and leave it empty where no list exists.
- **`schemars` vs hand-written schema:** schemars keeps the schema next to the Rust types but adds a dependency to the façade. Decide in the PR, behind a non-default feature if kept.
- **Hint quality** is a writing task as much as a code task. Ask the reviewer to read every hint once.

## Outcome

- **Generated where:** `cargo xtask gen codes` (`tools/xtask/src/codegen.rs`) validates `fixtures/codes.json` against `fixtures/codes.schema.json` (with `boon`), then writes `crates/lamina/src/generated/codes.rs` (`ErrorCode`, `WarningCode`, `explain` behind the façade's `explain` feature, via `include_str!` of the pages), `packages/sdk/src/generated/codes.ts` and `explain.ts`, `lamina.error/1` in `packages/sdk/schemas/` and `crates/lamina/schemas/`, and the tables of `docs/errors/README.md` (also printed by `--md`). `--check` runs in `cargo xtask check docs` (no build, no `node_modules`), so the TS files are written as they are and excluded from Biome rather than passed through `biome format`.
- **Schema:** emitted by xtask, not `schemars`: the façade gains no dependency, and the code enums come from `codes.json` rather than from Rust types. Validation uses `boon` (Rust, xtask and tests) and `ajv` (TS tests) instead of `jsonschema` (a larger graph with a network resolver by default).
- **Tables:** the façade's `kind()`, `hint()`, `docs_url()` and warning summaries, and `lamina-wasm`'s binding-code table, now come from the generated enums. The core crates keep their own `kind()` matches (L1c, checked by their tests): they can't depend on the façade, and generating into each core crate was not in this todo's scope. The first generation reproduced every kind, summary and hint (the façade's `generated_tables_match_codes_json` test); the deliberate differences are the TS file's group order (now `codes.json` order instead of alphabetical by group key), the generated `docs/errors/README.md` tables gaining a summary column, and the new `disposed` code.
- **Explain in TS:** `@lamina/pdf/explain` exports `explain`, `KINDS`, `HINTS`, `ERROR_CODES`, `WARNING_CODES` and `docsUrl`; the main entry exports only the types, so its bundle is unchanged.
- **Fixtures:** the shared cases are `fixtures/errors/contract.json`, the marker fixture `fixtures/errors/marker-text.pdf` (not `corpus/`, whose files all carry pypdf facts). The limit case reads the page count after opening: no hard limit fails `open` itself (repair turns it into `missing-root`).
- **`didYouMean`:** unchanged (B6b's runtime unknown-key check fills it); no new candidate lists.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
