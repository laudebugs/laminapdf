# B2a — `Limits`, warning codes, strict mode and the owned `Document`

| Field | Value |
| --- | --- |
| ID | `B2a` |
| Status | `done` |
| Plan | [pdf-library §7 B2](../../plans/pdf-library.md#b2-document-model), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B1a](./B1a-round-b-adrs.md) |
| Blocks | [B2b](../ready/B2b-standard-security-decryption.md), [B2c](../ready/B2c-lamina-model-corpus-fuzz.md), [B2d](../ready/B2d-windowed-reading.md) |
| ADRs | Implements [0003](../../decisions/0003-reader-tolerance-and-limits.md) §2–§4 and [0005](../../decisions/0005-document-model.md) §2 and §7; follows [0018](../../decisions/0018-test-strategy.md) (no wall-clock assertions in PR lanes) |
| Branch | `feat/b2a-limits-owned-document` |
| PR title | `feat(core): Limits, warning codes, strict mode and an owned Document over ByteSource` |
| Size | L (600–1,500 lines; mostly `lamina-cos` reader changes and tests) |
| PR | [#13](https://github.com/laudebugs/lamina/pull/13) |

## Goal

Make the reader's safety policy explicit and configurable, and make `Document` something a binding or a shell can hold. Today two limits are configurable and the rest are private constants; warnings are two un-coded variants; and `lamina_cos::Document<'a>` borrows a `&[u8]` and caches in `Rc`, so it is `!Send` and can't own an mmap, a range cache or a WASM buffer. Every later Round B todo (decryption, typed views, ops, save) builds on the `Limits`/`OpenOptions`/`Document` shape this PR fixes.

## Context

- [ADR 0003](../../decisions/0003-reader-tolerance-and-limits.md): §2 warning codes (table with `B2` status), §3 `Limits` table (kind, default, behaviour, today's constant), §4 enforcement (paired at-limit / past-limit tests, `fixtures/codes.json`).
- [ADR 0005 §2](../../decisions/0005-document-model.md#2-bytesource-and-ownership-of-the-input): `ByteSource` trait, `Arc<dyn ByteSource>`, `SourceError::Pending` → `Error::DataPending`, windowed parsing for sources without `as_slice`; [§7](../../decisions/0005-document-model.md#7-threads): `Rc` → `Arc`, `Document: Send + !Sync`.
- [ADR 0018 §1](../../decisions/0018-test-strategy.md#1-tiers): "the 30 s budget asserted in `tests/robustness.rs` moves to nextest configuration in B2".
- Code: [`crates/lamina-cos/src/reader/mod.rs`](../../../crates/lamina-cos/src/reader/mod.rs) (`OpenOptions`, `Warning`, `Document<'a>`, `open`, `open_with`), [`reader/xref.rs`](../../../crates/lamina-cos/src/reader/xref.rs), [`reader/repair.rs`](../../../crates/lamina-cos/src/reader/repair.rs), [`reader/objstm.rs`](../../../crates/lamina-cos/src/reader/objstm.rs), [`lamina-syntax/src/parser.rs`](../../../crates/lamina-syntax/src/parser.rs) (`MAX_DEPTH`), [`filters.rs`](../../../crates/lamina-syntax/src/filters.rs).
- [Rust guideline: errors and API design](../../guidelines/rust.md#api-design): private fields, `const fn` builders, `#[non_exhaustive]`.

## Scope

**In scope**

- `lamina_syntax::Limits` (`Copy`, private fields, `const fn` getters and `with_*` builders, `Limits::unlimited()`), with every field from ADR 0003 §3 including the reserved ones (`max_content_operators`, `max_xobject_depth`, `max_image_pixels`) so later todos only use them. Re-exported by `lamina-cos`.
- Hard limits raise `Error::LimitExceeded { limit: Limit, at: Option<u64> }` (replacing `TooDeep`, `DecompressLimit` and the `ReferenceCycle` used for resolve depth); tolerant paths (page-tree walk, xref load → repair, repair skipping an object stream) degrade with their warning.
- Soft limits drop the item and warn (`max_xref_sections`, `max_object_number`, `max_warnings`).
- `Warning` as a `#[non_exhaustive]` enum with every ADR 0003 §2 code (`missing-header`, `xref-repaired`, `xrefstm-ignored`, `stream-length-recovered`, `stale-generation`, `page-tree-node-skipped`, `page-count-mismatch`, `flate-truncated`, `objstm-index-mismatch`, `object-number-out-of-range`, `warnings-truncated`), a `code() -> &'static str`, byte offset or object number where one exists; lazy warnings recorded on the `Document`, deduplicated by (code, object); `Document::warnings()` live.
- Error codes: a `code()` on `lamina_cos::Error` and `lamina_syntax::Error` too; all codes listed once in `fixtures/codes.json` with a Rust test that the enums expose exactly those.
- `OpenOptions` with private fields: `limits`, `strict` (and a reserved slot for the password, filled by [B2b](../ready/B2b-standard-security-decryption.md)); strict mode turns any warning into `Error::Strict { code }`.
- `ByteSource` trait (`len`, `read_at`, `as_slice`), `SourceError` (incl. `Pending { offset, len }`), core impls for `Vec<u8>`, `Arc<[u8]>`, `&'static [u8]`; `Document` owns `Arc<dyn ByteSource>`, no lifetime; `Document::open(source: impl Into<Arc<dyn ByteSource>>, OpenOptions)`; `Error::DataPending { ranges }`. **As shipped in #13:** a source without `as_slice` is read into memory once at open (`read_all`, 64 KiB `read_at` calls), so `DataPending` reports the whole file. Windowed reading (64 KiB windows, doubling within `Limits`), the chunked repair scan and per-window `DataPending` ranges moved out of scope to [B2d](../ready/B2d-windowed-reading.md).
- Caches move to `Arc<Object>`; a compile-time assertion that `Document: Send` (and a doc note that it is `!Sync`).
- nextest config replaces the wall-clock `BUDGET` in `crates/lamina-cos/tests/robustness.rs`. `.config/nextest.toml`: `retries = 0`; default `slow-timeout = { period = "60s", terminate-after = 2 }` (any test killed at 120 s); an override for `binary(robustness)` with `slow-timeout = { period = "30s", terminate-after = 1 }`, so the robustness tests keep exactly the old 30 s limit (killed at 30 s, not 60 s).
- The hang guard exists only under nextest. The plain `cargo test` fallback has no per-test timeout; CI always installs the pinned cargo-nextest and runs it, and `cargo xtask test` prints a note (`…; using cargo test instead`) when nextest is missing or off-pin, so a local fallback run is visibly unguarded.

**Out of scope** (and which todo covers it)

- Decryption and the password: [B2b](../ready/B2b-standard-security-decryption.md).
- Typed views, `LaminaId`s, corpus facts, fuzzing, `cargo xtask golden`: [B2c](../ready/B2c-lamina-model-corpus-fuzz.md).
- Overlay and mutation (`CosMut`): [B3a](../ready/B3a-overlay-and-ops.md).
- Windowed reading of non-slice sources, chunked repair scan, `DataPending` per window (moved out after #13 shipped read-all-at-open): [B2d](../ready/B2d-windowed-reading.md).
- Edge `ByteSource`s (mmap, `Blob`, HTTP range + OPFS): desktop D2 and web C3 todos.
- The TS half of the codes test: [B6b](../ready/B6b-lamina-pdf-package.md).

## Tasks

- [x] Load `/rust-best-practices`; re-read ADR 0003 §3 and ADR 0005 §2/§7
- [x] `crates/lamina-syntax/src/limits.rs`: `Limits`, `Limit` (enum naming each field, `#[non_exhaustive]`), defaults from the ADR table, getters, `with_*`, `unlimited()`; unit tests for defaults
- [x] Thread `Limits` through the lexer/parser (`max_nesting_depth`, `max_string_length`) and filters (`max_stream_decoded`, `max_filter_chain`); a per-call decoded-bytes counter for `max_decoded_total`
- [x] Replace `MAX_DEPTH`, `DEFAULT_MAX_DECODED`, `MAX_RESOLVE_DEPTH`, `MAX_SECTIONS` with `Limits` fields; keep `MAX_FILTER_PARAM_DEPTH` internal
- [x] `Error::LimitExceeded { limit, at }` in both crates; remove `TooDeep` and `DecompressLimit`; resolve-depth overrun reports `LimitExceeded`, true cycles keep `ReferenceCycle`
- [x] `max_page_tree_depth` in `Document::pages`; `max_xref_sections` and `max_object_number` in `xref.rs`; `max_warnings` in the warning sink
- [x] `Warning` variants and `code()`; warning sink on `Document` (interior mutability, dedup by (code, object)); raise each new code at its site (`xref.rs`, `objstm.rs`, `parser.rs` `/Length` recovery, `filters.rs` truncated Flate, `pages` skip and `/Count` mismatch, stale generation in `resolve`)
- [x] `OpenOptions` with private fields and `with_limits`, `with_strict`; `Error::Strict { code }`
- [x] `crates/lamina-cos/src/source.rs`: `ByteSource`, `SourceError`, impls; `Error::DataPending` (non-slice sources read whole at open)
- [ ] ~~Windowed reader used by the parser when `as_slice` is `None`~~: moved to [B2d](../ready/B2d-windowed-reading.md)
- [x] Make `Document` own its source (`Arc<dyn ByteSource>`), drop the lifetime, `Rc` → `Arc`; update `data()` (returns `Option<&[u8]>` or is removed) and every test helper to pass owned buffers
- [x] `fixtures/codes.json` (warnings and errors, kebab-case) and `crates/lamina-cos/tests/codes.rs`
- [x] `.config/nextest.toml` (default 60 s × 2; `binary(robustness)` override 30 s × 1 = the old 30 s budget); remove `BUDGET`/`Instant` from `robustness.rs`; check `cargo xtask test` uses the profile and notes the `cargo test` fallback (no per-test timeout); CI installs pinned cargo-nextest
- [x] Update crate docs (`lamina-cos/src/lib.rs` policy section) and READMEs

## Acceptance criteria

- [x] Every hard limit has a pair of tests (at the limit succeeds; one past fails with `LimitExceeded { limit: <that field> }`) and a test that its tolerant path degrades with the matching warning
- [x] Every soft limit has a test that the item is dropped and its warning raised
- [x] Every ADR 0003 §2 code with status B2 is raised by at least one test in `policy.rs` or `reader_tolerance.rs`; strict mode turns each into `Error::Strict { code }`
- [x] `fixtures/codes.json` matches the Rust enums exactly (test fails if a variant is added without a code or a code is reused)
- [ ] A document opened from a source without `as_slice` yields the same page count, objects and warnings as from a `Vec<u8>` for every file in `fixtures/corpus`: #13 tests this on `hello.pdf` only (`tests/source.rs`); the corpus-wide check moved to [B2d](../ready/B2d-windowed-reading.md)
- [x] A test source returning `Pending` produces `Error::DataPending` (#13: the whole-file range `(0, len)`)
- [ ] Per-window ranges and a retry that succeeds once the bytes arrive: moved to [B2d](../ready/B2d-windowed-reading.md)
- [x] `static_assertions`-style check (or a `fn assert_send<T: Send>()` test) proves `Document: Send`
- [x] No `Instant`/`Duration` assertions remain in PR-lane tests; under nextest (what CI runs) a hung robustness test is killed at 30 s and any other test at 120 s; the `cargo test` fallback is documented as unguarded
- [x] All existing goldens, determinism and incremental tests unchanged and green

## Tests

- `crates/lamina-syntax`: unit tests for `Limits` and parser/filters at/past each limit.
- `crates/lamina-cos/tests/policy.rs`, `reader_tolerance.rs`: one named test per new warning code and per limit (ADR 0003 §4).
- `crates/lamina-cos/tests/source.rs`: non-slice and pending sources (whole-file read; windowed sources over the corpus are [B2d](../ready/B2d-windowed-reading.md)'s).
- `crates/lamina-cos/tests/codes.rs`: codes vs `fixtures/codes.json`.
- Fixtures: crafted minimal PDFs built in tests (deep nesting, long filter chains, many `/Prev` hops, huge object numbers); no new binary fixtures unless a case can't be generated.

## Docs to update

- [ ] ADR 0003: mark the B2 codes as "exists" in §2's status column (not done in #13; [B2b](../ready/B2b-standard-security-decryption.md) does it while editing that table)
- [x] [pdf-library §10 B2](../../plans/pdf-library.md#10-to-dos): tick the `Document::open(source, OpenOptions)` to-do (windowed reading left as its own open to-do, now [B2d](../ready/B2d-windowed-reading.md))
- [x] `crates/lamina-cos/README.md`, `CLAUDE.md` (nextest config, `fixtures/codes.json`)
- [x] `fixtures/README.md` (codes.json)

## Risks and open questions

- Windowed parsing touches the hottest path (now [B2d](../ready/B2d-windowed-reading.md)'s risk); keep the `as_slice` fast path identical to today and benchmark open + walk on the corpus before/after (ratio only).
- Where to keep the warning sink without `Sync` (a `RefCell<Vec<Warning>>` is fine because `Document` is `!Sync`).
- `max_decoded_total` "per top-level call" needs a clear definition of the call boundary in `lamina-cos`; document it on the getter.
- The 30 s guard depends on nextest: a contributor running plain `cargo test` gets no timeout (documented in `CLAUDE.md` and in the config comment; `cargo xtask test` says so when it falls back). Requiring nextest locally was rejected to keep the fallback.
- Breaking API change for `open(&[u8])`; nothing is published, so acceptable (ADR 0003 consequences).

## Definition of done

Merged in [#13](https://github.com/laudebugs/lamina/pull/13) (squash). Claimed in PR #14, before the claim-PR rule existed.


- [x] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [x] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [x] Squash-merged; dependants' "Depends on" now satisfied
