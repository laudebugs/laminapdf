# 0006 — Op log, transactions and undo

- Status: Accepted
- Date: 2026-10-28

## Context

Principle 2 of the [architecture plan](../plans/architecture.md#principles-each-becomes-part-of-an-adr) is that every change is a typed `Op`: undo, dirty tracking, collaboration sync and the public API consume the same stream. The research recommends typed semantic ops grouped into transactions that roll back on error, inverse ops for undo computed at undo time (Figma), undo scoped to local changes (Yjs `UndoManager`), and two **barrier** ops, redaction and signing, that can't be undone once saved ([report: editing](../research/architecture/app-and-package-architecture.md#editing-is-an-op-log-over-a-lazy-object-graph-and-signatures-and-redaction-govern-saving); [editing notes §2](../research/architecture/notes/editing-architecture.md#2-command-pattern--operation-log-undoredo-transactions-dirty-tracking)). The [full editor](../plans/full-editor.md) adds about 180 operations across rounds E1–E18, each shipped to Rust, WASM and TypeScript in the same PR; library B3 proves the design with `SetInfo` and `RotatePage` and requires that undo restores byte-identical output ([pdf-library B3](../plans/pdf-library.md#b3-ops-and-save)).

## Decision

### 1. The `Op` enum (`lamina-ops`)

```rust
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum Op {
    SetInfo { patch: InfoPatch },                    // B3
    RotatePage { page: PageId, rotate: Rotation },   // B3
    // later rounds: MovePages, InsertPages, DeletePages, AddAnnot, UpdateAnnot, DeleteAnnot,
    // SetField, AppendContent, EditContent, MarkRedaction, ApplyRedactions, …
}
```

- **Typed and semantic.** Payloads name things by `LaminaId` ([ADR 0005](0005-document-model.md)), never by object reference, and use domain types (`Rotation`, `InfoPatch`), not raw COS objects. Invalid states are unrepresentable where the type system allows it (`Rotation` is a multiple of 90).
- **Serialisable.** serde, with tsify generating the TypeScript types ([ADR 0002](0002-binding-strategy.md)). The JSON form crosses the WASM boundary and is what sync sends (ADR 0012). Unknown ops and unknown fields are rejected, never skipped.
- **No ambient inputs.** Anything from the environment, like a modification date, is in the payload (`at: Option<PdfDate>`), so applying the same op to the same document gives the same result on every platform ([ADR 0001](0001-core-language-and-licence.md)). Large binary payloads (images, fonts) are referenced by content hash (`BlobId`) into a blob store held by the document, so ops stay small.
- **Classified.** Each op reports a `ModificationClass` (`Metadata`, `FormFill`, `Annotation`, `Signature`, `PageStructure`, `Content`, `Document`) and whether it `requires_rewrite`. The save planner uses these to apply DocMDP/FieldMDP rules and `SaveConstraint` ([ADR 0007](0007-save-planner.md)).

### 2. Transactions

```rust
doc.transact(|tx| {
    tx.apply(Op::RotatePage { page, rotate: Rotation::R90 })?;
    tx.apply(Op::SetInfo { patch })?;
    Ok(())
})?;
```

- A transaction is atomic. On `Err` the overlay journal is unwound in reverse and nothing is recorded. There is no nesting: `transact` inside a transaction joins it.
- Each transaction carries an `Origin`: `Local`, `Remote(PeerId)` or `System` (for example a migration on open).
- TypeScript gets the same shape through a handle (`doc.transact(tx => …)` wraps `begin` / `apply` / `commit` / `rollback` in `try/finally`), because a closure can't cross the WASM boundary. `doc.apply(ops)` runs a list as one transaction.

### 3. How ops map to COS changes

`Op::apply(&self, tx: &mut Tx) -> Result<Applied>` reads through views and writes only through `CosMut`, which records each `CosChange` (`Create`, `Set { before, after }`, `Delete`) in the transaction journal and the overlay. `Applied` carries the **inverse op**, computed from the state at apply time, and the set of touched pages, so viewers re-render only those. Ops never write bytes. Two rules make undo byte-exact: dictionary edits keep key order (`Dict` is an ordered list), and an overlay entry equal to the base is dropped ([ADR 0005](0005-document-model.md) §3), so undoing to the saved state leaves the overlay empty and the next save identical to the last.

### 4. Undo and redo

- **Per document, local origin only.** Only `Local` transactions are pushed. Remote and system transactions are not undoable by this user and never rewind.
- **Inverses are recomputed at undo time** (Figma): undo applies the stored inverse as a new `Local`-but-unrecorded transaction, and records *its* inverse, computed from the state at that moment, as the redo entry. Redo does the mirror. If an entry no longer applies (its target was deleted remotely), it is dropped and the call returns `UndoOutcome::Skipped`.
- **Depth:** 500 transactions by default, configurable. Coalescing (typing into a field) is opt-in: `transact_coalescing(key, …)` merges into the previous entry with the same key.
- **Redo stack** is cleared by any new local transaction.

### 5. Barrier ops

`ApplyRedactions`, signing, and removing or changing encryption (full-editor E10–E12) are barriers.

- **Before save** they are undoable like anything else, but their inverse is a snapshot of the overlay entries they touched (`Inverse::Snapshot`), because a semantic inverse of a destructive edit doesn't exist. Snapshot inverses are local-only and never serialised or synced; in collaboration these ops take a lease (ADR 0012).
- **A save that commits a barrier segments the stack:** all undo and redo entries from before that save are dropped. Undo can't reach past applied redactions or a signature.
- **Signing is executed by the save planner** as its final step ([ADR 0007](0007-save-planner.md)); the `Sign` op only records the request. A successful signing save clears the stack.
- Saves that commit no barrier keep the undo stack; undoing after such a save produces new changes against the saved revision.

### 6. Versioning

- Logs and sync messages carry a schema version in an envelope: `{ "v": 1, "ops": [ … ] }`. Op names and field names are stable camelCase strings.
- Adding an op or an optional field is compatible and keeps `v`. Older readers reject what they don't know, so peers negotiate the version on connect (ADR 0012).
- Changing the meaning of a field, or removing one, bumps `v` and adds a `migrate_vN_to_vN+1` function in `lamina-ops`. Every older version stays loadable.
- Frozen JSON samples of each version live in `fixtures/ops/v1/…` and are deserialised and applied in tests ([ADR 0018](0018-test-strategy.md)).

### 7. Dirty state

Two levels: **semantic** (local transactions since the last save; the UI's "unsaved" dot) and **COS** (the overlay is non-empty; the authority the save planner uses).

## Consequences

- One mechanism serves the API, the editor's undo, history panels, batch processing (a saved op list applied to many files, full-editor E14) and sync.
- Every new op costs an `apply`, an inverse, a class, serde and a TS type, and needs property tests (apply then undo = original overlay).
- Barrier semantics keep the redaction and signature traps from [research ch. 04 §10](../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do) out of reach of a casual undo.
- Rejecting unknown ops makes mixed-version collaboration explicit rather than lossy.

## Alternatives considered

- **COS-level undo only** (snapshot every changed object). Exact and simple for one user, but it clobbers interleaved remote changes and can't be synced.
- **Inverses stored once at apply time and replayed blindly.** Wrong when other changes intervened; Figma's recompute rule fixes it.
- **A CRDT over the object graph.** Deferred with collaboration; see the report and ADR 0012.
- **Silently skipping unknown ops** for forward compatibility. Loses edits without telling anyone.
- **Global undo across all users.** Users expect to undo their own work, not a collaborator's (Yjs, Figma).
