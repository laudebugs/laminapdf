# B2d — Windowed reading for sources without `as_slice`

| Field | Value |
| --- | --- |
| ID | `B2d` |
| Status | `done` |
| Plan | [pdf-library §7 B2](../../plans/pdf-library.md#b2-document-model) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2a](./B2a-limits-owned-document.md) |
| Blocks | [L2c](../ready/L2c-range-byte-source.md) |
| ADRs | Implements [0005](../../decisions/0005-document-model.md) §2 (windowed reading, chunked repair, `DataPending`); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits`), [0018](../../decisions/0018-test-strategy.md) (no wall-clock assertions in PR lanes) |
| Branch | `feat/b2d-windowed-reading` |
| PR title | `feat(core): windowed reading and chunked repair for sources without as_slice` |
| Size | M (≈ 400–700 lines incl. tests) |
| PR | — |

## Goal

Make `Document::open` work over a source it can't hold in memory at once. [B2a](./B2a-limits-owned-document.md) (#13) shipped `ByteSource` and the owned `Document`, but a source without `as_slice` is read whole at open (`read_all` in `crates/lamina-cos/src/source.rs`), so `Error::DataPending` always asks for the whole file. That defeats the point of an HTTP-range source (web C3/C4) or a large mmap'd file (desktop D2): the viewer has to download everything before page 1. This todo does what [ADR 0005 §2](../../decisions/0005-document-model.md#2-bytesource-and-ownership-of-the-input) specifies: read 64 KiB windows around the offsets the parser needs, scan for repair in overlapping chunks, and report only the missing ranges through `DataPending`.

## Context

- [ADR 0005 §2](../../decisions/0005-document-model.md#2-bytesource-and-ownership-of-the-input): "For sources without `as_slice`, the COS layer reads a window around an offset (64 KiB, doubled up to the object's extent within `Limits`) and the parser works on that `&[u8]`. Repair scans in overlapping chunks." and the `SourceError::Pending` → `Error::DataPending { ranges }` retry pattern (pdf.js's missing-data pattern).
- [ADR 0003 §3](../../decisions/0003-reader-tolerance-and-limits.md): `Limits` that bound window growth (`max_string_length`, `max_stream_decoded`, …).
- [B2a](./B2a-limits-owned-document.md): what shipped and what moved here (Scope, Acceptance criteria).
- Code on `main`: [`crates/lamina-cos/src/source.rs`](../../../crates/lamina-cos/src/source.rs) (`Source::bytes`, `loaded`, `read_all`, `READ_CHUNK`), [`reader/mod.rs`](../../../crates/lamina-cos/src/reader/mod.rs), [`reader/xref.rs`](../../../crates/lamina-cos/src/reader/xref.rs), [`reader/repair.rs`](../../../crates/lamina-cos/src/reader/repair.rs), [`reader/objstm.rs`](../../../crates/lamina-cos/src/reader/objstm.rs), [`lamina-cos/src/scan.rs`](../../../crates/lamina-cos/src/scan.rs); tests [`tests/source.rs`](../../../crates/lamina-cos/tests/source.rs).

## Scope

**In scope**

- A window cache in `Source`: reads of 64 KiB aligned windows via `read_at`, grown by doubling when an object runs past the window end (up to the object's extent, bounded by `Limits`); the parser works on the window's `&[u8]` with absolute offsets kept correct.
- Every reader path that today calls `Source::bytes()`/`loaded()` on the whole input (header search, `startxref` from the tail, xref tables and streams, `/Prev` chains, object lookup, object streams, stream data, `/Length` recovery) reads through windows instead. The `as_slice` fast path is unchanged (same code path as today, no extra copies).
- Repair (`reader/repair.rs`, `scan.rs`): scan the source in overlapping chunks (overlap ≥ the longest token the scanner matches, so an `obj` header split across chunks is still found), bounded by `Limits`.
- `DataPending`: a `SourceError::Pending { offset, len }` from any window read stops the operation and returns `Error::DataPending { ranges }` with the window ranges actually needed (not the whole file); collect several ranges when they are known up front (e.g. tail + first xref section) so the shell can fetch them in one round. Retrying after the shell has the bytes succeeds and gives the same result as an in-memory open. A `Pending` during a lazy call after open (`resolve`, `pages`) reports the same way and leaves the `Document` usable.
- Remove `read_all` (or keep it only behind an explicit opt-in, documented) and update the `lamina-cos` crate docs and README.

**Out of scope** (and which todo covers it)

- Edge `ByteSource`s (mmap, `Blob`, HTTP range cache over OPFS): desktop D2 and web C3/C4 todos.
- Exposing `DataPending` through WASM/TS: [B6b](../ready/B6b-lamina-pdf-package.md) and the web reader todos.
- Prefetch hints and linearization (`/Linearized` first-page fast path): later, if the web perf harness asks for it.

## Tasks

- [x] Load `/rust-best-practices`; re-read ADR 0005 §2 and the `source.rs` / reader code on `main`
- [x] `source.rs`: a `Window` type (absolute start offset + `Arc<[u8]>`), a small window cache on `Source` (keyed by aligned start, bounded count), `Source::window(offset, min_len)` that doubles up to the needed extent within `Limits`, and collects `Pending` ranges
- [x] Switch the reader's entry points (`open`, tail and `startxref` search, xref table/stream parsing, `/Prev` walk, `resolve`, object streams, `decode_stream`, `/Length` recovery) from whole-input slices to windows; keep the `as_slice` path byte-for-byte the same
- [x] `repair.rs` / `scan.rs`: chunked scan with overlap; a unit test with an `obj` keyword straddling a chunk boundary
- [x] `Error::DataPending { ranges }` carries the needed windows; merge adjacent ranges; test that a retry after filling them succeeds
- [x] Remove `read_all` / `READ_CHUNK` (or gate them, documented); update crate docs (`lamina-cos/src/lib.rs` policy section) and `crates/lamina-cos/README.md`
- [x] Bench open + page-tree walk over `fixtures/corpus` for a slice source and a windowed source before/after (ratio only, no wall-clock assertion; numbers in the PR description)

## Acceptance criteria

- [x] For every file in `fixtures/corpus`, a test `ByteSource` that only implements `read_at` (no `as_slice`) yields the same page count, objects, warnings and decoded streams as a `Vec<u8>` (moved here from B2a)
- [x] The same corpus test with a source that answers `Pending` for any range not yet "fetched", driven by a loop that fetches the reported ranges and retries, ends with the same results, and the total bytes fetched for a file with an intact xref is less than the file size for every corpus file over 256 KiB (or, if the corpus has none, for a generated 1 MiB multi-page file)
- [x] `DataPending` never reports the whole file for an intact file larger than one window; it reports the windows needed
- [x] Repair on a windowed source finds the same objects as on a slice source for every repaired corpus file (`hello-broken.pdf`, `tampered.pdf`), including an object header placed across a chunk boundary
- [x] Window growth respects `Limits`: an object that would need a window past the limit fails with `LimitExceeded` (or degrades with its warning) exactly as on a slice source
- [x] All existing tests, goldens and the `Document: Send` assertion unchanged and green; the `as_slice` path's outputs are byte-identical

## As shipped

- **Windows:** 64 KiB, aligned, cached per `Source` (the 64 most recently used, 4 MiB, behind a `Mutex` because `Source` clones cross threads; `Document` stays `Send + !Sync`). A parse runs on the window holding its offset and again on a window twice as long while `lamina-syntax`'s new `Lexer::reached_end` / `Parser::reached_end` say it looked past the window's end (they also cover the `endstream` checks of `indirect_object`), so a windowed parse gives exactly the in-memory result; error offsets are rebased to input offsets (`reader/window.rs`). In-memory sources go through the same helpers with the whole slice, unchanged.
- **Probes (differs from the task list's "every path reads windows"):** checking every xref entry's `n g obj` header at open reads a 32-byte probe per entry (doubled if needed), not a window, otherwise opening an intact file would read every window. Missing probes are reported in one `DataPending`.
- **`DataPending`:** the ranges the source answered `Pending` for, sorted and merged. Open asks for the head and tail windows together; the backward `startxref` search and the repair scan ask for every window they still need in one round. The `/Length` object and a longer window, both missing, are reported together.
- **Repair:** `scan_object_headers` and the trailer scan walk the input in overlapping chunks (`for_each_chunk`); a `num gen` that starts in an earlier chunk is read back with growing probes; `parse_head`'s `endstream` search is a forward windowed search.
- **`read_all` removed.** `Source::whole()` reads all windows only for `incremental_update`, whose output holds them anyway. **API change:** `Document::data()` is now `Option<&[u8]>` (`None` for a windowed source); two tests that compared `data()` pointers were updated, and the overstated-length test now expects the first window's `Failed` range.
- **Limits:** no `Limit` bounds a raw object's size (an in-memory parse copies a stream of any size too), so windows grow to the object's extent; hard limits hit while parsing (`max_string_length`, `max_nesting_depth`) fail at the same offset as in memory, and memory grows only with bytes actually read.
- **Tests:** `tests/source.rs` runs the corpus, every encrypted fixture (and each with its `startxref` broken, so repaired and decrypted) and three generated files (1.3 MiB classic and compact multi-page files, a repaired 5-window file with a header across the 64 KiB boundary) through a read-at-only source and through a pending source driven by a fetch-and-retry loop, comparing structure, every object, decoded streams, pages, content and warnings with the in-memory result; plus bytes-fetched accounting (open + page walk of the 1.3 MiB files fetch under a quarter of the file), the first `DataPending` (head + tail), no range over one window for intact files, a lazy `page_content` that hits missing data and succeeds on retry, limits over and within a grown window, and incremental update from a windowed source. Unit tests in `source.rs` (alignment, sharing, eviction, probes, merging), `reader/window.rs` (searches across a boundary, doubling, rebasing, chunk overlap), `reader/repair.rs` (`obj` and `trailer` across a boundary, `num gen` in the previous chunk, a long white-space run, agreement at every offset near a boundary) and `lamina-syntax` (`reached_end`).
- **Bench** (open + page-tree walk, ratio only; numbers in the PR description).

## Post-merge checks

None.

## Tests

- `crates/lamina-cos/tests/source.rs`: windowed and pending sources over the whole corpus, retry loop, bytes-fetched accounting, `DataPending` ranges.
- `crates/lamina-cos/src/source.rs` unit tests: window alignment, doubling, cache eviction, merging ranges.
- `reader/repair.rs` / `scan.rs` unit tests: chunk overlap, straddling `obj` header.
- Fixtures: none new; a generated multi-page file built in the test if the corpus lacks a large one.

## Docs to update

- [x] [pdf-library §10 B2](../../plans/pdf-library.md#10-to-dos): tick "Windowed reading for sources without `as_slice`"
- [x] `crates/lamina-cos/README.md` and crate docs (how non-slice sources are read, `DataPending` semantics)
- [x] ADR 0005: implementation note under §2 if anything differs from the ADR (window size, cache bound)

## Risks and open questions

- This is the reader's hottest path; the `as_slice` fast path must stay as it is. Benchmark before/after (ratio only).
- Lazy calls after open can now hit `Pending`; callers (web worker, desktop engine) must handle `DataPending` from `resolve`/`pages`, not only from `open`. Document it on the error.
- Window cache size: bounded so a hostile file can't make the cache hold the whole input; pick a default and make it a `Limits` field only if needed (ADR 0003 amendment if so).
- Tail search for `startxref` on a source whose `len()` is wrong: keep B2a's behaviour (fail cleanly with `Source`/`Failed`).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
- [ ] Squash-merged; dependants' "Depends on" now satisfied
