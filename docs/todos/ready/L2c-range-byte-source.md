# L2c — Range `ByteSource`: `DataPending` ranges at the edges (HTTP Range, JS callback, native file and mmap), core stays synchronous

| Field | Value |
| --- | --- |
| ID | `L2c` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L2](../../plans/shareable-libraries.md#l2-typescript-hardening) |
| Round / milestone | Round L2 (shareable libraries) |
| Depends on | [B2d](../done/B2d-windowed-reading.md), [B6b](./B6b-lamina-pdf-package.md) |
| Blocks | None |
| ADRs | Implements the edge half of [0005](../../decisions/0005-document-model.md) §2 (`SourceError::Pending` → `Error::DataPending { ranges }`, edge `ByteSource`s); follows [0001](../../decisions/0001-core-language-and-licence.md) (bytes-in/bytes-out core), [0003](../../decisions/0003-reader-tolerance-and-limits.md) (`Limits` bound what is fetched), 0026 and 0028 (codes, cancellation; from [L1a](../done/L1a-contract-adrs.md)), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (new crate dependencies) |
| Branch | `feat/l2c-range-byte-source` |
| PR title | `feat(core): range byte sources with DataPending at the edges (HTTP range, JS callback, mmap)` |
| Size | L (≈ 500–800 lines of Rust, TS and tests) |
| PR | — |

## Goal

Open large and remote PDFs without downloading or loading them whole, while the core stays synchronous. [B2d](../done/B2d-windowed-reading.md) makes the reader ask only for the windows it needs, through `Error::DataPending { ranges }` (the report calls this `NeedData`; lamina already has the name `DataPending`, so no second one is added). This todo builds the other half: sources that start empty and are filled range by range, and edge adapters that run the fetch-and-retry loop.

- **TypeScript:** HTTP `Range` requests via `fetch`, and a caller-supplied async read callback.
- **Rust:** a file source with positioned reads and an opt-in mmap source in an edge crate.

([Report: bytes cross once, ranges are pulled](../../research/libraries/shareable-libraries.md#bytes-cross-once-ranges-are-pulled-and-the-core-stays-synchronous).)

## Context

- [ADR 0005 §2](../../decisions/0005-document-model.md#2-bytesource-and-ownership-of-the-input): `ByteSource` (`Send + Sync`, `read_at`, `as_slice`), the `Pending` → `DataPending` retry pattern, "edge implementations: mmap, `FileReaderSync`, HTTP-range cache … none in core crates".
- [B2d](../done/B2d-windowed-reading.md): windows, chunked repair, `DataPending` carrying only the needed ranges, and its risk that lazy calls after open can also return `DataPending`.
- [Report: bytes cross once](../../research/libraries/shareable-libraries.md#bytes-cross-once-ranges-are-pulled-and-the-core-stays-synchronous) and [rust-core-bridge note §4](../../research/libraries/notes/rust-core-bridge.md#4-zero-copy-streaming-partial-loading): pdf.js range/stream/auto-fetch options, PDFium's `FPDF_AVAIL` pull model **[unverified]**, the tail-first read for non-linearized files, and incremental save returning only the appended part.
- [C3a](./C3a-viewer-worker-rpc.md) (a TS `ByteSource` interface `read(offset, length, signal)`, `size`) and [C4c](./C4c-remote-files-encryption-privacy.md) (`HttpRangeByteSource` in the viewer with block cache, `ETag` check → `source-changed`, privacy fetch options). Align with both, so the viewer can reuse this source instead of writing its own.
- [D2b](./D2b-engine-process.md): the desktop main process uses positioned reads, **not** `memmap2`, because a mapped file truncated underneath raises `SIGBUS`.
- [Code organisation §2](../../guidelines/code-organization.md#2-boundaries-and-dependency-direction) and [Rust guideline: unsafe](../../guidelines/rust.md#unsafe): fs and `unsafe` only in edge crates, with `// SAFETY:` comments.

## Scope

**In scope**

- **Edge (`crates/lamina-source`, layer `edge`):** a `SparseSource` that implements `ByteSource` over chunks the shell supplies. It lives at the edge, not in `lamina-cos`, because `supply` runs after `Document::open` has shared the source, so the chunk map needs a lock, and ADR 0005 §7 forbids locks and caches in core (§2 also keeps range caches out of core). The pure parts (`SparseSource`, the retry helper) compile for wasm32; file and mmap sources sit behind a `native` feature.
  - API: `new(len)`, `supply(offset, bytes)`, `missing(ranges)`.
  - `read_at` returns `SourceError::Pending` for any gap.
  - Chunks are stored as `Arc<[u8]>`, merged when adjacent, and bounded by a byte budget.
  - No I/O, clocks or threads; `Send + Sync`.
- **Retry helper (same edge crate):** `open_with(source, opts, fetch: impl FnMut(&[Range]) -> Result<…>)`, a synchronous loop that calls the shell's fetch closure on `DataPending` and retries. Native shells and the wasm edge then don't each re-implement the loop.
- **wasm edge (`lamina-wasm`):** expose `SparseSource` (`supply` takes a `Uint8Array`), and surface `DataPending` as a `LaminaError` with `ranges: { offset, length }[]`.
- **TypeScript (`packages/pdf`):**
  - A `RangeSource` interface `{ size, read(offset, length, { signal }) => Promise<Uint8Array> }`, matching C3a's.
  - Adapters: `httpRangeSource(url, { headers?, credentials?, signal?, blockSize? })` and `callbackSource(size, read)`.
  - The HTTP adapter:
    - probes `Range: bytes=0-0` for size and range support;
    - merges adjacent ranges and aligns them to blocks;
    - checks `206`, `Content-Range`, `ETag` and `If-Range` (a change becomes `source-changed`);
    - falls back to one full GET when the server answers `200`;
    - fetches the tail first.
  - `await Document.openRange(source, opts)` runs the fetch-and-retry loop.
  - `await doc.ensure(fn)` runs a sync call and, on `data-pending`, fetches the ranges and retries, so lazy calls after open work.
  - The worker client ([L2a](./L2a-worker-client.md)), if merged, runs the same loop inside the worker, so remote handles stay plain async.
- **Native edge crate** `crates/lamina-source` (layer `edge`):
  - `FileSource`: positioned reads (`FileExt::read_at` / `seek_read`), no `unsafe`. A short read after truncation becomes `SourceError`, not a crash.
  - `MmapSource` behind an `mmap` feature (`memmap2`, licence-checked) with a `// SAFETY:` note and the documented `SIGBUS` risk. `as_slice` returns the mapping.
- Bytes fetched are bounded: no adapter fetches more than the ranges asked for, plus block alignment and a configurable readahead.

**Out of scope** (and which todo covers it)

- The windowed reader and chunked repair themselves: [B2d](../done/B2d-windowed-reading.md).
- Viewer block cache on OPFS, privacy fetch options and the remote-URL UI: [C4c](./C4c-remote-files-encryption-privacy.md) (it should wrap `httpRangeSource`).
- The desktop engine's IPC-backed source: [D2b](./D2b-engine-process.md).
- Linearization fast paths and prefetch hints: later, if the web perf harness ([C3c](./C3c-viewer-perf-harness.md)) asks for it.
- Returning only the appended bytes of an incremental save: a follow-up on the save API ([B3c](../done/B3c-save-planner-and-roundtrip.md) area).
- Streaming input (`ReadableStream`) with no `size`: later; it needs a growing source and tail-last handling.

## Tasks

- [ ] Load the `/rust-best-practices` skill and follow [docs/guidelines/rust.md](../../guidelines/rust.md). Re-read ADR 0005 §2 and B2d's `source.rs` on `main`. `lamina-cos` stays core: no fs, net, clocks or threads (`cargo xtask portability`)
- [ ] `crates/lamina-source/src/sparse.rs`:
  - [ ] `SparseSource` (interval map of supplied chunks, merge, byte budget)
  - [ ] Unit tests for gaps, overlaps, out-of-range supply and the budget
- [ ] `open_with` retry helper:
  - [ ] Stop when a retry asks for the same ranges again (no progress)
  - [ ] Cap the rounds via `Limits` or an option, failing with a coded error (`data-pending` with the last ranges)
- [ ] `lamina-wasm`: `SparseSource` class; `DataPending` → `LaminaError { code: "data-pending", ranges }`
- [ ] `packages/pdf/src/source/`: `RangeSource`, `httpRangeSource`, `callbackSource`, `Document.openRange`, `doc.ensure`. `AbortSignal` passes through to `fetch`. Never put document bytes or the URL in error messages (ADR 0026)
- [ ] Add codes `source-changed` and `range-unsupported` (if a warning is wanted for the 200 fallback) to `fixtures/codes.json` and regenerate
- [ ] `crates/lamina-source` (edge):
  - [ ] `Cargo.toml` with `[package.metadata.lamina] layer = "edge"`
  - [ ] `FileSource`; `MmapSource` behind `mmap`
  - [ ] `cargo deny` passes
- [ ] Wire `httpRangeSource` into [L2a](./L2a-worker-client.md)'s worker if merged; otherwise leave a TODO in the worker README
- [ ] README sections: remote files (CORS: `Range` triggers a preflight, `Content-Range`/`ETag` must be exposed; `Content-Encoding` breaks ranges), large local files (`FileSource` vs `MmapSource`)

## Acceptance criteria

- [ ] For every `fixtures/corpus` file, opening through `SparseSource` + `open_with` (fetch closure reads from the file bytes) gives the same page count, objects, warnings and decoded streams as a `Vec<u8>` open
- [ ] Total bytes supplied is less than the file size for every corpus file over 256 KiB with an intact xref (or a generated 1 MiB multi-page file), matching B2d's criterion
- [ ] TS: against a local test HTTP server that logs requests, `Document.openRange(httpRangeSource(url))` gives the same results as `Document.open(bytes)`, and the logged ranges cover less than the file for the same files. Reading page *n* fetches only that page's objects (asserted on the log)
- [ ] TS: a server that ignores `Range` (answers `200`) still opens the file with one full download; a server whose `ETag` changes mid-read fails with `source-changed`; aborting the signal rejects and leaves no request running
- [ ] TS: a lazy call (`doc.page(n)`) on a range document that needs more data works through `doc.ensure`, and throws `data-pending` with ranges when called bare
- [ ] Rust: `FileSource` over each corpus file equals a `Vec<u8>` open; truncating the file mid-read gives a `SourceError`, not a panic. `MmapSource` passes the same parity test
- [ ] `cargo xtask portability` still passes; `lamina-source` is the only crate with fs or `unsafe` code added

## Post-merge checks

None.

## Tests

- `crates/lamina-source/tests/sparse_source.rs`: corpus parity, bytes-supplied accounting, no-progress and round-cap failures.
- `crates/lamina-source/tests/`: `FileSource` and `MmapSource` parity; truncation (create a temp copy in the test, truncate it, read).
- `packages/pdf/test/source/*.test.ts`: a tiny local HTTP server in the test harness (range support on/off, wrong `Content-Range`, changing `ETag`, slow responses for abort); `callbackSource`; `doc.ensure`. Fixtures: `fixtures/corpus`, plus a large file generated in the test.

## Docs to update

- [ ] [shareable-libraries §9 L2](../../plans/shareable-libraries.md#9-to-dos): tick the range `ByteSource` to-do
- [ ] `crates/lamina-source/README.md` (sparse sources, `open_with`, native sources), `packages/pdf/README.md` (remote and large files)
- [ ] `CLAUDE.md` (layout: `crates/lamina-source`; reading API paragraph mentions range sources)
- [ ] ADR 0005 §2: implementation note naming `SparseSource` and the edge crate
- [ ] [C4c](./C4c-remote-files-encryption-privacy.md): a Context line pointing at `httpRangeSource`, so the viewer wraps it instead of duplicating it

## Risks and open questions

- **Sync API and lazy `DataPending`:** every lazy call on a range document can need more data. `doc.ensure` makes that explicit but clumsy. Recommendation: point range users to the worker client, where the loop is hidden behind Promises; keep `ensure` for in-thread use. Decide in the PR with the reviewer.
- **mmap at all?** [D2b](./D2b-engine-process.md) avoids it in the desktop main process because of `SIGBUS`. Recommendation: ship `FileSource` as the default and `MmapSource` only behind a feature, documented as unsafe if the file changes. Drop it if no consumer wants it.
- **Round trips:** non-linearized files need the tail, then the xref, then objects, so latency adds up. Recommendation: let B2d's `DataPending` return several ranges at once, add readahead in the adapter, and measure rounds per corpus file (ratio only, no wall clock, per ADR 0018).
- **CORS and compression** on third-party hosts are outside lamina's control. Document them, and fail with clear codes.
- **Crate name** `lamina-source` vs `lamina-io`: decide in the PR. Check [code organisation §11](../../guidelines/code-organization.md#11-when-to-split-a-crate-or-package-and-when-not-to) before adding a crate.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
