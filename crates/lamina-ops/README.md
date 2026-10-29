# lamina-ops

Editing PDFs through typed, serialisable ops ([ADR 0006](../../docs/decisions/0006-op-log-and-undo.md)), over the overlay of [`lamina-cos`](../lamina-cos/README.md) and the views of [`lamina-model`](../lamina-model/README.md). Part of lamina's pure core: bytes in, bytes out.

- `Op` (adjacently tagged JSON, `{ "op": "rotatePage", "args": { "page": "p12.0", "rotate": 90 } }`, unknown ops and fields rejected): `SetInfo { patch: InfoPatch, at: Option<PdfDate> }` and `RotatePage { page: PageId, rotate: Rotation }`, each with a `ModificationClass` and `requires_rewrite()`. `OpLog` is the versioned envelope `{ "v": 1, "ops": [ … ] }`; `OpLog::from_json` rejects an unknown `v` before reading any op. `BlobId` (a SHA-256 content hash) is declared for later large payloads.
- `Document`: wraps a `lamina_model::Document` (it dereferences to one, read-only) and is the only write path. `transact(origin, |tx| …)` is atomic (on `Err` the journal unwinds and nothing is recorded); `Tx::apply` rolls back a failing op's own changes; `Tx::savepoint` rolls back only its closure's; `begin` / `apply` / `commit` / `rollback` is the same across calls for bindings; a transaction inside an open one is `Error::NestedTransaction`. `apply_all(origin, ops)` runs a list as one transaction.
- `Applied { undo: UndoRecord, touched_pages }`: the undo record is derived from the COS journal (each changed object's or the trailer's value before and after, `properties()` per dictionary key), so every op gets undo without a hand-written inverse.
- `undo()` / `redo()` (local transactions only; depth 500, `set_undo_depth`; redo cleared by a new local transaction; `transact_coalescing(key, …)`) restore each property that still holds the value the transaction set and leave the rest: `UndoOutcome::{Applied, Partial { skipped }, Skipped { skipped }}`. Undoing to the opened state leaves the overlay empty (`is_dirty()` false); `has_unsaved_changes()` / `mark_saved()` are the semantic dirty level.
- Feature `tsify` (off by default; `lamina-wasm` enables it): derives `tsify::Tsify` on the op types for TypeScript declarations.

Tests: unit tests in `src/{record,tx,history}.rs`; `tests/ops.rs` (both ops over corpus files with indirect, direct and missing `/Info` and a file with inherited `/Rotate`; transactions and savepoints), `undo.rs` (undo, redo, remote changes, coalescing, dirty levels), `serde.rs` (the frozen samples in [`fixtures/ops/v1/`](../../fixtures/README.md#opsv1), rejected input, error codes) and `props.rs` (proptest, fixed seed: random transactions, undo all → empty overlay, redo all → the same overlay).

```sh
cargo test -p lamina-ops
```
