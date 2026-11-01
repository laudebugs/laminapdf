# B3b — `laminapdf-save`: move the writer, `WriteSink`, incremental and full writers over the overlay, rebase

| Field | Value |
| --- | --- |
| ID | `B3b` |
| Status | `done` |
| Plan | [pdf-library §7 B3](../../plans/pdf-library.md#b3-ops-and-save), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B3a](./B3a-overlay-and-ops.md) |
| Blocks | [B3c](./B3c-save-planner-and-roundtrip.md), [B4a](./B4a-font-loading-and-subsetting.md) |
| ADRs | Implements [0007](../../decisions/0007-save-planner.md) §5–§7 and [0005](../../decisions/0005-document-model.md) §3 (rebase, states); follows [0001](../../decisions/0001-core-language-and-licence.md) (determinism), [0018](../../decisions/0018-test-strategy.md) tier 3 |
| Branch | `feat/b3b-lamina-save` (the claim named `feat/b3b-lamina-save-writers`) |
| PR title | `feat(core): laminapdf-save: WriteSink, incremental and full writers over the overlay (B3b)` |
| Size | L (≈ 1,000–1,400 lines incl. moved code; moved tests count once) |
| PR | [#35](https://github.com/laudebugs/lamina/pull/35) |

## Goal

Turn the overlay into bytes, safely and deterministically, and keep the document consistent afterwards. The spike-era writer (`Writer`, `incremental_update`, `XrefEol`) moves from `laminapdf-cos` into a new `laminapdf-save` crate with its golden, determinism and incremental tests unchanged; on top of it come the incremental and full-rewrite writers that read the overlay, the streaming `WriteSink`/`ReadableSink` traits, `/ID` handling and the rebase that makes the saved output the new base. The mode-resolution and signature/rewrite logic (`SavePlanner`, `SaveConstraint`) follows in [B3c](../done/B3c-save-planner-and-roundtrip.md).

## Context

- [ADR 0007 §5](../../decisions/0007-save-planner.md#5-writers): incremental (copy base in chunks, append new/modified/deleted with generation+1, same xref kind as newest, `/Prev`, never renumber or shrink `/Size`, drop xref-stream-only trailer keys, `IncrementalOutput::AppendedOnly`, empty overlay → base unchanged); full rewrite (reachable objects only, `Renumber::Keep` default, `Compact` opt-in with a map, object streams + xref stream by default, classic on request); `/ID` rules.
- [ADR 0007 §6](../../decisions/0007-save-planner.md#6-sinks-and-determinism): `WriteSink`, `ReadableSink::into_source`, position-counting wrapper, no seeking, `Vec<u8>` and `std::io::Write` adapter in core.
- [ADR 0007 §7](../../decisions/0007-save-planner.md#7-moving-the-writer): which tests move.
- [ADR 0005 §3](../../decisions/0005-document-model.md#3-base-overlay-and-the-save-cycle): rebase steps (1)–(5), `SavedNotRebased`.
- [Research ch. 07 §4](../../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned) (xref-stream trailer keys), [01 §14](../../research/pdf/01-file-structure.md#14-implementation-guidance-for-lamina-summary).
- Code (paths after the move): [`crates/laminapdf-save/src/legacy.rs`](../../../crates/laminapdf-save/src/legacy.rs), tests [`golden.rs`](../../../crates/laminapdf-save/tests/golden.rs), [`determinism.rs`](../../../crates/laminapdf-save/tests/determinism.rs), [`incremental.rs`](../../../crates/laminapdf-save/tests/incremental.rs), [`raw_writer.rs`](../../../crates/laminapdf-save/tests/raw_writer.rs), [`roundtrip.rs`](../../../crates/laminapdf-save/tests/roundtrip.rs).

## Scope

**In scope**

- New core crate `crates/laminapdf-save`; `git mv` of `writer.rs` and the four writer test files ADR 0007 §7 names (`golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs`, history kept); `laminapdf-cos` keeps reading and the overlay only.
- The `laminapdf-cos` tests that stay still build their inputs with the writer: `tests/common/mod.rs` imports `XrefEol` and `incremental_update` and is pulled into every integration test by `mod common;`, `common/samples.rs` uses `Writer`, and `roundtrip.rs`, `policy.rs`, `reader_cache.rs` and `review_fixes.rs` call `Writer`/`incremental_update`. They keep compiling through a `[dev-dependencies] laminapdf-save = { path = "../laminapdf-save" }` in `laminapdf-cos` (Cargo allows this dev-dependency cycle; the integration tests link the single `laminapdf-cos` that `laminapdf-save` uses) with imports switched to `laminapdf_save::…`. `roundtrip.rs` (write → read round trips) moves to `laminapdf-save` too; `review_fixes.rs` is split: writer cases move, reader cases stay.
- `WriteSink`, `ReadableSink`, `SinkError`; impls for `Vec<u8>` and `IoWrite<W: std::io::Write>` (behind `std`, still core-safe: no fs); `CountingSink`.
- `IncrementalWriter` over base `ByteSource` + overlay; `FullWriter` (reachability walk from trailer roots, keep numbers, `Compact` with an old → new map, object streams + xref stream or classic); `SaveOptions { mode, output: IncrementalOutput, renumber, xref: XrefKind, eol }` subset needed here; `SaveReport` (bytes written, mode, renumber map, append offset).
- `/ID`: keep `[0]`, `[1]` = hash of content written (SHA-256 truncated to 16 bytes, deterministic), `[0]` derived the same way for new documents or when missing.
- `Document::save_incremental`/`save_full` (temporary names until B3c's `Document::save(sink, options)`), rebase: incremental output builds the new base in memory (old base ‖ appended); full rewrite rebases from a `ReadableSink` or enters `SavedNotRebased` and `Document::rebase(source)`; ops and saves refused in `SavedNotRebased`; `LaminaId` map kept (renumber map applied on compact).
- Refusals carried over: repaired base (`RepairedBase`) for incremental; encrypted base (`EncryptedBase`) for **both** writers. B3b has no decryption ([B2b](./B2b-standard-security-decryption.md)) and no encryption on write (E11b), so a full rewrite of an encrypted base would either copy ciphertext without `/Encrypt` or keep `/Encrypt` over objects it can't re-encrypt; `FullWriter` refuses before writing anything ([ADR 0007 §2](../../decisions/0007-save-planner.md#2-saveconstraint-all-the-conditions-not-one-of-them): an encrypted base "can only be saved by removing encryption"). The explicit `remove_encryption` option and its planner rules are [B3c](../done/B3c-save-planner-and-roundtrip.md)'s.

**Out of scope** (and which todo covers it)

- `SavePlanner`, `SaveConstraint`, `SaveMode::Auto` resolution, signature detection: [B3c](../done/B3c-save-planner-and-roundtrip.md).
- Round-trip with pdf.js/PDFium/qpdf: [B3c](../done/B3c-save-planner-and-roundtrip.md).
- Signing phase, encryption on write: full-editor E10, E11.
- Edge sinks (OPFS spool, files): web/desktop rounds.

## Tasks

- [x] Load `/rust-best-practices`; read ADR 0007 §5–7 and ADR 0005 §3
- [x] Create `crates/laminapdf-save` (core); `git mv crates/laminapdf-cos/src/writer.rs crates/laminapdf-save/src/legacy.rs` (or `writer/`); `git mv` `golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs` and `roundtrip.rs`, and the writer cases of `review_fixes.rs`, into `crates/laminapdf-save/tests/` (with a copy of the `common/` helpers they use); keep goldens byte-identical
- [x] In `laminapdf-cos`: add the `laminapdf-save` dev-dependency, switch `tests/common/mod.rs`, `common/samples.rs`, `policy.rs`, `reader_cache.rs` and the remaining `review_fixes.rs` cases to `laminapdf_save::{Writer, XrefEol, incremental_update}`; `cargo test -p laminapdf-cos` and `cargo test -p laminapdf-save` both compile and pass
- [x] `sink.rs`: traits, `Vec<u8>`/`IoWrite` impls, `CountingSink`
- [x] `incremental.rs`: stream base bytes in chunks from `ByteSource::read_at`, append overlay entries in object-number order, free entries for deletes (gen + 1), xref section matching the newest kind, trailer (`/Size` = max(old, new), `/Prev`, `/Root`, `/Info`, `/ID`, drop `/Type /XRef`-only keys)
- [x] `full.rs`: reachability walk (iterative, bounded by `Limits`), `Renumber::{Keep, Compact}`, object streams packing (non-stream objects, deterministic grouping), xref stream writer, classic option
- [x] `id.rs`: `/ID` derivation
- [x] Rebase in `laminapdf-cos`/`laminapdf-save` boundary: new base source, reload xref, clear overlay and journal, keep IDs, state machine `Clean | Dirty | SavedNotRebased`
- [x] Tests listed below; `cargo xtask golden --update` covers any new goldens (e.g. `fixtures/golden/full-rewrite-hello.pdf`, `incremental-set-info.pdf`)
- [x] Update xtask crate lists; `CLAUDE.md` layout comment for `laminapdf-cos` ("reader and overlay") and `laminapdf-save`

## Acceptance criteria

- [x] Moved tests (`golden`, `determinism`, `incremental`, `raw_writer`, `roundtrip`, writer cases of `review_fixes`) pass unchanged in `laminapdf-save` with byte-identical goldens
- [x] Every `laminapdf-cos` integration test that stays (`corpus`, `robustness`, `policy`, `reader_cache`, `reader_tolerance`, `limits`, `warnings`, `codes`, `source`, `content`, reader cases of `review_fixes`) compiles and passes, with `laminapdf-cos` having no normal dependency on `laminapdf-save`
- [x] Empty overlay + incremental save writes exactly the base bytes
- [x] Open → `SetInfo` → incremental save → reopen shows the new Info, `revisions()` + 1, base bytes are a prefix of the output, `/ID[0]` unchanged
- [x] Open → `RotatePage` → full save (`Keep`) → reopen: same object numbers, same `PageId`s, unreachable objects gone
- [x] `Compact` returns a complete renumber map and IDs follow it
- [x] Every writer test saves twice and compares bytes (determinism)
- [x] After a successful save the document is `Clean`; after a failed save (sink error injected) it stays `Dirty` with the overlay untouched
- [x] A full rewrite to a non-readable sink leaves `SavedNotRebased`; ops and saves fail until `rebase`
- [x] Incremental **and** full saves of an encrypted base (`fixtures/corpus/enc.pdf`) fail with `EncryptedBase`, write nothing to the sink and leave the document `Dirty` with the overlay untouched

## Post-merge checks

None.

## Tests

- `crates/laminapdf-save/tests/`: moved `golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs`, `roundtrip.rs` and the writer half of `review_fixes.rs`; new `overlay_incremental.rs`, `full_rewrite.rs`, `rebase.rs`, `id.rs`, `sink_errors.rs`, `encrypted_base.rs` (both writers refuse `enc.pdf`).
- Property test: random op sequences → save (both modes) → reopen → model facts equal the in-memory view.
- Goldens: new byte-exact outputs in `fixtures/golden/` (generated by `cargo xtask golden --update`).

## Docs to update

- [x] [pdf-library §10 B3](../../plans/pdf-library.md#10-to-dos): tick "Create `crates/laminapdf-save` … writers, `WriteSink`"
- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "Move the writer …" and partly `laminapdf-save`
- [x] `CLAUDE.md` (layout, `cargo test -p laminapdf-save`), crate READMEs, `fixtures/README.md`

## Risks and open questions

- Object-stream grouping must be deterministic and must not put the encryption dictionary, xref streams or stream objects inside object streams (§7.5.7).
- Streaming the base for incremental output with a `Pending` source: propagate `DataPending` before writing anything (check all ranges first).
- The `laminapdf-cos` → `laminapdf-save` dev-dependency is a cycle Cargo accepts for integration tests, but `laminapdf-cos` unit tests (`src/**` `#[cfg(test)]`) must not use `laminapdf_save`, or they see two copies of `laminapdf-cos` types. If that bites, `common/samples.rs` can embed its sample PDFs as committed bytes instead; record the choice in the PR.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings): run by the orchestrating session after the PR opened
- [ ] CI green (see the PR)
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome and deviations

- **Crate and moves.** `crates/laminapdf-save` (core) holds the spike-era writer as `src/legacy.rs` (`git mv` of `writer.rs`) and the moved tests `golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs`, `roundtrip.rs` (bodies unchanged, imports switched), the writer half of `review_fixes.rs`, `tests/common/samples.rs` and `examples/golden.rs` (`cargo xtask golden` now runs `-p laminapdf-save`). `tests/common/mod.rs` and `content.rs` are copies. Goldens `hello.pdf` and `imaging-demo.pdf` are byte-identical; new goldens `incremental-set-info.pdf` and `full-rewrite-hello.pdf` (both pass `qpdf --check`).
- **Dev-dependency cycle.** `laminapdf-cos`, `laminapdf-model` and `laminapdf-ops` depend on `laminapdf-save` as a dev-dependency (their integration tests, doc tests and, for model/ops, unit tests used `Writer`). `laminapdf-cos`'s unit tests (`src/overlay.rs`) build their file by hand, and its crate doc example opens hand-written bytes. Writer cases that live in other `laminapdf-cos` test files (`encryption.rs`, `overlay.rs`, `source.rs`) stayed and switched imports.
- **`laminapdf-cos` API used by the writers:** `XREF_STREAM_KEYS`, `size_floor`, `next_number`, `base_generation`, `newest_section_is_stream`, `whole_input` are public now; `Document::rebase(source)` is new (reopens with the same limits/strict mode, drops the overlay, carries the allocator).
- **`Save` trait** instead of inherent `Document::save_incremental`/`save_full`: `laminapdf-save` can't add inherent methods to `laminapdf_ops::Document`. The full rewrite has two entry points, `save_full` (write-only sink → `SavedNotRebased`) and `save_full_and_rebase` (a `ReadableSink`). The state lives in `laminapdf-ops` (`SaveState`, `saved_not_rebased`, `rebase`; new codes `saved-not-rebased`, `unsaved-changes`); `laminapdf-save` adds `SaveError` (new code `sink` under `saveErrors` in `fixtures/codes.json`; `repaired-base`/`encrypted-base` reuse the COS codes).
- **`SaveOptions` has no `mode`**: the method picks the writer; `SaveMode` appears in the report. B3c adds `mode` with `Auto`.
- **`IoWrite` is not behind a `std` feature**: no workspace crate builds without `std`, and `std::io::Write` passes `cargo xtask portability`.
- **Determinism**: each writer has save-twice tests (`overlay_incremental`, `full_rewrite`, `id`, and every case of the property test saves twice and compares), rather than a second save in literally every test.
- **Incremental windowed bases** are checked chunk by chunk before writing (two reads of the base); the rebased source chains the old source and the appended bytes without copying. In-memory bases are rebuilt as one buffer.
- ADR 0007 gained §8 "Implementation notes (B3b)".
- **After the `/code-review high` round (40 findings, fixed in one commit):** undo history now survives only an incremental save that freed nothing (`PendingRebase`; a full rewrite in either numbering clears it), `rebase` needs a pending save, `laminapdf_cos::Document::rebase` takes `Reserved::{Keep, Release}` and keeps warnings and journal sequence numbers; a failed rebase after a complete write returns `SaveError::NotRebased` with the output; `SaveError` has no own `RepairedBase`/`EncryptedBase` (they are the COS errors; new code `not-rebased`, and `unsaved-changes` became `no-pending-save` before anything shipped); sinks report what they hold (`WriteSink::held`, `SinkError::NotEmpty`); a save with nothing to append is a no-op even for repaired/encrypted bases; the full writer's xref is sparse (bounded by the data, not the input `/Size`), frees unwritten base objects with the next generation, fails on unloadable objects, keeps nested `/Length` references and indirect `/ID` elements, and counts nesting like the parser; incremental sections link into the base's free list (`free_list_head`), drop xref-stream keys (`/DL`, `/F`, `/FFilter`, `/FDecodeParms` joined `XREF_STREAM_KEYS`), hash the base for `/ID[1]`, and check the xref stream number against `Limits`. The test helpers are shared with `laminapdf-cos` by `#[path]` (no copied `content.rs`), and `spikes/krilla-fit` builds again (it was already broken against main's API; now on `laminapdf-save`).
- **Follow-up (efficiency, not done here):** a rebase reopens the new base from scratch (xref, caches); reusing the parsed sections and cached objects of an incremental save's untouched base is left for later.
