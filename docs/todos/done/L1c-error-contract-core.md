# L1c — Error contract in the Rust core: `codes.json` v2 with kinds and hints, tagged locations, no document text in messages

| Field | Value |
| --- | --- |
| ID | `L1c` |
| Status | `done` |
| Plan | [shareable-libraries §7 L1](../../plans/shareable-libraries.md#l1-contracts), [§4 principle 2](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) |
| Round / milestone | Round L1 (shareable libraries) — before B5c |
| Depends on | [L1a](./L1a-contract-adrs.md) |
| ADRs | Implements the Rust-core half of [0026](../../decisions/0026-error-and-diagnostic-contract.md) (§1.1 locations, §2 kinds, §3 codes, §4 no document text, §5 hints); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) §2 and §4 (codes listed once, checked by test) |
| Branch | `feat/l1c-error-contract-core` |
| PR title | `feat(codes): kinds, hints and tagged locations for every error and warning code` |
| Size | M (≈ 300–400 changed lines; `codes.json` and docs pages don't count) |
| PR | [#49](https://github.com/laudebugs/lamina/pull/49) |

## Goal

Give every existing Rust error and warning the parts of ADR 0026 that the façade (B5c) and the first TypeScript package (B6b) need to emit the final shape from day one. That means a `kind`, a `hint`, an unambiguous `location`, and messages without document text. L1b used to own this but comes after B6b, which depends on B5c through B6a. Without this todo, B5c and B6b would ship an error shape that changes later. Code generation, JSON Schema and TS polish stay in [L1b](../done/L1b-error-contract-codegen.md).

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
- `cargo xtask gen codes`, TS unions, the JSON Schema, the `explain` feature, the marker-text test: [L1b](../done/L1b-error-contract-codegen.md).

## Tasks

- [x] Load the `/rust-best-practices` skill; read ADR 0026 and the four error files
- [x] Convert `fixtures/codes.json` to v2. Fill `kind`, `summary` and `hint` for every code
- [x] `ErrorKind` (in `lamina-syntax`, re-exported) and `kind()` on every error and on `Warning`
- [x] `Position` and `location()`. Migrate `LimitExceeded.at` and the syntax offsets. Tag positions in `lamina-cos` where syntax errors are wrapped (in-memory parse, windowed reads, object streams)
- [x] Remove document text from the three `Display` impls
- [x] Update `crates/lamina-cos/tests/codes.rs` to read v2 and check kinds; update the tests that assert exact `at` values
- [x] Update `CLAUDE.md`'s codes sentence and `fixtures/README.md` (v2 format)

## Acceptance criteria

- [x] Every code in `codes.json` has `kind`, `summary` and `hint`, and the Rust `kind()` agrees for every variant (test)
- [x] A limit hit inside an object stream reports `InStream { object, offset }`, a nesting error in the file a `FileOffset`, and a reference-chain limit an `Object` (one test each)
- [x] No `Display` of an error or warning contains a string read from the input: a test builds the three affected errors with a marker string and asserts the marker is absent from `to_string()`
- [x] `docs/errors/` has a page for every code, and every page's kind line matches `codes.json`

## Post-merge checks

None.

## Tests

`crates/lamina-cos/tests/codes.rs` (v2, kinds). Position tests in `lamina-cos` (object stream, windowed source, in-memory). Marker-string `Display` tests per crate.

## Docs to update

- [x] `CLAUDE.md` (codes paragraph), `fixtures/README.md`, `docs/errors/` pages that change

## Risks and open questions

- **Windowed offsets:** the parser sees a window, not the file. The window's start offset must be added when tagging, or every offset from a range source is wrong. Test it with a `ByteSource` that is not in memory.
- **Breaking change:** `LimitExceeded.at` changes type. Nothing is published, but every test that matches on it changes.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **Kinds per type.** Each error and warning type's `kind()` is an exhaustive `const` match over its variants (no wildcard, no default kind); wrappers delegate to the wrapped error's `kind()`. Each crate has a unit test with one sample per variant that checks `code()` against `CODES` and `kind()` against `codes.json` (shared helper `crates/lamina-cos/tests/common/codes.rs`, included by `#[path]`). A first version used one `code → kind` table in `lamina-syntax`; review found it inverted the layering and let a missing code default to `internal`.
- **The codes check covers every group.** `every_listed_code_is_complete` reads every top-level group of `codes.json` (not only `warnings`/`errors`), so groups other todos add (B4a's `textErrors`, B9a's content warnings) are held to v2 too: `kind`, `summary`, `hint` and `since`, one of the nine kinds, hint parameters only `{limit}`/`{option}`, and a page with the fixed headings and a matching `- Kind:` line. `every_page_is_for_a_listed_code` checks the other direction.
- **B9a's codes.** After B9a (#39) merged, its ten content warnings got v2 entries (kind `input`), pages and `Warning::location()` arms (`object` + `streamOffset` in the decoded content; none when the content has no object), and `lamina-content` tags its limits: saved-state nesting and string length as `InStream` in the content, Form XObject depth as `Object`.
- **B4a's codes.** After B4a (#48) merged, `textErrors`/`textWarnings` got v2 entries (kinds as its pages give them), `lamina_text::Error` and `Warning` an exhaustive `kind()` and a `location()` (`WrongDocument` at its object; COS errors delegate), unit tests per variant, and `crates/lamina-text/tests/codes.rs` reads v2 (its page check moved to the cos codes test, which covers every group).
- **Positions.** The syntax layer reports offsets in the buffer it parsed as `Position::FileOffset`; `lamina-cos` re-tags them with `Error::map_position` (`Position::shifted` for windows, `Position::in_stream` for object-stream members). The syntax variants' field is now `at: Position` (was `offset: usize`).
- **Deviations.** The marker-text tests are here, not in L1b, because this todo's acceptance criteria ask for them (per crate: `lamina-cos/tests/codes.rs`, `lamina-ops/tests/serde.rs`, `lamina-save/tests/codes.rs`); L1b's fixture-wide check remains. Beyond the three variants ADR 0026 §4 names, `SinkError::{Io, Failed}` (host text that can name a path) and `lamina_model::ParseIdError::Syntax` (caller text) stopped formatting their strings too. `codes.json` v2 also holds the `docsUrl` constant (ADR 0026 §6) and a `version`. The four codes without pages (`saved-not-rebased`, `no-pending-save`, `sink`, `not-rebased`) got them. Review added the code `sink-not-empty` (kind `usage`: a sink that already held bytes is a caller mistake, not an I/O failure), split from `sink`. `lamina_ops::Error::{UnknownPage, DirectPage}` show and locate only object IDs: a path ID's keys come from the document or the op log. `SaveError::NotRebased` gives no location, because its inner error's positions are in the new output, not the input. `ErrorKind` is not `#[non_exhaustive]` (a closed set, noted in `docs/guidelines/rust.md`). `since` is `0.1.0` (the first release) for every current code.
