# 0005 — Document model: lazy COS over `ByteSource`, dirty overlay, typed views, `LaminaId`s

- Status: Accepted. §6 (generation strategy) is recommended by the [`krilla-fit` spike](../../spikes/krilla-fit/README.md) and accepted when the spike's PR merges (owner sign-off)
- Date: 2026-10-28

## Context

Every mature engine keeps a COS object graph under a typed model. lopdf's eager whole-document model doesn't fit large files in WASM, which has a 4 GB ceiling on Safari (no Memory64), and PDFBox's hand-flagged dirty tracking silently lost changes ([report: editing](../research/architecture/app-and-package-architecture.md#editing-is-an-op-log-over-a-lazy-object-graph-and-signatures-and-redaction-govern-saving)). So dirty tracking must be automatic at the COS mutation API, and objects must load on demand from a source that need not be in memory: mmap on desktop, `Blob` slices or OPFS in a browser, HTTP range requests in a Worker ([ADR 0001](0001-core-language-and-licence.md) consequences).

Today `lamina_cos::Document<'a>` borrows a `&[u8]`, parses objects lazily and caches them in `RefCell<BTreeMap<u32, Rc<Object>>>`, which makes it `!Send` and `!Sync` (`crates/lamina-cos/src/reader/mod.rs`). ADR 0001 left "which types are `Send`/`Sync`" to this ADR. Editing ops ([ADR 0006](0006-op-log-and-undo.md)) and sync need IDs that survive page reorders ([editing notes §2](../research/architecture/notes/editing-architecture.md#2-command-pattern--operation-log-undoredo-transactions-dirty-tracking)).

## Decision

### 1. Layers

`lamina-syntax` (objects, parser) → `lamina-cos` (xref, lazy resolution, overlay) → `lamina-model` (typed views, `LaminaId`s) → `lamina-ops` / `lamina-save` → `lamina` façade. Views read; only ops write; only the save planner serialises.

### 2. `ByteSource` and ownership of the input

```rust
pub trait ByteSource: Send + Sync {
    fn len(&self) -> u64;
    fn is_empty(&self) -> bool { self.len() == 0 }
    /// Fills `buf` from `offset`; returns the bytes read (short only at end of input).
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError>;
    /// The whole input, when it is already in memory (lets the parser skip window copies).
    fn as_slice(&self) -> Option<&[u8]> { None }
}
```

- **`Document` owns its input** as an `Arc<dyn ByteSource>` and has no lifetime parameter. `Document::open(source: impl Into<Arc<dyn ByteSource>>, options)`. Zero-copy comes from shared ownership, not borrowing: core implementations are `Vec<u8>` (moved in), `Arc<[u8]>` and `&'static [u8]`, and all of them return `Some` from `as_slice`, so in-memory input is parsed in place as today. The borrowed `lamina_cos::Document<'a>` and `open(&[u8])` are replaced in B2; tests pass owned or shared buffers.
- **Synchronous**, as the core has no async runtime. `SourceError::Pending { offset, len }` means "not fetched yet": the call fails with `Error::DataPending { ranges }`, the shell fetches those ranges and retries (pdf.js's missing-data pattern). That keeps HTTP range loading out of the core.
- **Edge implementations:** mmap (desktop engine process), `FileReaderSync` over a `Blob`, and an HTTP-range cache over OPFS (`lamina-wasm`/web worker). None of these live in core crates.
- For sources without `as_slice`, the COS layer reads a window around an offset (64 KiB, doubled up to the object's extent within `Limits`) and the parser works on that `&[u8]`. Repair scans in overlapping chunks.

### 3. Base, overlay and the save cycle

- The **base** is the current revision's bytes, and it is immutable. Every mutation lands in an **overlay** keyed by object number: `New(Object) | Modified(Object) | Deleted`, plus a journal per transaction for rollback ([ADR 0006](0006-op-log-and-undo.md)). Resolution checks the overlay first, then the base.
- Mutation is only possible through the overlay API (`CosMut`), which ops receive inside a transaction. There is no way to change an object without the overlay knowing, so dirty tracking is automatic.
- An overlay entry whose value equals the base object is dropped, so undoing back to the base leaves nothing dirty and a save writes nothing new.
- New objects get fresh numbers above the highest number in any xref section; numbers are never reused within a session, even after a delete. Numbers of undone creations stay reserved so redo recreates the same objects.

**After a successful save the document rebases onto what it wrote.** The states are:

| State | Meaning | Transitions |
| --- | --- | --- |
| `Clean` | Overlay empty; base = last opened or saved bytes | an op → `Dirty` |
| `Dirty` | Overlay non-empty | undo to base → `Clean`; save succeeds → rebase → `Clean`; save fails → stays `Dirty`, overlay untouched |
| `SavedNotRebased` | Written to a sink the core can't read back, and not rebased yet | `Document::rebase(source)` → `Clean` |

Rebasing (1) makes the saved output the new base: for an incremental save the core builds it itself as the old base followed by the appended revision, which it keeps in memory; for a full rewrite the planner gets it back from a readable sink ([ADR 0007](0007-save-planner.md) §6), or the shell passes the saved file to `rebase`; (2) loads the xref of the new newest revision; (3) clears the overlay and the dirty set; (4) keeps the `LaminaId` map, applying the renumbering map if the save compacted; and (5) re-detects the `SaveConstraint` from the new base. So open → sign → save → edit → save appends to the signed revision, and a signature made in this session makes the document `AppendOnly`. In `SavedNotRebased`, reads work and ops are refused, and so is another save, so nothing is ever written against a stale base.

### 4. Typed views (`lamina-model`)

`Page`, `Outline`/`OutlineItem`, `Info`, `NameTree`, `NumberTree`, `PageLabels` in B2; annotations, fields and the structure tree in later rounds. Views are thin, borrowed wrappers (`Page<'d>` holds `&'d Document` and an object reference) that read fields lazily and apply [ADR 0003](0003-reader-tolerance-and-limits.md)'s tolerance per field: a bad `/MediaBox` is `None` plus a warning, not a failed page. Views never cache copies of the graph. Bindings don't see lifetimes: the façade exposes owned handles and IDs ([ADR 0002](0002-binding-strategy.md) decision 3).

### 5. `LaminaId`s

A `LaminaId` is an opaque, typed identifier for a page, annotation, outline item or field (`PageId`, `AnnotId`, … newtypes over one `LaminaId`). Ops and sync refer to IDs, never to object references.

- **Object identity.** An item's ID is its kind plus the number and generation of the indirect object that represents it (`p12.0` in the JSON string form, documented as opaque). Object numbers are unique across the document, so IDs are unique document-wide; nothing depends on siblings, page position or walk order.
- **Allocation.** One document-level allocator assigns IDs: existing objects get theirs on first sight, and new objects get theirs when the overlay allocates their number (§3). Numbers, and therefore IDs, are never reused in a session; undoing a delete brings the same ID back.
- **Direct dictionaries** (a page or annotation written inline, which is non-standard) have no object identity. They get a path ID relative to their nearest indirect parent (`p5.0/Kids/3`), stable for the same bytes. The first op that touches one promotes it to an indirect object, and the session keeps the old ID as an alias of the new object.
- **Page ops keep page objects**: moving, rotating or relabelling a page changes the page tree and the page dictionary but never replaces the page object, so `MovePages` keeps every `PageId`. Inserting pages from another document creates new objects and new IDs.
- **`/NM` is a hint, never the ID.** It is not unique in practice and is attacker-controlled. The model keeps a document-wide index from `/NM` to IDs for XFDF and sync matching; duplicates are reported as a warning and the lookup returns all matches. Annotations lamina creates get an `/NM` derived from their ID, which is unique document-wide.
- **Across saves and reopen:** an incremental save never renumbers, and a full rewrite keeps object numbers by default ([ADR 0007](0007-save-planner.md)), so IDs survive save and reopen. With `Renumber::Compact`, the session applies the old → new map, and the `SaveReport` returns it for sidecars (sync, ADR 0012); after reopen, the compacted IDs are the new ones. lamina doesn't write private keys into files just to persist IDs.

### 6. Generation strategy — Option B (recommended by the `krilla-fit` spike; accepted when its PR merges)

The `krilla-fit` spike ([`spikes/krilla-fit`](../../spikes/krilla-fit/README.md), B1b) generated the same 3-page document (subset font, PNG with alpha, JPEG, paths, link, outline) with krilla and with a minimal builder over `lamina-cos`, and measured both.

- **Option A: krilla generates, lamina opens.** `finish()` serialises with krilla and opens the bytes as a `Document`. Gains krilla's text, tagging and PDF/A/UA work; costs a serialise-and-parse round trip, and content can't be added to an existing document this way, which full-editor E6 needs.
- **Option B: `lamina-build` writes into the overlay.** Our own builder over `lamina-cos`, reusing the crates beneath krilla (`subsetter`, `skrifa`, HarfRust). One code path for new documents and for adding content to existing ones; more work up front.

**Decision: Option B.** Against the criteria, in order:

1. **krilla can't emit into a caller-owned object-number space.** `Document::finish()` returns `Vec<u8>` only. Refs are allocated in a private `SerializeContext` from 1, and `ChunkContainer::finish` renumbers everything and writes its own catalog, XMP and trailer (all `pub(crate)` in krilla 0.8.2). Importing krilla's pages into an existing document works (0.11 ms for 3 pages), but outline, `Lang`, page labels and names need merge code, and the imported pages lose their tagging unless a structure-tree (`ParentTree`) merge is written.
2. **Determinism:** both are byte-identical across runs.
3. **Time and size** are equivalent: 3.98 ms / 104 497 B (krilla) against 3.51 ms / 102 624 B (lamina builder), release, best of 30.
4. **WASM (gzip, no wasm-opt):** 830 KB with krilla against 377 KB for the lamina builder. The budget is 350 KB *including* shaping, so **both are over it**, the lamina builder before a shaper is even added. Own crates can be split into a base module and a lazily loaded text module (skrifa, subsetter, HarfRust); krilla can't. B6 (ADR 0016) must decide the split, re-measure with wasm-opt, and revise the budget if needed.
5. **PDF/UA and PDF/A:** krilla's validators check the file krilla writes; after an import into a lamina `Document`, conformance has to be checked by lamina anyway. B8 builds lamina-side profiles, with krilla's rule lists as the reference.
6. **Licence:** both are clean under `deny.toml`. krilla is active but mostly has one maintainer, and it pins the font stack (`skrifa` 0.42 while 0.49 is current).

How it fits this ADR: the builder allocates objects in the overlay (§3) as `New` entries with fresh numbers. Objects that are known only at `finish()` (font programs after subsetting, the outline, the structure tree) are reserved at first use and filled in at the end; the spike does this with `Writer::reserve`/`set`. A new document is a `Document` whose base is empty; adding content to an open document uses the same path and saves incrementally. Content streams use `pdf_writer::Content` (bytes, no refs); dictionaries are lamina `Object`s. krilla is a reference implementation (code ported with attribution) and a test-only oracle for differential tests, not a dependency of core crates.

### 7. Threads

- Cached objects move from `Rc<Object>` to `Arc<Object>`; caches stay in `RefCell`. So **`Document` is `Send` but `!Sync`**: it can move between threads (a Tauri command pool, a `spawn_blocking`), but one thread uses it at a time. The atomic refcount costs little next to parsing, and compiles to plain operations on single-threaded wasm32.
- Parallelism is the shells' job ([ADR 0001](0001-core-language-and-licence.md)): open one `Document` per thread or worker over the same `Arc<dyn ByteSource>`. Opening is cheap because everything is lazy. Edits stay on one `Document`.
- No `Mutex` or atomics-based caches in the core until a measured case (for example parallel text extraction sharing one overlay) needs a `Sync` document; that would amend this ADR.

## Consequences

- Large files open in bounded memory on every platform, and remote files load by range without an async core.
- PDFBox's lost-changes bug is unrepresentable: the overlay is the only write path.
- Rebasing after every save means constraints, IDs and the next save always refer to what is actually on disk.
- `Document` owns its input; the borrowed `Document<'a>`, `Rc` → `Arc` and windowed parsing are B2 work in `lamina-cos`, with the existing tests kept.
- Page IDs are not persisted in the file; collaboration needs a sidecar map, which ADR 0012 already plans.
- The builder is ours (§6): B4 (`lamina-text`) and B5 (`lamina-build`) build over the overlay with `pdf-writer`, `subsetter`, `skrifa` and HarfRust, and own tagging and conformance in B8.

## Alternatives considered

- **Eager parse into an in-memory graph** (lopdf). Simple, but memory grows with file size.
- **Async `ByteSource`.** Natural for HTTP, but it pulls an executor into the core and colours every API. The pending-and-retry pattern is enough.
- **Keeping a borrowed `Document<'a>`.** Zero-copy, but bindings and shells can't hold it, and it can't own an mmap or a range cache.
- **`Rc` everywhere (`!Send`).** Cheapest, but forces every shell into a dedicated thread per document.
- **`Arc` + `Mutex` (`Sync`).** Lets threads share one document at the price of locking on every resolve; no current need.
- **IDs from `/NM` or walk position.** Not unique, or changed by unrelated edits.
- **Keeping the original base after a save** and holding committed changes in memory. The next incremental save would rebuild from the old file, and in-session signatures would go undetected.
- **Writing page IDs into `/PieceInfo`.** Persists IDs but changes files the user only viewed and leaks editor state.
