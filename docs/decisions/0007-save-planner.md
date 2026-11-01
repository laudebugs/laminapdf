# 0007 — Save planner and `SaveConstraint`

- Status: Accepted
- Date: 2026-10-28

## Context

How a PDF is saved decides whether signatures stay valid and whether "deleted" content is really gone. An incremental update appends a revision and leaves every earlier byte in the file (ISO 32000-1 §7.5.6). A signature covers a `/ByteRange` of the file as it was when signed, so a full rewrite that moves bytes invalidates it (§12.8; [research ch. 04 §8.7](../research/pdf/04-interactive-security.md#87-incremental-updates-after-signing), [§9](../research/pdf/04-interactive-security.md#9-signing-via-incremental-update-practical-constraints)). After certification, DocMDP (§12.8.2.2) limits which incremental changes are allowed; usage-rights signatures (§12.8.2.3) and FieldMDP (§12.8.2.4) add more. Applying redaction annotations (§12.5.6.23) is only real if no earlier revision survives, which takes a full rewrite ([ch. 04 §10.2](../research/pdf/04-interactive-security.md#102-what-true-redaction-must-do)).

Other engines leave this to callers: PDFium exposes two mutually exclusive flags, LibPDF can't detect when incremental saves are required ([report: editing](../research/architecture/app-and-package-architecture.md#editing-is-an-op-log-over-a-lazy-object-graph-and-signatures-and-redaction-govern-saving)). Principle 3 of the [architecture plan](../plans/architecture.md#principles-each-becomes-part-of-an-adr) makes the planner own this. Today the writer (`Writer`, `incremental_update`, `XrefEol`) lives in `crates/laminapdf-cos/src/writer.rs` and already refuses repaired and encrypted bases.

## Decision

### 1. API (`laminapdf-save`)

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
- **Full rewrite**: writes only objects reachable from the trailer (`/Root`, `/Info`, `/ID`, and `/Encrypt` when kept), so earlier revisions and unreferenced objects disappear. Object numbers are kept by default (`Renumber::Keep`), which keeps `LaminaPdfId`s ([ADR 0005](0005-document-model.md) §5); `Renumber::Compact` is opt-in. Output is object streams plus an xref stream by default, classic on request ([ch. 01 §14](../research/pdf/01-file-structure.md#14-implementation-guidance-for-lamina-summary)).
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

In B3 the current writer moves from `laminapdf-cos` into `laminapdf-save` with its golden, determinism and incremental tests unchanged (`tests/golden.rs`, `determinism.rs`, `incremental.rs`, `raw_writer.rs`); `laminapdf-cos` keeps reading and the overlay.

### 8. Implementation notes (B3b)

How [B3b](../todos/done/B3b-lamina-save-writers.md) realised §5–§7, where the sketch above left a choice (the planner, §1–§4, is [B3c](../todos/done/B3c-save-planner-and-roundtrip.md)'s):

- **Two layers.** `IncrementalWriter` and `FullWriter` work on a `laminapdf_cos::Document` and a `WriteSink`; the `Save` trait on `laminapdf_ops::Document` (`save_incremental`, `save_full`, `save_full_and_rebase`, temporary names until `Document::save`) adds the state machine and the rebase. `laminapdf-ops` owns the state (`SaveState`, `saved_not_rebased(PendingRebase)`, `rebase`), since it is what refuses ops in `SavedNotRebased`; `rebase` needs a pending save. `laminapdf_cos::Document::rebase(source, Reserved)` is the mechanism (reopen with the same limits, drop the overlay and journal, keep the warnings and the journal's sequence numbers, keep or release the reserved numbers).
- **Undo across a save.** The history survives only an incremental save that freed nothing: every object it names still exists under its number and generation. A full rewrite drops unreachable objects and an incremental delete bumps generations, so undo could restore references to objects that are gone; those saves clear the history and release reserved numbers.
- **`/ID[1]`** is the first 16 bytes of SHA-256: for an incremental save over the base and the revision up to its trailer (so `AppendedOnly` and whole output agree), for a full rewrite over everything before the trailer (including the xref rows). `/ID[0]`, when missing, is the same value.
- **`/Size` and the xref.** An incremental `/Size` is never below `Document::next_number()`, so a number reserved in the session is still free after the save and redo can recreate it. A full rewrite lists only numbers that hold something (objects, and in `Keep` the base objects it frees, with the next generation), so a hostile input `/Size` costs nothing. Both check their numbers against `Limits::max_object_number` before writing; free entries are linked into the free list, in front of the base's for an incremental update.
- **Before writing.** Both writers do everything that can fail on input first: the incremental one reads every base chunk of a windowed source (so `DataPending` lists all missing ranges and the sink stays empty), the full one loads every reachable object (one that can't be loaded fails the save) and allocates every number. Sink errors, a source failing between the check and the copy, and a classic offset past ten digits can leave a partial output; a rebase failing after a complete write returns `SaveError::NotRebased` with the output. A sink that already holds bytes is refused (`held()`), since offsets count from the output's start.
- **Full rewrite details.** The walk starts at the trailer's document keys except `/ID` (rewritten) and skips a stream's own `/Length` (written directly); dangling and stale references are written as `null` (§7.3.10); object streams hold up to 100 members in number order and never streams or objects with a generation above 0.
- **Moved tests** kept their bodies; only imports changed. `laminapdf-cos`, `laminapdf-model` and `laminapdf-ops` reach `Writer` through a dev-dependency on `laminapdf-save`; `laminapdf-cos`'s unit tests build files with `tests/common/classic.rs` instead (through the dev-dependency they would see a second copy of the crate), which `laminapdf-save`'s tests share by `#[path]` with the content builder.

### 9. Implementation notes (B3c)

How [B3c](../todos/done/B3c-save-planner-and-roundtrip.md) realised §1–§4:

- **API.** `laminapdf_ops::Document` lives below `laminapdf-save`, so `Document::save` and `Document::plan` are methods of the `Save` trait: `plan(&options)`, `save(&mut sink, &options)` (a full rewrite into a write-only sink leaves the document `SavedNotRebased`, as before) and `save_and_rebase(sink, &options)` for a `ReadableSink`. B3b's `save_incremental`, `save_full` and `save_full_and_rebase` are gone. There is no `doc.write(plan, sink)`: `save` plans again, so a plan can never be stale. `SavePlanner::new(&doc)` detects once (refused while a transaction is open, whose changes aren't classified yet); `constraint()`, `plan(&options)`, `would_violate(&op)` and `would_violate_class(class)` read the result. `SavePlan::resolve(constraint, &options)` is the table on its own, for any constraint.
- **Detection.** Signature fields come from the `/AcroForm /Fields` tree (`/Kids` followed, `/FT` inherited, cycles bounded) where `/V` is a dictionary, direct fields included; `/Perms /DocMDP` and `/UR3` (or `/UR`) add their signatures, matched to field values by the stored value (a reference, or an equal direct dictionary); each signature dictionary's `/Type /DocTimeStamp` and `/Reference` transforms (`/DocMDP` with `/P`; `/FieldMDP`; `/UR3` with its rights) decide its kind and restrictions; a signed field's `/Lock` is a FieldMDP restriction, and its PDF 2.0 `/P` (Table 236) a DocMDP one. Nothing is assumed absent: a branch of the form that can't be followed (deeper than `Limits::max_tree_depth`, or not an array or dictionary where one belongs) sets `SaveConstraint::signatures_unknown`, which counts as signed (a full rewrite is refused); a certification without a readable DocMDP permission (no transform, unreadable parameters, a `/P` other than 1–3) is taken as `/P 1`, with `DocMdp::assumed` set; an absent `/P` is 2 (Table 257). Only a repaired base is also scanned object by object for signature dictionaries (`/Type /Sig` or `/DocTimeStamp`, or, `/Type` being optional, `/ByteRange` with a string `/Contents`; an object that doesn't parse is skipped; unavailable input still fails), since repair can pick a catalog that no longer reaches a signature. `SignatureInfo` also carries the signature dictionary's reference and the end of its `/ByteRange`; `revision` counts the xref sections of the `/Prev` chain (`laminapdf_cos::Document::xref_section_offsets`) that start before that end, a linearized file's first-page section joining the main one, and is `None` (never 0) when none does or the base was repaired.
- **Violations.** `laminapdf_ops::Document::pending_changes()` holds the op classes (and `requires_rewrite`) of every transaction committed since the last rebase, remote and system ones too; undo and redo add the classes of the entry they revert, and an empty overlay clears them; a model document wrapped while dirty has unknown changes (every class, rewrite needed). They err on the side of reporting. DocMDP: `/P 1` allows nothing, 2 form filling and signing, 3 also annotations. FieldMDP: op classes don't name fields yet (form ops arrive with the editor rounds), so any form change counts against every lock that may cover a field (all but `/Action /Include` with no `/Fields`); field-level checks replace this when ops name fields. UR3 (Table 260): `/Form [/FillIn]` covers form filling, `/Annots` with `/Create`, `/Delete` or `/Modify` annotations, `/Signature [/Modify]` signing; everything else violates. `would_violate(&op)` also reports every signature (`Restriction::SignedBytes`) for an op that needs a full rewrite.
- **Mode table.** Exactly §3, with three additions before it: an unchanged document (`SaveConstraint::unchanged`) saves incrementally in `Auto` and `Incremental`, writing the base bytes unchanged (which breaks nothing and keeps encryption), unless encryption is being removed; a document without base bytes has `RewriteReason::NoBase` (`Auto` is a full rewrite, `Incremental` is refused up front); and the encrypted-base refusal below. An incremental save requested on a `Conflict` with the override is still `RequiresRewrite` when there are rewrite reasons. The override invalidates every signature in a full rewrite; in an incremental save, the signatures whose restriction a pending change violates and then, since removing a signature's value is a change no certification, usage-rights signature or lock permits, every signature that restricts changes.
- **Encrypted base.** Without encryption on write (E11), `EncryptedBase` is a rewrite reason, and every mode that would write something is refused with `SaveError::EncryptedBase` (code `encrypted-base`, the same as the COS refusal) unless `SaveOptions::with_remove_encryption(true)`; only then does §3's `RequiresRewrite` row apply (`Auto` and `FullRewrite` write a full rewrite, `Incremental` is `RequiresRewrite`). The check comes before the table, so a signed and encrypted document needs both overrides. The plaintext rewrite writes the objects as the reader decrypted them, drops a leading `/Crypt` stage (and its `/DecodeParms` entry; an indirect `/Filter` or `/DecodeParms` is resolved first, with `laminapdf_cos::strip_crypt_filter`) from streams and `/Encrypt` from the trailer, and keeps `/ID[0]`; `SaveReport::encryption_removed` says so. `keep_encryption` arrives with E11b.
- **Override.** The planner computes the changes that remove `/V` from each invalidated signature's field and the `/Perms` entries pointing at an invalidated signature (`/Perms` itself when empty), direct fields and a direct `/Perms` edited in the object that contains them, and the writers write them over the document's values. The document itself changes only through the rebase onto the output: there is no edit outside the ops to undo when a save fails, and no public way around the ops.
- **Round trip.** `cargo xtask roundtrip` (full lane): laminapdf-save's `roundtrip` example saves 22 fixtures × 2 proving ops × 3 modes × 4 override combinations, checks each outcome against the fixture's category and reopens each output in lamina (removing encryption must leave no `/Encrypt`); qpdf `--check` (12.4.2, checked), pdf.js and PDFium then check page count, rotations, `/Title` and that the output is unencrypted where lamina says so; every tool's version is recorded, failing or not. PDFium reads no `/Title` from a direct `/Info` dictionary, so lamina flags those outputs. `fixtures/corpus/tampered.pdf` opens without repair (its changed byte is inside the signed range), so the signed and repaired case is that file with its newest catalog damaged as well.
- **Fuzzing.** `ops_json` (an op log, or a single op, applied to two corpus files, saved (which must succeed) and reopened, then undone op by op back to the unchanged base) and `save_roundtrip` (eight control bytes choose ops and save options before a PDF; every saved file must reopen), with their entry points in `crates/laminapdf-save/tests/common/fuzz_entry.rs`; both are in the nightly `fuzz.yml` matrix.

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
