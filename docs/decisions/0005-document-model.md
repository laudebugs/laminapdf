# 0005 — Document model: lazy COS over `ByteSource`, dirty overlay, typed views, `LaminaId`s

- Status: Accepted, except §6 (generation strategy), which stays **Proposed** until the `krilla-fit` spike (B1b) reports
- Date: 2026-10-28

## Context

Every mature engine keeps a COS object graph under a typed model. lopdf's eager whole-document model doesn't fit large files in WASM, which has a 4 GB ceiling on Safari (no Memory64), and PDFBox's hand-flagged dirty tracking silently lost changes ([report: editing](../research/architecture/app-and-package-architecture.md#editing-is-an-op-log-over-a-lazy-object-graph-and-signatures-and-redaction-govern-saving)). So dirty tracking must be automatic at the COS mutation API, and objects must load on demand from a source that need not be in memory: mmap on desktop, `Blob` slices or OPFS in a browser, HTTP range requests in a Worker ([ADR 0001](0001-core-language-and-licence.md) consequences).

Today `lamina_cos::Document<'a>` borrows a `&[u8]`, parses objects lazily and caches them in `RefCell<BTreeMap<u32, Rc<Object>>>`, which makes it `!Send` and `!Sync` (`crates/lamina-cos/src/reader/mod.rs`). ADR 0001 left "which types are `Send`/`Sync`" to this ADR. Editing ops ([ADR 0006](0006-op-log-and-undo.md)) and sync need IDs that survive page reorders ([editing notes §2](../research/architecture/notes/editing-architecture.md#2-command-pattern--operation-log-undoredo-transactions-dirty-tracking)).

## Decision

### 1. Layers

`lamina-syntax` (objects, parser) → `lamina-cos` (xref, lazy resolution, overlay) → `lamina-model` (typed views, `LaminaId`s) → `lamina-ops` / `lamina-save` → `lamina` façade. Views read; only ops write; only the save planner serialises.

### 2. `ByteSource`

```rust
pub trait ByteSource: Send + Sync {
    fn len(&self) -> u64;
    /// Fills `buf` from `offset`; returns the bytes read (short only at end of input).
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> Result<usize, SourceError>;
}
```

- Synchronous, as the core has no async runtime. `SourceError::Pending { offset, len }` means "not fetched yet": the call fails with `Error::DataPending { ranges }`, the shell fetches those ranges and retries (pdf.js's missing-data pattern). That keeps HTTP range loading out of the core.
- Core implementations: `&[u8]`, `Vec<u8>`, `Arc<[u8]>`. Edge implementations: mmap (desktop engine process), `FileReaderSync` over a `Blob` and an HTTP-range cache over OPFS (`lamina-wasm`/web worker). None of these live in core crates.
- The parser keeps working on `&[u8]` windows: the COS layer reads a window around an offset (64 KiB, doubled up to the object's extent within `Limits`) and parses inside it. Repair scans in overlapping chunks. `open(&[u8])` stays as the zero-copy path for one-shot tools and tests.

### 3. Base bytes and the dirty overlay

- The original bytes are **immutable**. Every mutation lands in an overlay keyed by object number: `New(Object) | Modified(Object) | Deleted`, plus a journal per transaction for rollback ([ADR 0006](0006-op-log-and-undo.md)). Resolution checks the overlay first, then the base.
- Mutation is only possible through the overlay API (`CosMut`), which ops receive inside a transaction. There is no way to change an object without the overlay knowing, so dirty tracking is automatic.
- An overlay entry whose value equals the base object is dropped, so undoing back to the saved state leaves nothing dirty and a save writes nothing new.
- New objects get fresh numbers above the highest number in any xref section; numbers are never reused within a session, even after a delete. Reserved numbers of undone creations stay reserved so redo recreates the same objects.
- After a successful save the overlay is committed: entries stay in memory as the new current revision, and the dirty set is cleared. A shell that can hand the written bytes back may reopen over them instead.

### 4. Typed views (`lamina-model`)

`Page`, `Outline`/`OutlineItem`, `Info`, `NameTree`, `NumberTree`, `PageLabels` in B2; annotations, fields and the structure tree in later rounds. Views are thin, borrowed wrappers (`Page<'d>` holds `&'d Document` and an object reference) that read fields lazily and apply [ADR 0003](0003-reader-tolerance-and-limits.md)'s tolerance per field: a bad `/MediaBox` is `None` plus a warning, not a failed page. Views never cache copies of the graph. Bindings don't see lifetimes: the façade exposes owned handles and IDs ([ADR 0002](0002-binding-strategy.md) decision 3).

### 5. `LaminaId`s

A `LaminaId` is an opaque, typed identifier for a page, annotation, outline item or field (`PageId`, `AnnotId`, … newtypes over one `LaminaId`). Ops and sync refer to IDs, never to object references.

- **Derivation.** From the object that represents the item: kind plus object number and generation (`p12.0` in the JSON string form, documented as opaque). Annotations with a `/NM` (§12.5.2) unique on their page use it instead (`a:` plus the name). Objects created in the session get numbers at creation (§3), so they derive IDs the same way. A direct (non-indirect) page dictionary, which is non-standard, falls back to its position in the walk.
- **Unique and never reused** within a session; undo restores the same ID.
- **Survive reopen:** yes, because derivation is deterministic: the same bytes with the same lamina version give the same IDs.
- **Survive save, same session:** always. A save that renumbers updates the internal map.
- **Survive save and reopen:** an incremental save never renumbers, so yes. A full rewrite keeps object numbers by default ([ADR 0007](0007-save-planner.md)), so yes; with compaction (`Renumber::Compact`), annotations keep IDs through `/NM`, which lamina writes on every annotation it creates, and pages don't; the `SaveReport` returns the old → new map for sidecars (sync, ADR 0012). lamina doesn't write private keys into files just to persist page IDs.

### 6. Generation strategy — **Proposed, open point**

Decided by the `krilla-fit` spike (`spikes/krilla-fit`, B1b), which generates a 3-page document into a `lamina-cos` graph and measures it.

- **Option A: krilla generates, lamina opens.** `finish()` serialises with krilla and opens the bytes as a `Document`. Gains krilla's text, tagging and PDF/A/UA work; costs a serialise-and-parse round trip, and content can't be added to an existing document this way, which full-editor E6 needs.
- **Option B: `lamina-build` writes into the overlay.** Our own builder over `lamina-cos`, reusing the crates beneath krilla (`subsetter`, `skrifa`, HarfRust). One code path for new documents and for adding content to existing ones; more work up front.

Criteria, in order: can krilla emit objects into a caller-owned object-number space; byte determinism across runs and targets ([ADR 0001](0001-core-language-and-licence.md)); time and output size for the 3-page sample; WASM size delta; coverage of PDF/UA-1 and PDF/A-2/3 needs (B8); licence and maintenance activity. The spike README records the decision and this section is updated to Accepted.

### 7. Ownership and threads

- Cached objects move from `Rc<Object>` to `Arc<Object>`; caches stay in `RefCell`. So **`Document` is `Send` but `!Sync`**: it can move between threads (a Tauri command pool, a `spawn_blocking`), but one thread uses it at a time. The atomic refcount costs little next to parsing, and compiles to plain operations on single-threaded wasm32.
- Parallelism is the shells' job ([ADR 0001](0001-core-language-and-licence.md)): open one `Document` per thread or worker over a shared `Arc<dyn ByteSource>`. Opening is cheap because everything is lazy. Edits stay on one `Document`.
- No `Mutex` or atomics-based caches in the core until a measured case (for example parallel text extraction sharing one overlay) needs a `Sync` document; that would amend this ADR.

## Consequences

- Large files open in bounded memory on every platform, and remote files load by range without an async core.
- PDFBox's lost-changes bug is unrepresentable: the overlay is the only write path.
- `Document` loses its borrowed lifetime and owns its source; zero-copy `&[u8]` reading remains for tools.
- `Rc` → `Arc` and the windowed parser are B2 work in `lamina-cos`, with the existing tests unchanged.
- Page IDs are not persisted in the file; collaboration needs a sidecar map, which ADR 0012 already plans.
- Until §6 is settled, B4 and B5 can't start the builder.

## Alternatives considered

- **Eager parse into an in-memory graph** (lopdf). Simple, but memory grows with file size.
- **Async `ByteSource`.** Natural for HTTP, but it pulls an executor into the core and colours every API. The pending-and-retry pattern is enough.
- **`Rc` everywhere (`!Send`).** Cheapest, but forces every shell into a dedicated thread per document.
- **`Arc` + `Mutex` (`Sync`).** Lets threads share one document at the price of locking on every resolve; no current need.
- **Random UUIDs as IDs.** Not deterministic, and the core has no randomness.
- **Writing page IDs into `/PieceInfo`.** Persists IDs but changes files the user only viewed and leaks editor state.
