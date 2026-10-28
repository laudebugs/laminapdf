# 0007 — Save planner and `SaveConstraint`

- Status: Accepted
- Date: 2026-10-28

## Context

How a PDF is saved decides whether signatures stay valid and whether "deleted" content is really gone. An incremental update appends a revision and leaves every earlier byte in the file (ISO 32000-1 §7.5.6). A signature covers a `/ByteRange` of the file as it was when signed, so a full rewrite that moves bytes invalidates it (§12.8; [research ch. 04 §8.7](../research/pdf/04-interactive-security.md#87-incremental-updates-after-signing), [§9](../research/pdf/04-interactive-security.md#9-signing-via-incremental-update-practical-constraints)). After certification, DocMDP (§12.8.2.2) limits which incremental changes are allowed; usage-rights signatures (§12.8.2.3) and FieldMDP (§12.8.2.4) add more. Applying redaction annotations (§12.5.6.23) is only real if no earlier revision survives, which takes a full rewrite ([ch. 04 §10.2](../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do)).

Other engines leave this to callers: PDFium exposes two mutually exclusive flags, LibPDF can't detect when incremental saves are required ([report: editing](../research/architecture/app-and-package-architecture.md#editing-is-an-op-log-over-a-lazy-object-graph-and-signatures-and-redaction-govern-saving)). Principle 3 of the [architecture plan](../plans/architecture.md#principles-each-becomes-part-of-an-adr) makes the planner own this. Today the writer (`Writer`, `incremental_update`, `XrefEol`) lives in `crates/lamina-cos/src/writer.rs` and already refuses repaired and encrypted bases.

## Decision

### 1. API (`lamina-save`)

```rust
let plan = SavePlanner::new(&doc).plan(&SaveOptions { mode: SaveMode::Auto, ..Default::default() })?;
plan.constraint();          // SaveConstraint
plan.mode();                // the resolved mode: Incremental or FullRewrite
let report = plan.write(&mut sink)?;   // SaveReport
```

`Document::save(sink, options)` is the shorthand. Planning is separate from writing so the UI can show the constraint and the consequences before anything is written.

- `SaveMode::Incremental | FullRewrite | Auto` (default `Auto`).
- `SaveConstraint::Free | AppendOnly | RequiresRewrite`, computed from the base file plus the overlay ([ADR 0005](0005-document-model.md)).
- `SaveReport`: bytes written, the resolved mode, signatures invalidated (if any), the old → new object-number map when renumbered, and the offset where appended bytes start.

### 2. Detecting the constraint

**`RequiresRewrite`** when any of:
- the overlay contains an applied redaction (`ApplyRedactions`), or any op that reports `requires_rewrite` ([ADR 0006](0006-op-log-and-undo.md)), such as sanitising or optimising;
- encryption is being removed or changed (old revisions would keep the old encryption or plaintext);
- the base was repaired ([ADR 0003](0003-reader-tolerance-and-limits.md)): a revision appended to an unusable xref would be unusable too (today's `RepairedBase`).

**`AppendOnly`** when the base has any signature: a signature field (`/FT /Sig`) whose `/V` is a signature dictionary, a document timestamp (`/Type /DocTimeStamp`), or `/Perms` with `/DocMDP` or `/UR3`. Within `AppendOnly` the planner also checks each pending transaction's `ModificationClass` against:
- **DocMDP** `/P` of the certification signature: 1 allows nothing; 2 allows form filling, page templates and signing; 3 adds annotations;
- **FieldMDP** and `/Lock` dictionaries: changes to locked fields are violations;
- **UR3**: changes outside the granted rights invalidate usage rights.

**`Free`** otherwise. A newly built document is `Free` with no base.

Checks run at plan time, and the editor can ask earlier (`planner.would_violate(&op)`) to block or warn before an edit, but the planner is the authority.

### 3. Resolving the mode and conflicts

| Constraint | `Auto` | `Incremental` requested | `FullRewrite` requested |
| --- | --- | --- | --- |
| `Free` | Incremental if there is a base, else FullRewrite | OK | OK |
| `AppendOnly` | Incremental | OK, unless DocMDP/FieldMDP is violated | Error |
| `RequiresRewrite` | FullRewrite | Error | OK |

- **Conflicts are errors by default**: `SaveError::WouldInvalidateSignatures { signatures }` when a rewrite is needed (or requested) on a signed document, or a pending change violates DocMDP/FieldMDP.
- **Explicit override only.** With `SaveOptions::invalidate_signatures: true` (the UI asks the user, naming the signatures), the save goes ahead: signature values (`/V`) and `/Perms` entries are removed so the output doesn't carry signatures that would show as broken, the fields stay unsigned, and the report lists them. Redacting a signed document is exactly this path.
- An encrypted base can't be saved incrementally until encryption on write lands (full-editor E11); until then saving it requires removing encryption, which makes it `RequiresRewrite`.

### 4. Canonical order

**redact → full rewrite → sign → incremental saves.** In one save the planner runs, in order: pending rewrite-requiring changes as a full rewrite; then a pending signature as an incremental revision on top of that output; later saves are append-only. A signature can never be followed by a rewrite without the override, and a redaction can never be left in an appended revision. Signing is two-phase (placeholder `/Contents` and `/ByteRange`, external signer, inject); its details, including how the sink patches the placeholder, are ADR 0020.

### 5. Writers

- **Incremental** (§7.5.6): copies the base bytes from the `ByteSource` in chunks, unchanged, then appends the overlay's new and modified objects, deleted objects as free entries with the generation incremented, and a new xref section of the same kind as the newest one (classic or stream), with `/Prev`. It keeps `/ID[0]`, never renumbers, never shrinks `/Size`, and drops xref-stream-only trailer keys ([ch. 07 §4](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned)). Option `IncrementalOutput::AppendedOnly` writes only the new bytes, so the desktop app can append in place. A save with an empty overlay writes the base bytes unchanged.
- **Full rewrite**: writes only objects reachable from the trailer (`/Root`, `/Info`, `/ID`, and `/Encrypt` when kept), so earlier revisions and unreferenced objects disappear. Object numbers are kept by default (`Renumber::Keep`), which keeps `LaminaId`s ([ADR 0005](0005-document-model.md) §5); `Renumber::Compact` is opt-in. Output is object streams plus an xref stream by default, classic on request ([ch. 01 §14](../research/pdf/01-file-structure.md#14-implementation-guidance-for-lamina-summary)).

### 6. `WriteSink` and determinism

```rust
pub trait WriteSink {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError>;
}
```

Output streams through the sink, never as one buffer, so a Worker with 128 MB or wasm32 with 4 GB can save files larger than memory allows. The planner wraps the sink to count the position. Core implementations: `Vec<u8>` and an adapter over any `std::io::Write`; files, OPFS and HTTP uploads are edge implementations. Output is **byte-deterministic** ([ADR 0001](0001-core-language-and-licence.md)): the same document state and options give the same bytes on every platform. No clocks (dates come from ops), `/ID` derived from a hash of the content written, and a test saves twice and compares.

### 7. Moving the writer

In B3 the current writer moves from `lamina-cos` into `lamina-save` with its golden, determinism and incremental tests unchanged (`tests/golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs`); `lamina-cos` keeps reading and the overlay.

## Consequences

- Signature-breaking and redaction-leaking saves are impossible by default; the only way is an explicit, reported override.
- The editor can show a `SaveConstraint` banner and block edits early, using the same rules as the save.
- Callers can't choose "full rewrite of a signed file" by accident, which some will find strict; that is the point.
- Detecting DocMDP/FieldMDP violations needs every op to classify itself correctly; a misclassified op is a security bug and gets a test per op.
- Keeping object numbers on full rewrite makes outputs slightly larger than compacted ones.

## Alternatives considered

- **Caller-chosen mode only** (PDFium, LibPDF). Leaves every trap to the caller.
- **Always full rewrite.** Breaks every signature and loses revision history.
- **Always incremental.** Leaks redacted content and old encryption in earlier revisions.
- **Silently keep invalid signatures on override.** Leaves documents that viewers show as tampered.
- **Compact renumbering by default.** Smaller files, but IDs and sidecars break on every save.
