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
plan.constraint();          // &SaveConstraint: everything that restricts this save
plan.mode();                // the resolved mode: Incremental or FullRewrite
plan.would_invalidate();    // the signatures this save would break (empty unless overridden)
let report = doc.write(plan, &mut sink)?;   // SaveReport; the document then rebases (ADR 0005 §3)
```

`Document::save(sink, options)` is the shorthand. Planning is separate from writing so the UI can show every restriction and every signature at risk before anything is written.

- `SaveMode::Incremental | FullRewrite | Auto` (default `Auto`).
- `SaveReport`: bytes written, the resolved mode, signatures invalidated (if any), the old → new object-number map when renumbered, and the offset where appended bytes start.

### 2. `SaveConstraint`: all the conditions, not one of them

The conditions overlap (a signed document can also have a pending redaction), so the constraint is a struct and the planner reports all of them:

```rust
#[non_exhaustive]
pub struct SaveConstraint {
    pub signatures: Vec<SignatureInfo>,        // field, kind (approval, certification, doc timestamp, UR3), revision
    pub doc_mdp: Option<DocMdp>,               // P1 | P2 | P3 of the certification signature
    pub rewrite_reasons: Vec<RewriteReason>,   // why only a full rewrite is safe
    pub violations: Vec<Violation>,            // pending changes DocMDP, FieldMDP or UR3 forbid
}
```

Detection, from the base plus the overlay ([ADR 0005](0005-document-model.md)):

- **Signatures**: a signature field (`/FT /Sig`) whose `/V` is a signature dictionary, a document timestamp (`/Type /DocTimeStamp`), or `/Perms` with `/DocMDP` or `/UR3`. Because the document rebases after every save, a signature made in this session counts from then on.
- **Rewrite reasons**: an applied redaction (`ApplyRedactions`) or another op that reports `requires_rewrite` ([ADR 0006](0006-op-log-and-undo.md)), such as sanitising or optimising; encryption being removed or changed (old revisions would keep the old encryption or the plaintext); a repaired base ([ADR 0003](0003-reader-tolerance-and-limits.md)), since a revision appended to an unusable xref would be unusable too (today's `RepairedBase`); an encrypted base while encryption on write is missing (full-editor E11; today's `EncryptedBase`), which can only be saved by removing encryption.
- **Violations**: each pending transaction's `ModificationClass` checked against DocMDP `/P` (1 allows nothing; 2 allows form filling, page templates and signing; 3 adds annotations), FieldMDP and `/Lock` dictionaries (changes to locked fields), and UR3 (changes outside the granted rights).

`constraint.kind()` summarises it for the UI banner: **`Free`** (no signatures, no rewrite reasons), **`AppendOnly`** (signatures, nothing that conflicts), **`RequiresRewrite`** (rewrite reasons, no signatures), or **`Conflict`** (signatures plus rewrite reasons or violations). The editor can ask before an edit (`planner.would_violate(&op)`) to block or warn early, but the planner is the authority.

### 3. Resolving the mode

| `kind()` | `Auto` | `Incremental` requested | `FullRewrite` requested |
| --- | --- | --- | --- |
| `Free` | Incremental if there is a base, else FullRewrite | OK | OK |
| `AppendOnly` | Incremental | OK | Error, unless overridden |
| `RequiresRewrite` | FullRewrite | Error | OK |
| `Conflict` | Error, unless overridden | Error, unless overridden (and only without rewrite reasons) | Error, unless overridden |

- Errors are `SaveError::WouldInvalidateSignatures { signatures, rewrite_reasons, violations }` (the whole constraint, so the UI can name every signature) or `SaveError::RequiresRewrite { reasons }`.
- **Explicit override only.** With `SaveOptions::invalidate_signatures: true` (the UI asks the user, naming the signatures from `would_invalidate()`), the save goes ahead: with rewrite reasons it is a full rewrite, otherwise incremental. The invalidated signatures' values (`/V`) and the matching `/Perms` entries are removed, so the output doesn't carry signatures that viewers would show as broken; the fields stay, unsigned, and the report lists them. Redacting a signed document is this path.

### 4. Canonical order

**redact → full rewrite → sign → incremental saves.** In one save the planner runs, in order: pending rewrite-requiring changes as a full rewrite; then a pending signature as an incremental revision on top of that output; later saves are append-only. A signature is never followed by a rewrite without the override, and a redaction is never left in an appended revision.

**Signing is two-phase and needs no seeking.** The signing revision (changed objects, the signature dictionary with a fixed-size `/Contents` placeholder and `/ByteRange`) is assembled in memory, where its offsets are known. The digest is computed by streaming the base bytes from the `ByteSource` and the in-memory revision, skipping the placeholder. The external signer (WebCrypto, an HSM, a server) returns the CMS, which is injected into the in-memory revision, and only then is anything written to the sink. When a rewrite precedes signing in the same save, the rewrite goes to a readable sink (§6), which becomes the base the signing phase reads. Certificate handling, timestamps and PAdES levels are ADR 0020.

### 5. Writers

- **Incremental** (§7.5.6): copies the base bytes from the `ByteSource` in chunks, unchanged, then appends the overlay's new and modified objects, deleted objects as free entries with the generation incremented, and a new xref section of the same kind as the newest one (classic or stream), with `/Prev`. It never renumbers, never shrinks `/Size`, and drops xref-stream-only trailer keys ([ch. 07 §4](../research/pdf/07-spike-rust-and-typescript.md#4-what-we-learned)). Option `IncrementalOutput::AppendedOnly` writes only the new bytes, so the desktop app can append in place. A save with an empty overlay writes the base bytes unchanged.
- **Full rewrite**: writes only objects reachable from the trailer (`/Root`, `/Info`, `/ID`, and `/Encrypt` when kept), so earlier revisions and unreferenced objects disappear. Object numbers are kept by default (`Renumber::Keep`), which keeps `LaminaId`s ([ADR 0005](0005-document-model.md) §5); `Renumber::Compact` is opt-in. Output is object streams plus an xref stream by default, classic on request ([ch. 01 §14](../research/pdf/01-file-structure.md#14-implementation-guidance-for-lamina-summary)).
- **`/ID`** (§14.4): `/ID[0]` is the document's permanent identifier and, for the standard security handler R2–R4, an input to the file key (§7.6.3.3). Every save, incremental or full, keeps it. Only `/ID[1]` changes, derived from a hash of the content written. `/ID[0]` is derived the same way only for a new document or when the original has no `/ID`.

### 6. Sinks and determinism

```rust
pub trait WriteSink {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError>;
}
pub trait ReadableSink: WriteSink {
    /// What was written, as a source (for rebasing and for signing on top of a rewrite).
    fn into_source(self) -> Result<Arc<dyn ByteSource>, SinkError>;
}
```

Output streams through the sink, never as one buffer, so a Worker with 128 MB or wasm32 with 4 GB can save files larger than memory allows. The planner wraps the sink to count the position. Sinks are write-only; nothing ever seeks or patches (§4). Core implementations: `Vec<u8>` (readable) and an adapter over any `std::io::Write`; spooling to OPFS or a temp file, and files and uploads, are edge implementations. A full rewrite into a sink that isn't readable leaves the document in `SavedNotRebased` until the shell calls `rebase` ([ADR 0005](0005-document-model.md) §3); an incremental save always rebases by itself.

Output is **byte-deterministic** ([ADR 0001](0001-core-language-and-licence.md)): the same document state and options give the same bytes on every platform. No clocks (dates come from ops), `/ID[1]` from a content hash, and a test saves twice and compares.

### 7. Moving the writer

In B3 the current writer moves from `lamina-cos` into `lamina-save` with its golden, determinism and incremental tests unchanged (`tests/golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs`); `lamina-cos` keeps reading and the overlay.

### 8. Implementation notes (B3b)

How [B3b](../todos/done/B3b-lamina-save-writers.md) realised §5–§7, where the sketch above left a choice (the planner, §1–§4, is [B3c](../todos/ready/B3c-save-planner-and-roundtrip.md)'s):

- **Two layers.** `IncrementalWriter` and `FullWriter` work on a `lamina_cos::Document` and a `WriteSink`; the `Save` trait on `lamina_ops::Document` (`save_incremental`, `save_full`, `save_full_and_rebase`, temporary names until `Document::save`) adds the state machine and the rebase. `lamina-ops` owns the state (`SaveState`, `saved_not_rebased(PendingRebase)`, `rebase`), since it is what refuses ops in `SavedNotRebased`; `rebase` needs a pending save. `lamina_cos::Document::rebase(source, Reserved)` is the mechanism (reopen with the same limits, drop the overlay and journal, keep the warnings and the journal's sequence numbers, keep or release the reserved numbers).
- **Undo across a save.** The history survives only an incremental save that freed nothing: every object it names still exists under its number and generation. A full rewrite drops unreachable objects and an incremental delete bumps generations, so undo could restore references to objects that are gone; those saves clear the history and release reserved numbers.
- **`/ID[1]`** is the first 16 bytes of SHA-256: for an incremental save over the base and the revision up to its trailer (so `AppendedOnly` and whole output agree), for a full rewrite over everything before the trailer (including the xref rows). `/ID[0]`, when missing, is the same value.
- **`/Size` and the xref.** An incremental `/Size` is never below `Document::next_number()`, so a number reserved in the session is still free after the save and redo can recreate it. A full rewrite lists only numbers that hold something (objects, and in `Keep` the base objects it frees, with the next generation), so a hostile input `/Size` costs nothing. Both check their numbers against `Limits::max_object_number` before writing; free entries are linked into the free list, in front of the base's for an incremental update.
- **Before writing.** Both writers do everything that can fail on input first: the incremental one reads every base chunk of a windowed source (so `DataPending` lists all missing ranges and the sink stays empty), the full one loads every reachable object (one that can't be loaded fails the save) and allocates every number. Sink errors, a source failing between the check and the copy, and a classic offset past ten digits can leave a partial output; a rebase failing after a complete write returns `SaveError::NotRebased` with the output. A sink that already holds bytes is refused (`held()`), since offsets count from the output's start.
- **Full rewrite details.** The walk starts at the trailer's document keys except `/ID` (rewritten) and skips a stream's own `/Length` (written directly); dangling and stale references are written as `null` (§7.3.10); object streams hold up to 100 members in number order and never streams or objects with a generation above 0.
- **Moved tests** kept their bodies; only imports changed. `lamina-cos`, `lamina-model` and `lamina-ops` reach `Writer` through a dev-dependency on `lamina-save`; `lamina-cos`'s unit tests build files with `tests/common/classic.rs` instead (through the dev-dependency they would see a second copy of the crate), which `lamina-save`'s tests share by `#[path]` with the content builder.

## Consequences

- Signature-breaking and redaction-leaking saves are impossible by default; the only way is an explicit, reported override that names every signature it breaks.
- The editor can show a constraint banner and block edits early, using the same rules as the save.
- Callers can't choose "full rewrite of a signed file" by accident, which some will find strict; that is the point.
- Detecting DocMDP/FieldMDP violations needs every op to classify itself correctly; a misclassified op is a security bug and gets a test per op.
- Signing with an external signer holds the signing revision in memory between the two phases; it is small.
- Keeping object numbers on full rewrite makes outputs slightly larger than compacted ones.

## Alternatives considered

- **Caller-chosen mode only** (PDFium, LibPDF). Leaves every trap to the caller.
- **A single-valued `SaveConstraint` enum.** Overlapping conditions hide each other; a signed document with a redaction would report only one.
- **Always full rewrite.** Breaks every signature and loses revision history.
- **Always incremental.** Leaks redacted content and old encryption in earlier revisions.
- **Patching the `/Contents` placeholder in a seekable sink.** Rules out streaming sinks (uploads, Workers); building the small signing revision in memory avoids it.
- **A new `/ID[0]` on full rewrite.** Breaks the permanent identifier and, for older encryption, the key.
- **Silently keep invalid signatures on override.** Leaves documents that viewers show as tampered.
- **Compact renumbering by default.** Smaller files, but IDs and sidecars break on every save.
