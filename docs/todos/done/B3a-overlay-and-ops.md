# B3a — COS overlay and `lamina-ops`: `Op`, `transact`, savepoints, undo/redo

| Field | Value |
| --- | --- |
| ID | `B3a` |
| Status | `done` |
| Plan | [pdf-library §7 B3](../../plans/pdf-library.md#b3-ops-and-save), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md) |
| Blocks | [B3b](../ready/B3b-lamina-save-writers.md) |
| ADRs | Implements [0005](../../decisions/0005-document-model.md) §3 (overlay, journal, number allocation) and [0006](../../decisions/0006-op-log-and-undo.md) §1–§4, §6–§7; follows [0018](../../decisions/0018-test-strategy.md) tier 5 (property tests) |
| Branch | `feat/b3a-overlay-and-ops` |
| PR title | `feat(ops): COS overlay, typed Op enum, transactions with savepoints, undo and redo` |
| Size | L (≈ 1,000–1,400 lines) |
| PR | — |

## Goal

Make the document editable through one audited path: an overlay in `lamina-cos` that is the only way to mutate objects (so dirty tracking is automatic and PDFBox's lost-changes bug is unrepresentable), and `lamina-ops` with the typed, serialisable `Op` enum, atomic transactions, savepoints and undo/redo computed from the current state. The two proving ops, `SetInfo` and `RotatePage`, exercise every part of the mechanism. Saving the result is [B3b](../ready/B3b-lamina-save-writers.md).

## Context

- [ADR 0005 §3](../../decisions/0005-document-model.md#3-base-overlay-and-the-save-cycle): `New | Modified | Deleted`, journal per transaction, entries equal to base are dropped, fresh numbers above every xref section's highest, numbers never reused, undone creations stay reserved. States `Clean`/`Dirty` (rebase is B3b).
- [ADR 0006](../../decisions/0006-op-log-and-undo.md): §1 `Op` (adjacently tagged `op`/`args`, `deny_unknown_fields`, no ambient inputs, `ModificationClass`, `requires_rewrite`), §2 `transact`, `NestedTransaction`, `savepoint`, `Origin`, §3 `CosMut` + `CosChange` + `Applied` (undo record, touched pages), §4 undo at undo time with `UndoOutcome::{Applied, Partial, Skipped}`, depth 500, coalescing, §6 envelope `{ "v": 1, "ops": [...] }` and `fixtures/ops/v1/`, §7 two dirty levels.
- [Editing notes §2](../../research/architecture/notes/editing-architecture.md#2-command-pattern--operation-log-undoredo-transactions-dirty-tracking).
- [Research 04 §5.1 Info](../../research/pdf/04-interactive-security.md#51-the-info-dictionary-1433); ISO 32000-2 §14.3.3 (Info), §7.7.3.3 (`/Rotate`, multiple of 90, inheritable).
- Code: `lamina-model` from [B2c](../done/B2c-lamina-model-corpus-fuzz.md) (`PageId`, `Page::rotate`), [`lamina-syntax/src/object.rs`](../../../crates/lamina-syntax/src/object.rs) (`Dict` keeps key order).

## Scope

**In scope**

- `lamina-cos::overlay`: `Overlay` (`BTreeMap<u32, Entry>`), `CosMut` (`create`, `set`, `delete`, `get` reading overlay then base), `CosChange` journal, rollback to a mark, equality-with-base drop, allocator (`next_number`, reserved numbers), `Document::is_dirty()` (COS level), resolution through overlay first.
- `crates/lamina-ops` (core): `Op` (`SetInfo`, `RotatePage`), payloads `SetInfo { patch: InfoPatch, at: Option<PdfDate> }` (`InfoPatch` fields `Option<Option<String>>` to set/clear; `ModDate` only from `at`), `RotatePage { page: PageId, rotate: Rotation }` (absolute; `Rotation` = 0/90/180/270), `ModificationClass`, `requires_rewrite()`.
- `Tx` (`apply`, `savepoint`), `Document::transact(origin, |tx| …)`, `Error::NestedTransaction`, `Applied { undo: UndoRecord, touched_pages }`.
- `History` (undo/redo stacks, depth 500 configurable, local origin only, redo cleared on new local tx, `transact_coalescing(key, …)`), `undo()`/`redo()` → `UndoOutcome`.
- Semantic dirty state (local transactions since last save) next to COS dirty.
- serde JSON: envelope `OpLog { v: 1, ops }` with `deny_unknown_fields`; `fixtures/ops/v1/*.json` samples; `tsify` attributes behind a `tsify` feature (off by default; enabled by `lamina-wasm` in B6a).
- `BlobId` type declared (content hash) for later large payloads; not used yet.

**Out of scope** (and which todo covers it)

- Writing bytes, rebasing after save, `SavePlanner`: [B3b](../ready/B3b-lamina-save-writers.md), [B3c](../ready/B3c-save-planner-and-roundtrip.md).
- Barrier ops (`ApplyRedactions`, `Sign`, encryption): full-editor E10–E12.
- Other ops (`MovePages`, annotations, …): full-editor E-rounds.
- XMP sync with Info: full-editor E1.
- `Remote` origin semantics beyond "not undoable": collaboration (ADR 0012).

## Tasks

- [x] Load `/rust-best-practices`; read ADR 0005 §3 and ADR 0006 in full
- [x] `crates/lamina-cos/src/overlay.rs` + integrate in `Document` resolution; unit tests for create/set/delete/rollback/equal-to-base drop and number allocation (never reused, reserved after undo). The integration in `reader/mod.rs` is one appended `impl` block plus a field and the `resolve`/`object`/`trailer`/`catalog` entry points, so B2d's windowed-reading changes there merge cleanly. `lamina_model::IdAllocator` became a borrowed view of the overlay's allocator (`Document::ids()` returns it by value), so there is one number source
- [x] Create `crates/lamina-ops` (core layer); `op.rs`, `info.rs` (`InfoPatch`), `rotate.rs`, `class.rs`. PDF text-string encoding (PDFDocEncoding when representable, else UTF-16BE with BOM) goes in `lamina_syntax::text` as `encode_text_string`, next to the table and decoder that [B2c](../done/B2c-lamina-model-corpus-fuzz.md) added; reuse that table rather than writing another (also UTF-16BE when the PDFDocEncoding bytes would start with a byte order mark). Plus `record.rs` (undo records), `blob.rs` (`BlobId`), `wire.rs` (serde forms of `PageId`, `PdfDate`, `Rotation`, since `lamina-model` has no serde)
- [x] `tx.rs`: `Tx`, journal marks, savepoints, error unwinding; `Document::transact` lives in `lamina-ops` as an extension trait or wrapper type (decide; ADR 0005 §1 layering). **Decided: a wrapper type**, `lamina_ops::Document` over `lamina_model::Document`; recorded in [ADR 0006 §8](../../decisions/0006-op-log-and-undo.md#8-implementation-notes-b3a). The handle form (`begin`/`apply`/`commit`/`rollback`) is there too, which is what makes `NestedTransaction` reachable (inside a closure the type system already prevents nesting)
- [x] `history.rs`: undo record capture (`target`, `before`, `after` per property), inverse computed at undo time, outcome reporting, coalescing. The record is derived from the COS journal (per object or trailer, `properties()` per dictionary key) rather than written by each op
- [x] serde: adjacently tagged enum, `deny_unknown_fields`, envelope with version check; `fixtures/ops/v1/set-info.json`, `rotate-page.json`, `envelope.json`
- [x] `RotatePage` writes `/Rotate` on the page object itself (materialising an inherited value), never replaces the page object (ADR 0005 §5)
- [x] proptest: random sequences of `SetInfo`/`RotatePage` → apply, undo all → overlay empty; redo all → same overlay as before undo (grouped into random multi-op and coalescing transactions, over four files)
- [x] Wire `lamina-ops` into xtask's crate lists (portability, wasm check): nothing to change, xtask derives the lists from `[package.metadata.lamina] layer = "core"`

## Acceptance criteria

- [x] Mutation is impossible except through `CosMut` inside a transaction (no public `&mut` path to base objects; a compile-fail doc test or API review note). Base objects are never mutable: the only write path is `lamina_cos::Document::edit()` → `CosMut`, which journals into the overlay. `lamina_ops::Document` exposes no `&mut` to the model or COS document (compile-fail doc test on `Document`); code holding a bare `lamina_cos::Document` can still call `edit()` outside a `lamina-ops` transaction, and those writes are journaled and dirty-tracked all the same (API review note in `overlay.rs`)
- [x] A failing transaction leaves the overlay exactly as before (test with a mid-transaction error)
- [x] `transact` inside `transact` returns `NestedTransaction`; a failing savepoint rolls back only its own changes
- [x] Apply then undo leaves the overlay empty for every op (property test, fixed seed printed)
- [x] Undo after a simulated remote change to the same property returns `Partial` and leaves the remote value
- [x] JSON samples in `fixtures/ops/v1/` deserialise and apply; an unknown op, an unknown field and an unknown `v` each fail
- [x] `RotatePage` keeps the `PageId` and the page object number

## Post-merge checks

None.

## Tests

- Unit tests in `overlay.rs`, `tx.rs`, `history.rs`.
- `crates/lamina-ops/tests/{ops,undo,serde,props}.rs` over `fixtures/corpus` files (inherited `/Rotate`, missing `/Info`, Info as indirect vs direct).
- Fixtures: `fixtures/ops/v1/*.json` (frozen; never rewritten by a test).

## Docs to update

- [x] [pdf-library §10 B3](../../plans/pdf-library.md#10-to-dos): tick `lamina-ops` and (partly) the proving-ops to-do
- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick `lamina-ops`
- [x] `CLAUDE.md` layout (new crate), `fixtures/README.md` (`ops/`), crate READMEs (also the root `README.md`, ADR 0006 §8, and `opErrors` in `fixtures/codes.json`)

## Risks and open questions

- Where `transact` lives (on `Document` in `lamina-cos`, or a `lamina-ops` wrapper) shapes the façade; decide in the PR and note it in ADR 0006 if it differs from the sketch.
- Info as a direct dictionary in the trailer vs indirect: an edit must create an indirect `/Info` and update the trailer in the overlay; the trailer becomes an overlay-managed item.
- Undo records hold `Object` clones; bound memory by depth only for now; measure in nightly perf later.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Deviations

- **`tsify` feature pulls `wasm-bindgen`.** tsify 0.5.8 can't build without its `js` or `json` feature, and its derive expands to `#[wasm_bindgen]` glue, so the optional `tsify` feature enables `tsify` (with `js`) and `wasm-bindgen` as optional dependencies of this core crate. Both stay out of default builds, so `cargo xtask portability` (default features) is unaffected; `lamina-wasm` turns the feature on in B6a.
- **`RotatePage` on a direct page dictionary** returns `Error::DirectPage` instead of promoting the page to an indirect object (ADR 0005 §5); promotion arrives with the page-structure ops.
- **New error codes** go in `fixtures/codes.json` under `opErrors` (`nested-transaction`, `no-transaction`, `unknown-page`, `direct-page`, `unsupported-version`, `invalid-op-log`); COS errors keep their codes. `lamina-cos` got no new error variant (overlay misuse is `invalid-structure`), to stay clear of B2d's `error.rs` changes.
- **`UndoOutcome::Skipped`** carries the skipped properties (`Skipped { skipped }`), like `Partial`.
- **Undo records hold `Arc<Object>` values** (shared with the overlay), not deep clones; memory is still bounded only by the depth.
