# B3a — COS overlay and `lamina-ops`: `Op`, `transact`, savepoints, undo/redo

| Field | Value |
| --- | --- |
| ID | `B3a` |
| Status | `ready` |
| Plan | [pdf-library §7 B3](../../plans/pdf-library.md#b3-ops-and-save), [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md) |
| Blocks | [B3b](./B3b-lamina-save-writers.md) |
| ADRs | Implements [0005](../../decisions/0005-document-model.md) §3 (overlay, journal, number allocation) and [0006](../../decisions/0006-op-log-and-undo.md) §1–§4, §6–§7; follows [0018](../../decisions/0018-test-strategy.md) tier 5 (property tests) |
| Branch | `feat/b3a-overlay-and-ops` |
| PR title | `feat(ops): COS overlay, typed Op enum, transactions with savepoints, undo and redo` |
| Size | L (≈ 1,000–1,400 lines) |
| PR | — |

## Goal

Make the document editable through one audited path: an overlay in `lamina-cos` that is the only way to mutate objects (so dirty tracking is automatic and PDFBox's lost-changes bug is unrepresentable), and `lamina-ops` with the typed, serialisable `Op` enum, atomic transactions, savepoints and undo/redo computed from the current state. The two proving ops, `SetInfo` and `RotatePage`, exercise every part of the mechanism. Saving the result is [B3b](./B3b-lamina-save-writers.md).

## Context

- [ADR 0005 §3](../../decisions/0005-document-model.md#3-base-overlay-and-the-save-cycle): `New | Modified | Deleted`, journal per transaction, entries equal to base are dropped, fresh numbers above every xref section's highest, numbers never reused, undone creations stay reserved. States `Clean`/`Dirty` (rebase is B3b).
- [ADR 0006](../../decisions/0006-op-log-and-undo.md): §1 `Op` (adjacently tagged `op`/`args`, `deny_unknown_fields`, no ambient inputs, `ModificationClass`, `requires_rewrite`), §2 `transact`, `NestedTransaction`, `savepoint`, `Origin`, §3 `CosMut` + `CosChange` + `Applied` (undo record, touched pages), §4 undo at undo time with `UndoOutcome::{Applied, Partial, Skipped}`, depth 500, coalescing, §6 envelope `{ "v": 1, "ops": [...] }` and `fixtures/ops/v1/`, §7 two dirty levels.
- [Editing notes §2](../../research/architecture/notes/editing-architecture.md#2-command-pattern--operation-log-undoredo-transactions-dirty-tracking).
- [Research 04 §5.1 Info](../../research/pdf/04-interactive-security.md#51-the-info-dictionary-1433); ISO 32000-2 §14.3.3 (Info), §7.7.3.3 (`/Rotate`, multiple of 90, inheritable).
- Code: `lamina-model` from [B2c](./B2c-lamina-model-corpus-fuzz.md) (`PageId`, `Page::rotate`), [`lamina-syntax/src/object.rs`](../../../crates/lamina-syntax/src/object.rs) (`Dict` keeps key order).

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

- Writing bytes, rebasing after save, `SavePlanner`: [B3b](./B3b-lamina-save-writers.md), [B3c](./B3c-save-planner-and-roundtrip.md).
- Barrier ops (`ApplyRedactions`, `Sign`, encryption): full-editor E10–E12.
- Other ops (`MovePages`, annotations, …): full-editor E-rounds.
- XMP sync with Info: full-editor E1.
- `Remote` origin semantics beyond "not undoable": collaboration (ADR 0012).

## Tasks

- [ ] Load `/rust-best-practices`; read ADR 0005 §3 and ADR 0006 in full
- [ ] `crates/lamina-cos/src/overlay.rs` + integrate in `Document` resolution; unit tests for create/set/delete/rollback/equal-to-base drop and number allocation (never reused, reserved after undo)
- [ ] Create `crates/lamina-ops` (core layer); `op.rs`, `info.rs` (`InfoPatch`, PDF text-string encoding: PDFDocEncoding when representable, else UTF-16BE with BOM), `rotate.rs`, `class.rs`
- [ ] `tx.rs`: `Tx`, journal marks, savepoints, error unwinding; `Document::transact` lives in `lamina-ops` as an extension trait or wrapper type (decide; ADR 0005 §1 layering)
- [ ] `history.rs`: undo record capture (`target`, `before`, `after` per property), inverse computed at undo time, outcome reporting, coalescing
- [ ] serde: adjacently tagged enum, `deny_unknown_fields`, envelope with version check; `fixtures/ops/v1/set-info.json`, `rotate-page.json`, `envelope.json`
- [ ] `RotatePage` writes `/Rotate` on the page object itself (materialising an inherited value), never replaces the page object (ADR 0005 §5)
- [ ] proptest: random sequences of `SetInfo`/`RotatePage` → apply, undo all → overlay empty; redo all → same overlay as before undo
- [ ] Wire `lamina-ops` into xtask's crate lists (portability, wasm check)

## Acceptance criteria

- [ ] Mutation is impossible except through `CosMut` inside a transaction (no public `&mut` path to base objects; a compile-fail doc test or API review note)
- [ ] A failing transaction leaves the overlay exactly as before (test with a mid-transaction error)
- [ ] `transact` inside `transact` returns `NestedTransaction`; a failing savepoint rolls back only its own changes
- [ ] Apply then undo leaves the overlay empty for every op (property test, fixed seed printed)
- [ ] Undo after a simulated remote change to the same property returns `Partial` and leaves the remote value
- [ ] JSON samples in `fixtures/ops/v1/` deserialise and apply; an unknown op, an unknown field and an unknown `v` each fail
- [ ] `RotatePage` keeps the `PageId` and the page object number

## Tests

- Unit tests in `overlay.rs`, `tx.rs`, `history.rs`.
- `crates/lamina-ops/tests/{ops,undo,serde,props}.rs` over `fixtures/corpus` files (inherited `/Rotate`, missing `/Info`, Info as indirect vs direct).
- Fixtures: `fixtures/ops/v1/*.json` (frozen; never rewritten by a test).

## Docs to update

- [ ] [pdf-library §10 B3](../../plans/pdf-library.md#10-to-dos): tick `lamina-ops` and (partly) the proving-ops to-do
- [ ] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick `lamina-ops`
- [ ] `CLAUDE.md` layout (new crate), `fixtures/README.md` (`ops/`), crate READMEs

## Risks and open questions

- Where `transact` lives (on `Document` in `lamina-cos`, or a `lamina-ops` wrapper) shapes the façade; decide in the PR and note it in ADR 0006 if it differs from the sketch.
- Info as a direct dictionary in the trailer vs indirect: an edit must create an indirect `/Info` and update the trailer in the overlay; the trailer becomes an overlay-managed item.
- Undo records hold `Object` clones; bound memory by depth only for now; measure in nightly perf later.

## Definition of done

- [ ] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (status field updated, PR link added)
- [ ] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [ ] Squash-merged; dependants' "Depends on" now satisfied
