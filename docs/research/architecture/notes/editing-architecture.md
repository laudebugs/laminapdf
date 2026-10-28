# Editing & Collaboration Architecture for a Rust/WASM PDF Editor (as of 2026-10-28)

Scope: how to model, edit, undo, save and collaboratively sync PDF documents in "lamina" (Rust core → WASM for web, native for desktop). Research was done with web search and fetch on 2026-10-28. Statements marked **[prior knowledge, unverified]** come from the researcher's background knowledge and were not re-checked this session. Treat them as hypotheses.

## 1. Document model and how existing engines edit/save (incremental vs full rewrite)

### Takeaway
Every mature engine keeps two layers: a COS object graph (indirect objects, xref, streams) and a typed model on top of it. Saving is either an **append-only incremental update**, which writes only new or changed objects plus a new xref/trailer after the original bytes, or a **full rewrite**. Incremental is needed to keep existing signatures valid. A full rewrite is needed whenever bytes must really disappear (redaction, removing old revisions or encryption, shrinking the file). Lamina's core should track dirty objects at the COS level so it can emit either mode from the same edit state.

### Cited Findings
- **PDFium** exposes both modes as flags on one save entry point: `FPDF_INCREMENTAL (1<<0)`, `FPDF_NO_INCREMENTAL (1<<1)`, `FPDF_REMOVE_SECURITY (1<<2)`. The header says "FPDF_INCREMENTAL and FPDF_NO_INCREMENTAL cannot be used together." Output goes through a streaming callback struct, `FPDF_FILEWRITE { int version; int (*WriteBlock)(self, const void* data, unsigned long size); }`, via `FPDF_SaveAsCopy(doc, file_write, flags)` and `FPDF_SaveWithVersion(..., int file_version)`. — [PDFium fpdf_save.h](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/fpdf_save.h)
- **pdf.js**: the worker's `SaveDocument` handler passes `originalData: stream.bytes`, the previous `startXRef` and a new temporary ref for the new xref, and notes "Use the same kind of XRef as the previous one." That is an append-style incremental update. If `changes.size === 0` it returns the original bytes unchanged. It also has special paths for XFA datasets, `NeedAppearances`, and for creating or updating the StructTreeRoot when annotations are added (tagged-PDF accessibility). The researcher read the handler but not `writer.js`'s `incrementalUpdate`, so the append behaviour is inferred from the call, not confirmed. — [pdf.js src/core/worker.js](https://github.com/mozilla/pdf.js/blob/master/src/core/worker.js)
- In pdf.js, editor annotations are written out through `PDFDocumentProxy.saveDocument()`. `AnnotationStorage` holds form values and annotation data and has a size/modified check, which works as dirty tracking. A third-party tutorial says the built-in editor layer does not expose annotation events for persistence, so syncing annotations to a DB needs a custom layer. — [Nutrient blog on pdf.js editor layer](https://www.nutrient.io/blog/pdfjs-annotation-editor-layer/); [Tessl pdfjs-dist annotation API summary](https://tessl.io/registry/tessl/npm-pdfjs-dist/files/docs/annotation-api.md) (third-party summaries)
- **qpdf** is at 12.4.2 (2026-09-26). Recent hardening: limits on page-tree depth (12.4.0, 2026-08-09); streams with more than 25 filters treated as unfilterable by default, `--max-stream-filters`, and a new "global limits API" (12.3.0, 2026-01-10); dictionaries/arrays with more than 5000 elements ignored during xref recovery (12.2.0); "radically decrease memory usage … when linearizing" (12.4.2). The release notes the researcher read do not describe incremental-update writing. — [qpdf release notes](https://qpdf.readthedocs.io/en/stable/release-notes.html)
- **qpdf/pikepdf incremental save**: search found no confirmation either way. **[prior knowledge, unverified]** qpdf (and so pikepdf) always rewrites the whole file and has no incremental-update writer, so it cannot add changes to a signed PDF without breaking the signature. — search: [pikepdf PyPI](https://pypi.org/project/pikepdf)
- **PDFBox**: incremental saving historically depended on callers flagging each changed COS object with `COSUpdateInfo.setNeedToBeUpdated(true)`. PDFBOX-5263 reports that with `saveIncremental` "most of the time made changes are completely ignored" and proposes that COS objects track their own changes. Whether that change was merged is unverified. PDFBox 3.0 removed classic signing and kept external signing only (PDFBOX-3492). — [COSUpdateInfo Javadoc](https://pdfbox.apache.org/docs/2.0.11/javadocs/org/apache/pdfbox/cos/COSUpdateInfo.html); [PDFBOX-5263](https://issues.Apache.org/jira/browse/PDFBOX-5263); [PDFBox release notes](https://apache.googlesource.com/pdfbox/+/trunk/RELEASE-NOTES.txt)
- **lopdf 0.46.0** (MSRV 1.85) keeps "everything in memory as high-level objects until finally serializing the entire document". It is eager, not lazy. `doc.objects` is directly mutable. Save options are `save()` (classic xref), `save_modern()` (object streams + xref streams, forces version ≥1.5) and `save_with_options(SaveOptions)`. An `IncrementalDocument` type exists but is undocumented. — [lopdf docs.rs](https://docs.rs/lopdf/latest/lopdf/)
- **LibPDF (Documenso, TS)**: full rewrite is the default, and `pdf.save({ incremental: true })` appends. The docs say "LibPDF cannot automatically detect when incremental saves are required". An incremental save on a new document logs a warning and falls back to a full save. `sign()` switches to incremental automatically when signatures already exist, and the instance reloads after signing. Full rewrite is listed as required for new docs, removing encryption, and dropping old revisions (which invalidates signatures). — [LibPDF incremental saves](https://libpdf.documenso.com/docs/concepts/incremental-saves)
- **Nutrient**: a full save "rewrites the entire PDF document. It takes all the objects still in use and writes them into the file". Incremental saving is opt-in on its Web SDK. Instant JSON is the internal format for persistence and sync, and XFDF is reserved for interop with external tools. — [Nutrient: incremental vs full save](https://www.nutrient.io/blog/incremental-and-full-save-in-pdfs/); [Nutrient document processing](https://nutrient.io/guides/web/features/document-processing); [Nutrient annotation sync](https://nutrient.io/guides/web/annotations/synchronization)
- **Apryse WebViewer** separates *marking* redactions (annotations) from *applying* them. `applyRedactions` "removes the text, images or page areas … from the PDF document's content stream. This cannot be reversed." — [Apryse redaction guide](https://docs.apryse.com/web/guides/features/redaction/redact); [Apryse secure redaction guide](https://apryse.com/secure-redaction-guide)
- A pdf-lib fork (Cantoo) offers incremental saving "designed to be used for pdf signing", with an option to force a full rewrite. — [@cantoo/pdf-lib README](https://unpkg.com/@cantoo/pdf-lib@2.7.1/README.md)

### Inferences
- Recommended layering for lamina:
  1. **`cos` layer**: lazy xref-driven object loader. Objects are parsed on demand from a byte source (mmap on desktop, `Uint8Array`/OPFS-backed range reader on web). An overlay map `ObjRef → (New|Modified|Deleted)` sits on top of the immutable base revision.
  2. **typed model**: `Page`, `Annotation`, `Field`, `Outline` and so on, as typed views/handles onto COS objects, not separate copies.
  3. **save planner**: chooses `Incremental` (write overlay only, keep the base bytes verbatim; required if signatures exist and must stay valid) or `FullRewrite` (garbage-collect unreachable objects and re-serialize; required after applied redaction, for decryption, compaction or linearization). Expose both explicitly, as PDFium and LibPDF do. Also auto-select or warn, which LibPDF does not do.
- The PDFBox lesson: change tracking must be automatic at the COS mutation API (every `set` marks the object dirty). Manual flags are a known source of bugs.
- lopdf's eager whole-document model does not fit large files in WASM. Lamina should either build its own lazy COS layer or fork lopdf toward lazy loading. **[inference]**

### Gaps
- The researcher did not read pdf.js `writer.js` `incrementalUpdate`, so it is unconfirmed whether pdf.js ever falls back to a full rewrite (for example after page reorganization).
- No authoritative source found on whether qpdf/pikepdf can write incremental updates.
- Apryse and Nutrient internal object models are proprietary and their mechanics are not documented.

## 2. Command pattern / operation log, undo/redo, transactions, dirty tracking

### Takeaway
Use a **semantic operation log** (typed ops such as `AddAnnot`, `SetFieldValue`, `MovePage`, `ApplyRedaction`) on top of COS mutations, grouped into transactions with inverse ops for undo. Undo should be local-user-scoped (Figma/Yjs style) and should not rewind the global history.

### Cited Findings
- Figma's undo principle: undoing, copying and redoing back to the present should leave the document unchanged. "An undo operation modifies redo history at the time of the undo" (inverse ops are captured from the current state, not the original state). Deleted object data is kept in the deleting client's undo buffer, not on the server. — [Figma multiplayer blog](https://www.figma.com/blog/how-figmas-multiplayer-technology-works/)
- Yjs `UndoManager`: every transaction has an origin (null by default). `trackedOrigins` selects which changes are undoable, and the manager always tracks its own origin. Remote changes are ignored by default, and `captureTimeout` merges ops that happen close together into one undo step. — [Yjs UndoManager docs](https://docs.yjs.dev/api/undo-manager); [Tessl yjs 13.6 summary](https://tessl.io/registry/tessl/npm-yjs/13.6.0/files/docs/undo-redo-system.md) (third-party)
- pdf.js returns the original bytes when its change map is empty. Its dirty tracking is "set of changed refs" plus AnnotationStorage `modified`. — [pdf.js worker.js](https://github.com/mozilla/pdf.js/blob/master/src/core/worker.js)

### Inferences
- Suggested shape (Rust):
  - `enum Op { AddAnnot{page, id, data}, UpdateAnnot{id, patch}, DeleteAnnot{id}, SetField{name, value}, InsertPages{at, src}, MovePage{id, to}, RotatePage{..}, DeletePage{id}, EditContent{page, edits}, MarkRedaction{..}, ApplyRedactions{ids}, Sign{field, ..} }`
  - `Transaction { ops: Vec<Op>, inverse: Vec<Op>, origin: Origin, ts }`
  - `doc.transact(|tx| …)` applies ops atomically. On error it rolls back the COS overlay.
  - The undo stack stores inverse ops, recomputed on undo as Figma does.
- Op classes need different undo semantics:
  - **Reversible-in-session**: annotations, fields, page moves.
  - **Barrier ops**: `ApplyRedactions` and `Sign` cannot be undone once saved, so they should clear or segment the undo stack after the save commits. Before the save, undo can still drop them from the overlay.
- Dirty tracking works at two levels: semantic (op log non-empty since last save) for UI "unsaved" state, and COS (changed refs) for the save planner.
- Keep stable lamina IDs for pages and annotations, for example via `/NM` on annotations and a sidecar ID map for pages, so ops remain valid across reorders. This is also needed for sync (section 3). **[inference]**

### Gaps
- No PDF-specific public write-up was found on op-log undo inside pdf.js, PDFium or Nutrient.

## 3. Collaboration: annotation layer sync, CRDT vs server-authoritative, conflict semantics

### Takeaway
Industry practice (Nutrient Instant, Figma, tldraw) is **server-authoritative, record-level or property-level last-writer-wins** over a document of records, with the base PDF treated as immutable. Annotations and form values sync as a separate layer (Instant JSON/XFDF-like records). Page operations are rarer and structural, so they need stricter handling: serialize them through the server or give them explicit ordering via fractional indices. CRDT libraries (Yjs, Automerge 3, Loro 1.x) fit offline-first or peer-to-peer, but are not required for the record-shaped annotation data.

### Cited Findings
- **Figma**: client/server with one process per document. "Since Figma is centralized (our server is the central authority), we can simplify our system". It is inspired by CRDTs but is not a CRDT. The server keeps the last value per (object, property), so edits to different properties don't conflict. The client discards incoming server changes that conflict with its own unacknowledged property changes. Object create/delete is explicit. Reparenting is a parent-pointer property, and the server rejects cycles. Child order uses fractional indexing, stored together with the parent link in one property. Offline: on reconnect the client fetches a fresh copy, reapplies its offline edits and resumes. — [Figma multiplayer blog (2019)](https://www.figma.com/blog/how-figmas-multiplayer-technology-works/)
- **tldraw sync**: `TLSocketRoom` "stores the authoritative copy of the document". The Cloudflare template uses one Durable Object per room, persisted with `SQLiteSyncStorage` (the in-memory default loses data on restart). The document is schema-validated records shared between client and server, and migrations let clients on different versions collaborate. "There's only ever one `TLSocketRoom` globally for each room." Hibernation is supported via `onSessionSnapshot`/`handleSocketResume`. Comments are a separate record lane with per-session read/write access. — [tldraw sync docs](https://tldraw.dev/docs/sync)
- **Nutrient Instant**: a prebuilt server (Document Engine) that syncs annotations across users and devices. "When using Nutrient Instant, conflict resolution is automatically managed for you", and "Conflict resolution isn't supported without Instant synchronization." It distinguishes update conflicts from delete conflicts. Instant JSON is the sync payload format. `applyOperations` with Instant JSON on an open document forces a reload. The exact resolution rules (LWW etc.) are not published. — [Nutrient conflict resolution](https://www.nutrient.io/guides/web/annotations/synchronization/conflict-resolution.md); [Nutrient Instant sync](https://nutrient.io/guides/web/instant-synchronization); [Nutrient real-time collaboration](https://www.nutrient.io/guides/document-engine/viewer/real-time-collaboration/)
- **Automerge 3.0**: memory use cut by more than 10x. One example: Moby Dick pasted in takes 700 MB in Automerge 2 and 1.3 MB in Automerge 3, by using the columnar on-disk format in memory. Same file format as Automerge 2, nearly backwards-compatible API. July 2026: the Hexane columnar storage engine v1 was rewritten and is about 2–9× faster depending on benchmark. — [Automerge 3.0 blog](https://automerge.org/blog/automerge-3/); [This Month in Automerge: July '26](https://automerge.org/blog/2026-july/)
- **Loro 1.0**: stable encoding format, faster import/export, version control features, history GC. Loro's own benchmarks (2024) show snapshot sizes comparable to Automerge. They are self-published and predate Automerge 3. — [Loro v1.0 changelog](https://loro.dev/changelog/v1.0.0-beta); [Loro doc-size benchmarks](https://cn.loro.dev/docs/performance/docsize); [Loro JS/WASM benchmarks](https://loro.dev/docs/performance)
- **Cloudflare Durable Objects limits**: 30 s default CPU per request, configurable to 5 min (resets per request or WebSocket message). 10 GB SQLite storage per object on the paid plan. 32 MiB max received WebSocket message. Soft limit of 1,000 req/s per object. The limits page does not state memory per isolate. — [DO limits](https://developers.cloudflare.com/durable-objects/platform/limits/)

### Inferences
- **Recommended model for lamina**: base PDF bytes are content-addressed and immutable (stored in R2). A "layer document" of records sits on top: annotations keyed by stable id, field values keyed by fully qualified name, and page-order entries `{pageId, srcRef, rotation, fracIndex}`. It is synced through one DO per document, with property-level LWW plus explicit create/delete, as Figma and tldraw do. Flattening into PDF bytes happens at export or save time through the Rust core. This matches Nutrient's split of "Instant JSON layer for sync, PDF for interchange".
- **Conflict semantics**:
  - annotation property edits use LWW per property;
  - deletes win over concurrent updates, or are tombstoned for undo;
  - page reorders use fractional indices;
  - page delete vs. concurrent annotation on that page: either orphan the annotations (hidden) or cascade-delete. Pick explicitly. Figma's "temporarily remove from tree" approach suggests orphaning;
  - content-stream edits and applied redaction are **not mergeable**. Serialize them with a server lock or lease, or run them as a server job that produces a new base revision, which bumps the base version for every client.
- A CRDT (Loro has a movable tree and Rust-native core; Automerge 3 is Rust with much lower memory) is worth it if offline-first or peer-to-peer without a server is a requirement. Both are Rust crates and can live inside the same WASM module as lamina's core. Yjs is JS-first (Yrs is its Rust port). **[inference; Loro movable-tree and Yrs details are prior knowledge, unverified]**
- Keep the 32 MiB WS message limit in mind: never send PDF bytes over the sync socket. Send ops and refs to R2 blobs instead.

### Gaps
- Liveblocks' current architecture (Storage/Yjs, 2026) was not researched in this session.
- Nutrient's exact conflict rules are unpublished.
- No head-to-head 2026 benchmark of Yjs vs Automerge 3 vs Loro was found.

## 4. Where operations run; WASM memory limits

### Takeaway
On the web, run the Rust core in a **dedicated Web Worker**, as pdf.js does with its worker-side `SaveDocument`. On desktop, use native threads. Send heavy jobs (OCR, conversion, applying redactions to huge files) to server containers. wasm32 caps linear memory at 4 GB, and **Memory64 is still unsupported in Safari (desktop and iOS, up to 27.x) as of Oct 2026**, so the web build must stream and stay lazy instead of loading whole documents.

### Cited Findings
- Memory64: Chrome 133+ and Firefox 134+ support it. Safari desktop does **not** (through 27.1 and Technology Preview), and Safari iOS does **not** (through 27.2). — [caniuse Memory64](https://caniuse.com/wf-wasm-memory64); [SpiderMonkey: Is Memory64 actually worth using? (2025-01-15)](https://spidermonkey.dev/blog/2025/01/15/is-memory64-actually-worth-using.html)
- The SpiderMonkey post evaluates Memory64's performance cost (bounds checks) relative to wasm32. — [SpiderMonkey blog](https://spidermonkey.dev/blog/2025/01/15/is-memory64-actually-worth-using.html)
- pdf.js parses and saves in its worker (`src/core/worker.js` handles `SaveDocument`), keeping the main thread for display. — [pdf.js worker.js](https://github.com/mozilla/pdf.js/blob/master/src/core/worker.js)
- DO CPU limits (30 s default, 5 min max per request) make DOs a poor fit for heavy PDF jobs. They suit coordination; heavy compute belongs in containers or jobs. — [DO limits](https://developers.cloudflare.com/durable-objects/platform/limits/)

### Inferences
- Ship wasm32 only for now. Design the COS layer around a `ByteSource` trait with range reads (File/Blob slices, OPFS, HTTP range) so a 2 GB PDF never has to sit in linear memory all at once. Saves stream out through a `WriteBlock`-style sink, as PDFium's `FPDF_FILEWRITE` does, into OPFS or a stream.
- Keep the typed model and op log in the worker. The UI thread holds a projection (view model) updated via message diffs.
- Use the same Rust crate natively on desktop, with a thread pool for rendering and parsing.

### Gaps
- Cloudflare Containers limits (memory/CPU, 2026) were not fetched.
- Practical per-tab memory ceilings in Safari/iOS for WASM were not verified.

## 5. Safety: parsing untrusted PDFs

### Takeaway
Treat PDFs as hostile. Parse them only in an isolated worker or process, use explicit resource limits (nesting depth, filter-chain length, decompressed size, object counts, page-tree depth), and fuzz continuously. qpdf's 2025–2026 changes are a good checklist.

### Cited Findings
- qpdf limits added recently: page-tree depth limit (12.4.0, Aug 2026); more than 25 filters means unfilterable, configurable through `--max-stream-filters`, plus a global limits API (12.3.0, Jan 2026); arrays/dicts with more than 5000 elements ignored during xref recovery (12.2.0, May 2025). qpdf is fuzzed on OSS-Fuzz, and its JSON parser was added in 11.9.0 (Feb 2024). — [qpdf release notes](https://qpdf.readthedocs.io/en/stable/release-notes.html)

### Inferences
- Lamina should provide a `Limits` struct with: max object depth, max filter chain, max decompressed bytes per stream and in total (zip-bomb guard), max xref entries, max page-tree depth, max content-stream ops per page, and a time or fuel budget.
- Fuzz with `cargo-fuzz` (libFuzzer) on parser, xref repair, filters and content-stream lexer, and apply to OSS-Fuzz. Rust removes memory-unsafety classes but not DoS or panics, so use `catch_unwind` at the worker boundary or abort-and-restart the worker. **[inference]**
- Desktop: run parsing in a separate sandboxed process (OS sandbox) if feasible. Web: the worker plus WASM linear memory gives memory isolation for free. **[inference]**

### Gaps
- pdf.js and PDFium fuzzing setups were not researched this session.

## 6. Signatures and redaction: implications for the architecture

### Takeaway
These two features drive the save planner. **Signing is append-only** and must be the last write of its revision. Later edits must be incremental, or the signature breaks. **Applied redaction requires a full rewrite**, because the removed content must be gone from every revision. That makes it incompatible with keeping earlier signatures.

### Cited Findings
- Signatures cover byte ranges, and rewriting moves those bytes and breaks verification. Use incremental saves for form filling on signed docs, for adding a second signature, and for certified docs. — [LibPDF incremental saves](https://libpdf.documenso.com/docs/concepts/incremental-saves)
- A full rewrite drops old revisions and "invalidates signatures". — [LibPDF incremental saves](https://libpdf.documenso.com/docs/concepts/incremental-saves); [Nutrient incremental vs full save](https://www.nutrient.io/blog/incremental-and-full-save-in-pdfs/)
- Apryse `applyRedactions` removes the content from content streams irreversibly. Marking and applying are separate steps. — [Apryse redaction](https://docs.apryse.com/web/guides/features/redaction/redact)
- Apryse community: with multiple signatures you can only remove the most recent one without invalidating the others. Signed PDFs are typically saved incrementally. — [Apryse community thread](https://community.pdftron.com/t/removing-digital-signatures-from-a-pdf-document/2061)
- PDFBox 3 supports external signing only, through `saveIncrementalForExternalSigning`, which yields the byte range to sign. Calling it more than once caused an IllegalStateException (PDFBOX-3811/3978). — [PDFBox release notes](https://apache.googlesource.com/pdfbox/+/trunk/RELEASE-NOTES.txt); [PDFBOX-3811](https://jira.apache.org/jira/browse/PDFBOX-3811)

### Inferences
- Model the document as a **revision chain**: `base revision + [incremental revisions]`. Lamina's document state carries a `SaveConstraint` with three values:
  - `Free`: no signatures;
  - `AppendOnly`: signatures present, so only incremental saves are allowed and ops are restricted to what the DocMDP/FieldMDP permissions allow;
  - `RequiresRewrite`: pending applied redactions or decryption.
- If both apply (redact a signed doc), the UI must warn that the signatures will be invalidated. Then do a full rewrite, and remove or flatten the signature fields.
- Ordering rule: redact, then full rewrite, then sign, then incremental save. Signing should be a two-phase op: prepare (reserve the `/Contents` placeholder and compute the ByteRange), then the external signer (WebCrypto, HSM or a server), then inject. That supports remote and server signing.
- Redaction must also scrub annotations, form values, metadata/XMP, the structure tree, thumbnails and embedded files, and must drop previous revisions (full rewrite). The researcher found no single source with this list. **[inference/prior knowledge]**

### Gaps
- No vendor doc explicitly states "applyRedactions + save invalidates existing signatures". This is inferred from the byte-range mechanics.
- DocMDP/FieldMDP permission enforcement details were not researched.

## 7. API shape for generate-and-edit (TS and Rust)

### Takeaway
Generator libraries (krilla, printpdf, PDFKit, typst) expose a write-once **builder** (`Document → page → surface → finish() → bytes`). Editors (pdf-lib, LibPDF, PDFium, lopdf) expose a mutable **document API** over a loaded file. Lamina needs both, sharing one COS core: `Builder` for authoring, which produces a `Document`, and `Document::open(bytes)` for editing, with `save({mode})`.

### Cited Findings
- **krilla 0.8.2**: `Document::new()`, then `start_page_with(PageSettings::from_wh(..))`, then `page.surface()` with `draw_text`/`draw_path`/`set_fill`, then `surface.finish()`, `page.finish()`, and `document.finish()` to get bytes. Modules include `configure` (version/export mode), `tagging` (accessibility), `annotation`, `outline` and `embed`. It is built on `pdf-writer` and generate-only, with no parsing or editing API on the index page. — [krilla docs.rs](https://docs.rs/krilla/latest/krilla/)
- **lopdf 0.46**: low-level mutable object map (`doc.objects.insert(..)`) with `save`/`save_modern`/`save_with_options`. — [lopdf docs.rs](https://docs.rs/lopdf/latest/lopdf/)
- **LibPDF (TS)**: `await pdf.save()` and `await pdf.save({ incremental: true })`, with `sign()` that auto-incrementalizes. — [LibPDF](https://libpdf.documenso.com/docs/concepts/incremental-saves)
- **PDFium**: handle-based C API with a streaming writer callback and save flags. — [fpdf_save.h](https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/fpdf_save.h)

### Inferences
- **[prior knowledge, unverified this session]**
  - pdf-lib (TS) uses `PDFDocument.create()/load(bytes)`, `doc.addPage()`, `page.drawText()`, `doc.getForm().getTextField(name).setText()`, `doc.copyPages()` and `doc.save()`. It is an eager, full-rewrite-only API (upstream). Its maintenance has been dormant for years, which is why forks like @cantoo/pdf-lib exist.
  - PDFKit (Node) is a streaming, chainable generator (`doc.text().moveDown()`, piped output).
  - printpdf is a Rust generator with an op-list page model.
  - typst compiles a markup model to frames and exports through krilla.
- Proposed lamina surface:
  - Rust: `Document::open(src, Limits)`; `doc.pages()`, `doc.page(i).annotations()`; `doc.transact(|tx| { tx.add_annotation(..); tx.move_page(..) })`; `doc.undo()`; `doc.save(sink, SaveMode::{Auto, Incremental, Full})`. Authoring uses `Document::build()`, then `page_builder` and `surface` (krilla-like), and returns a `Document`, so generated files can be edited further in the same process.
  - TS (wasm-bindgen, in a worker): an async, promise-based mirror (`await doc.transact(tx => …)`), events (`doc.on('change', ops)`) for UI and sync bindings, and serializable `Op` JSON as the boundary type. The op stream is what sync and undo consume.
- Consider delegating authoring to krilla, a mature Rust generator used by typst, and building lamina's own editing COS layer. Check krilla's licence (MIT/Apache, prior knowledge) and whether it can write into an existing object graph. It is currently write-only. **[inference]**

### Gaps
- No version or status check was done this session for pdf-lib, PDFKit, printpdf or typst.
- Whether krilla can emit objects into an existing document (needed for "edit with builder") is unknown.
