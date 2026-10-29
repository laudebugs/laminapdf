# L1c — Error contract in the Rust core: `codes.json` v2 with kinds and hints, tagged locations, no document text in messages

| Field | Value |
| --- | --- |
| ID | `L1c` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L1](../../plans/shareable-libraries.md#l1-contracts), [§4 principle 2](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) |
| Round / milestone | Round L1 (shareable libraries) — before B5c |
| Depends on | [L1a](../done/L1a-contract-adrs.md) |
| Blocks | [B5c](./B5c-lamina-facade-and-examples.md), [L1b](./L1b-error-contract-codegen.md) |
| ADRs | Implements the Rust-core half of [0026](../../decisions/0026-error-and-diagnostic-contract.md) (§1.1 locations, §2 kinds, §3 codes, §4 no document text, §5 hints); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) §2 and §4 (codes listed once, checked by test) |
| Branch | `feat/l1c-error-contract-core` |
| PR title | `feat(codes): kinds, hints and tagged locations for every error and warning code` |
| Size | M (≈ 300–400 changed lines; `codes.json` and docs pages don't count) |
| PR | — |

## Goal

Give every existing Rust error and warning the parts of ADR 0026 that the façade (B5c) and the first TypeScript package (B6b) need to emit the final shape from day one. That means a `kind`, a `hint`, an unambiguous `location`, and messages without document text. L1b used to own this but comes after B6b, which depends on B5c through B6a. Without this todo, B5c and B6b would ship an error shape that changes later. Code generation, JSON Schema and TS polish stay in [L1b](./L1b-error-contract-codegen.md).

## Context

- [ADR 0026](../../decisions/0026-error-and-diagnostic-contract.md): §1.1 (tagged `location`), §2 (kinds and the initial assignment for every current code), §4 (the three variants that put document text in their messages), §5 (hints), §6 (`docs/errors/<code>.md` pages, which L1a created).
- `crates/lamina-syntax/src/error.rs`, `crates/lamina-cos/src/error.rs`, `crates/lamina-cos/src/reader/warning.rs`, `crates/lamina-ops/src/error.rs`: today's enums and `code()`.
- `crates/lamina-cos/tests/codes.rs`: the test that checks the enums against `fixtures/codes.json`.

## Scope

**In scope**

- **`fixtures/codes.json` v2.** Each warning and error becomes `{ "code", "kind", "summary", "hint", "since" }`. `limits` stay as they are, and retired codes keep `"retired": "<version>"`. Kinds as ADR 0026 §2 assigns them. Hints as ADR 0026 §5 requires: no document text, and parameters only from the fixed vocabulary.
- **`kind()`** (`ErrorKind`, nine kinds) on every crate's `Error` and on `Warning`, as a table lookup by `code()`. A test checks it against `codes.json`. The table is hand-written here; L1b generates it.
- **Tagged positions** (ADR 0026 §1.1). `LimitExceeded { at: Option<u64> }` and the syntax errors' bare `offset`s become a `Position` (`FileOffset(u64)`, `Object(u32)`, `InStream { object: u32, offset: u64 }`). `lamina-cos` tags syntax errors when it wraps them: it knows whether it parsed the file, a read window (convert to a file offset) or an object stream's decoded data. Add `location()` returning the §1.1 fields.
- **No document text in `Display`** (ADR 0026 §4): `UnsupportedFilter`, `UnsupportedSecurityHandler` and `InvalidLog` keep their strings as fields but stop formatting them.
- `docs/errors/<code>.md`: update a page wherever its kind, hint or data fields change.

**Out of scope** (and which todo covers it)

- `lamina::Error`, `hint()`/`docs_url()`/`Serialize` into the §1 shape, and the job codes `cancelled`/`job-finished`/`document-changed`: [B5c](./B5c-lamina-facade-and-examples.md).
- `cargo xtask gen codes`, TS unions, the JSON Schema, the `explain` feature, the marker-text test: [L1b](./L1b-error-contract-codegen.md).

## Tasks

- [ ] Load the `/rust-best-practices` skill; read ADR 0026 and the four error files
- [ ] Convert `fixtures/codes.json` to v2. Fill `kind`, `summary` and `hint` for every code
- [ ] `ErrorKind` (in `lamina-syntax`, re-exported) and `kind()` on every error and on `Warning`
- [ ] `Position` and `location()`. Migrate `LimitExceeded.at` and the syntax offsets. Tag positions in `lamina-cos` where syntax errors are wrapped (in-memory parse, windowed reads, object streams)
- [ ] Remove document text from the three `Display` impls
- [ ] Update `crates/lamina-cos/tests/codes.rs` to read v2 and check kinds; update the tests that assert exact `at` values
- [ ] Update `CLAUDE.md`'s codes sentence and `fixtures/README.md` (v2 format)

## Acceptance criteria

- [ ] Every code in `codes.json` has `kind`, `summary` and `hint`, and the Rust `kind()` agrees for every variant (test)
- [ ] A limit hit inside an object stream reports `InStream { object, offset }`, a nesting error in the file a `FileOffset`, and a reference-chain limit an `Object` (one test each)
- [ ] No `Display` of an error or warning contains a string read from the input: a test builds the three affected errors with a marker string and asserts the marker is absent from `to_string()`
- [ ] `docs/errors/` has a page for every code, and every page's kind line matches `codes.json`

## Post-merge checks

None.

## Tests

`crates/lamina-cos/tests/codes.rs` (v2, kinds). Position tests in `lamina-cos` (object stream, windowed source, in-memory). Marker-string `Display` tests per crate.

## Docs to update

- [ ] `CLAUDE.md` (codes paragraph), `fixtures/README.md`, `docs/errors/` pages that change

## Risks and open questions

- **Windowed offsets:** the parser sees a window, not the file. The window's start offset must be added when tagging, or every offset from a range source is wrong. Test it with a `ByteSource` that is not in memory.
- **Breaking change:** `LimitExceeded.at` changes type. Nothing is published, but every test that matches on it changes.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
