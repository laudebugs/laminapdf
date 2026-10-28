# B3b — `lamina-save`: move the writer, `WriteSink`, incremental and full writers over the overlay, rebase

| Field | Value |
| --- | --- |
| ID | `B3b` |
| Status | `ready` |
| Plan | [pdf-library §7 B3](../../plans/pdf-library.md#b3-ops-and-save), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B3a](./B3a-overlay-and-ops.md) |
| Blocks | [B3c](./B3c-save-planner-and-roundtrip.md), [B4a](./B4a-font-loading-and-subsetting.md) |
| ADRs | Implements [0007](../../decisions/0007-save-planner.md) §5–§7 and [0005](../../decisions/0005-document-model.md) §3 (rebase, states); follows [0001](../../decisions/0001-core-language-and-licence.md) (determinism), [0018](../../decisions/0018-test-strategy.md) tier 3 |
| Branch | `feat/b3b-lamina-save-writers` |
| PR title | `feat(save): lamina-save with streaming WriteSink, incremental and full writers, and rebase` |
| Size | L (≈ 1,000–1,400 lines incl. moved code; moved tests count once) |
| PR | — |

## Goal

Turn the overlay into bytes, safely and deterministically, and keep the document consistent afterwards. The spike-era writer (`Writer`, `incremental_update`, `XrefEol`) moves from `lamina-cos` into a new `lamina-save` crate with its golden, determinism and incremental tests unchanged; on top of it come the incremental and full-rewrite writers that read the overlay, the streaming `WriteSink`/`ReadableSink` traits, `/ID` handling and the rebase that makes the saved output the new base. The mode-resolution and signature/rewrite logic (`SavePlanner`, `SaveConstraint`) follows in [B3c](./B3c-save-planner-and-roundtrip.md).

## Context

- [ADR 0007 §5](../../decisions/0007-save-planner.md#5-writers): incremental (copy base in chunks, append new/modified/deleted with generation+1, same xref kind as newest, `/Prev`, never renumber or shrink `/Size`, drop xref-stream-only trailer keys, `IncrementalOutput::AppendedOnly`, empty overlay → base unchanged); full rewrite (reachable objects only, `Renumber::Keep` default, `Compact` opt-in with a map, object streams + xref stream by default, classic on request); `/ID` rules.
- [ADR 0007 §6](../../decisions/0007-save-planner.md#6-sinks-and-determinism): `WriteSink`, `ReadableSink::into_source`, position-counting wrapper, no seeking, `Vec<u8>` and `std::io::Write` adapter in core.
- [ADR 0007 §7](../../decisions/0007-save-planner.md#7-moving-the-writer): which tests move.
- [ADR 0005 §3](../../decisions/0005-document-model.md#3-base-overlay-and-the-save-cycle): rebase steps (1)–(5), `SavedNotRebased`.
- [Research ch. 07 §4](../../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned) (xref-stream trailer keys), [01 §14](../../research/pdf/01-file-structure.md#14-implementation-guidance-for-lamina-summary).
- Code: [`crates/lamina-cos/src/writer.rs`](../../../crates/lamina-cos/src/writer.rs), tests [`golden.rs`](../../../crates/lamina-cos/tests/golden.rs), [`determinism.rs`](../../../crates/lamina-cos/tests/determinism.rs), [`incremental.rs`](../../../crates/lamina-cos/tests/incremental.rs), [`raw_writer.rs`](../../../crates/lamina-cos/tests/raw_writer.rs), [`roundtrip.rs`](../../../crates/lamina-cos/tests/roundtrip.rs).

## Scope

**In scope**

- New core crate `crates/lamina-save`; `git mv` of `writer.rs` and the four writer test files ADR 0007 §7 names (`golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs`, history kept); `lamina-cos` keeps reading and the overlay only.
- The `lamina-cos` tests that stay still build their inputs with the writer: `tests/common/mod.rs` imports `XrefEol` and `incremental_update` and is pulled into every integration test by `mod common;`, `common/samples.rs` uses `Writer`, and `roundtrip.rs`, `policy.rs`, `reader_cache.rs` and `review_fixes.rs` call `Writer`/`incremental_update`. They keep compiling through a `[dev-dependencies] lamina-save = { path = "../lamina-save" }` in `lamina-cos` (Cargo allows this dev-dependency cycle; the integration tests link the single `lamina-cos` that `lamina-save` uses) with imports switched to `lamina_save::…`. `roundtrip.rs` (write → read round trips) moves to `lamina-save` too; `review_fixes.rs` is split: writer cases move, reader cases stay.
- `WriteSink`, `ReadableSink`, `SinkError`; impls for `Vec<u8>` and `IoWrite<W: std::io::Write>` (behind `std`, still core-safe: no fs); `CountingSink`.
- `IncrementalWriter` over base `ByteSource` + overlay; `FullWriter` (reachability walk from trailer roots, keep numbers, `Compact` with an old → new map, object streams + xref stream or classic); `SaveOptions { mode, output: IncrementalOutput, renumber, xref: XrefKind, eol }` subset needed here; `SaveReport` (bytes written, mode, renumber map, append offset).
- `/ID`: keep `[0]`, `[1]` = hash of content written (SHA-256 truncated to 16 bytes, deterministic), `[0]` derived the same way for new documents or when missing.
- `Document::save_incremental`/`save_full` (temporary names until B3c's `Document::save(sink, options)`), rebase: incremental output builds the new base in memory (old base ‖ appended); full rewrite rebases from a `ReadableSink` or enters `SavedNotRebased` and `Document::rebase(source)`; ops and saves refused in `SavedNotRebased`; `LaminaId` map kept (renumber map applied on compact).
- Refusals carried over: repaired base (`RepairedBase`) for incremental; encrypted base (`EncryptedBase`) for **both** writers. B3b has no decryption ([B2b](./B2b-standard-security-decryption.md)) and no encryption on write (E11b), so a full rewrite of an encrypted base would either copy ciphertext without `/Encrypt` or keep `/Encrypt` over objects it can't re-encrypt; `FullWriter` refuses before writing anything ([ADR 0007 §2](../../decisions/0007-save-planner.md#2-saveconstraint-all-the-conditions-not-one-of-them): an encrypted base "can only be saved by removing encryption"). The explicit `remove_encryption` option and its planner rules are [B3c](./B3c-save-planner-and-roundtrip.md)'s.

**Out of scope** (and which todo covers it)

- `SavePlanner`, `SaveConstraint`, `SaveMode::Auto` resolution, signature detection: [B3c](./B3c-save-planner-and-roundtrip.md).
- Round-trip with pdf.js/PDFium/qpdf: [B3c](./B3c-save-planner-and-roundtrip.md).
- Signing phase, encryption on write: full-editor E10, E11.
- Edge sinks (OPFS spool, files): web/desktop rounds.

## Tasks

- [ ] Load `/rust-best-practices`; read ADR 0007 §5–7 and ADR 0005 §3
- [ ] Create `crates/lamina-save` (core); `git mv crates/lamina-cos/src/writer.rs crates/lamina-save/src/legacy.rs` (or `writer/`); `git mv` `golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs` and `roundtrip.rs`, and the writer cases of `review_fixes.rs`, into `crates/lamina-save/tests/` (with a copy of the `common/` helpers they use); keep goldens byte-identical
- [ ] In `lamina-cos`: add the `lamina-save` dev-dependency, switch `tests/common/mod.rs`, `common/samples.rs`, `policy.rs`, `reader_cache.rs` and the remaining `review_fixes.rs` cases to `lamina_save::{Writer, XrefEol, incremental_update}`; `cargo test -p lamina-cos` and `cargo test -p lamina-save` both compile and pass
- [ ] `sink.rs`: traits, `Vec<u8>`/`IoWrite` impls, `CountingSink`
- [ ] `incremental.rs`: stream base bytes in chunks from `ByteSource::read_at`, append overlay entries in object-number order, free entries for deletes (gen + 1), xref section matching the newest kind, trailer (`/Size` = max(old, new), `/Prev`, `/Root`, `/Info`, `/ID`, drop `/Type /XRef`-only keys)
- [ ] `full.rs`: reachability walk (iterative, bounded by `Limits`), `Renumber::{Keep, Compact}`, object streams packing (non-stream objects, deterministic grouping), xref stream writer, classic option
- [ ] `id.rs`: `/ID` derivation
- [ ] Rebase in `lamina-cos`/`lamina-save` boundary: new base source, reload xref, clear overlay and journal, keep IDs, state machine `Clean | Dirty | SavedNotRebased`
- [ ] Tests listed below; `cargo xtask golden --update` covers any new goldens (e.g. `fixtures/golden/full-rewrite-hello.pdf`, `incremental-set-info.pdf`)
- [ ] Update xtask crate lists; `CLAUDE.md` layout comment for `lamina-cos` ("reader and overlay") and `lamina-save`

## Acceptance criteria

- [ ] Moved tests (`golden`, `determinism`, `incremental`, `raw_writer`, `roundtrip`, writer cases of `review_fixes`) pass unchanged in `lamina-save` with byte-identical goldens
- [ ] Every `lamina-cos` integration test that stays (`corpus`, `robustness`, `policy`, `reader_cache`, `reader_tolerance`, `limits`, `warnings`, `codes`, `source`, `content`, reader cases of `review_fixes`) compiles and passes, with `lamina-cos` having no normal dependency on `lamina-save`
- [ ] Empty overlay + incremental save writes exactly the base bytes
- [ ] Open → `SetInfo` → incremental save → reopen shows the new Info, `revisions()` + 1, base bytes are a prefix of the output, `/ID[0]` unchanged
- [ ] Open → `RotatePage` → full save (`Keep`) → reopen: same object numbers, same `PageId`s, unreachable objects gone
- [ ] `Compact` returns a complete renumber map and IDs follow it
- [ ] Every writer test saves twice and compares bytes (determinism)
- [ ] After a successful save the document is `Clean`; after a failed save (sink error injected) it stays `Dirty` with the overlay untouched
- [ ] A full rewrite to a non-readable sink leaves `SavedNotRebased`; ops and saves fail until `rebase`
- [ ] Incremental **and** full saves of an encrypted base (`fixtures/corpus/enc.pdf`) fail with `EncryptedBase`, write nothing to the sink and leave the document `Dirty` with the overlay untouched

## Post-merge checks

None.

## Tests

- `crates/lamina-save/tests/`: moved `golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs`, `roundtrip.rs` and the writer half of `review_fixes.rs`; new `overlay_incremental.rs`, `full_rewrite.rs`, `rebase.rs`, `id.rs`, `sink_errors.rs`, `encrypted_base.rs` (both writers refuse `enc.pdf`).
- Property test: random op sequences → save (both modes) → reopen → model facts equal the in-memory view.
- Goldens: new byte-exact outputs in `fixtures/golden/` (generated by `cargo xtask golden --update`).

## Docs to update

- [ ] [pdf-library §10 B3](../../plans/pdf-library.md#10-to-dos): tick "Create `crates/lamina-save` … writers, `WriteSink`"
- [ ] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "Move the writer …" and partly `lamina-save`
- [ ] `CLAUDE.md` (layout, `cargo test -p lamina-save`), crate READMEs, `fixtures/README.md`

## Risks and open questions

- Object-stream grouping must be deterministic and must not put the encryption dictionary, xref streams or stream objects inside object streams (§7.5.7).
- Streaming the base for incremental output with a `Pending` source: propagate `DataPending` before writing anything (check all ranges first).
- The `lamina-cos` → `lamina-save` dev-dependency is a cycle Cargo accepts for integration tests, but `lamina-cos` unit tests (`src/**` `#[cfg(test)]`) must not use `lamina_save`, or they see two copies of `lamina-cos` types. If that bites, `common/samples.rs` can embed its sample PDFs as committed bytes instead; record the choice in the PR.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
