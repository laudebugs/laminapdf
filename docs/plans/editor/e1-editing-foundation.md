# E1: editing foundation

**Status:** Draft

The first editor round turns the web reader into an editor that can change a document, undo the change and save it safely. It does this with the smallest set of edits that touch every layer: the document's metadata (title, authors, subject, keywords, dates, producer and creator) and the page rotation that already exists. Each later round adds its tools to what this one builds: the edit session, the undo history and the save path. It also builds the strip that says what saving will do to a signed or repaired file, and the in-app page that shows what Lamina PDF can edit.

- **Round:** [full-editor §E1](../full-editor.md#e1-editing-foundation), phase 2, milestone M5 Organiser
- **Feature-plan todo:** [E1a](../../todos/done/E1a-editing-foundation-feature-plan.md)
- **Follows:** ADR [0005](../../decisions/0005-document-model.md) (the model and the overlay), [0006](../../decisions/0006-op-log-and-undo.md) (the op log and undo), [0007](../../decisions/0007-save-planner.md) (the save planner), [0009](../../decisions/0009-web-client-architecture.md) (the document worker), [0010](../../decisions/0010-platform-adapter.md) (the `Platform` adapter), [0024](../../decisions/0024-app-shell-ux.md) (modes and the command registry)
- **Design:** [O01–O05 in the Organise area](../../design/paper/edit-session-organise.md), the strip in [design language §7.29](../../design/paper/design-language.md#729-strip-proposed)
- **Sequence:** the round starts on the web reader and the shared packages. Its desktop parts are separate todos that wait on the desktop todos they need ([full-editor §7](../full-editor.md#7-phases-and-rounds))

## 1. Where things stand

The library can already edit and save, but nothing above it can, and its metadata support is half done.

- **Ops.** `laminapdf-ops` has two ops. `SetInfo` patches six Info text entries and can write `/ModDate`; `RotatePage` turns a page. Both are class `Metadata` or `PageStructure`, and neither needs a rewrite. Undo, redo, transactions and savepoints work, with frozen JSON samples in `fixtures/ops/v1/`.
- **Saving.** The planner detects signatures, DocMDP certification, FieldMDP locks, usage rights and repaired or encrypted bases, and picks an incremental save or a full rewrite. Saves are deterministic. `SaveOptions` has no date or ID for the metadata a save should stamp.
- **XMP.** Nothing reads an XMP packet beyond handing out its bytes. The builder writes one with hand-written code, and `SetInfo` changes Info without touching it, so an edited document's Info and XMP disagree.
- **Bindings.** The façade, the WASM module and `@laminapdf/sdk` expose `apply`, `undo`, `redo`, `save` and `hasUnsavedChanges`, and the SDK's worker client serves them. None of them exposes the save constraint.
- **The web reader.** It holds the document in the viewer's worker as a Lamina PDF model next to a separate PDFium document opened from the same bytes. PDFium draws every page, so an edit to the model doesn't change what the reader shows. The viewer can't redraw one page or change a page's size after it opens a file.
- **The interface.** `@laminapdf/ui` has the mode registry with one mode (`read`), a reader state machine whose only interaction mode is `select`, the command registry (the web app registers `file.save-copy` and `file.properties` in `apps/web/src/output.tsx`), and a read-only Properties dialog. There is no Save, Undo, history, dirty state, close guard or strip component.

## 2. The operations

The round implements four rows of the [feature inventory](../../research/pdf/06-feature-inventory.md) to `round-trip` in the [capability matrix](../../capabilities/README.md). Their rows are already in `docs/capabilities/manifest.toml` with `round = "E1"`. E1c flips the core and `pdf` (`@laminapdf/sdk`) columns, E1f the web column and E1i the desktop column.

| ID | What it means | Target |
| --- | --- | --- |
| J.01 | Title, author, subject, keywords, kept the same in Info and XMP | round-trip |
| J.02 | Creation and modification dates, in Info and XMP | round-trip |
| J.03 | Producer and creator tool | round-trip |
| J.10 | Remove selected metadata from both Info and XMP | round-trip |

Every edit is an `Op` ([full-editor §5](../full-editor.md#5-requirements-that-hold-for-every-round), requirement 1). The ops name what they change and carry every value they write, so a log replays to the same bytes: no op reads a clock, a random source or the platform.

| Op | Payload | Class | Rewrite |
| --- | --- | --- | --- |
| `SetMetadata` | `{ patch: MetadataPatch, at: Option<PdfDate> }`. Each field of the patch (`title`, `authors` as a list, `subject`, `keywords`, `creator`, `producer`, `created`, `modified`) is absent (left alone), `null` (removed) or a value; `at` is the modification date the edit writes, given by the caller | `Metadata` | no |
| `RemoveMetadata` | `{ keys: Vec<MetadataKey> }`: the fields to remove from Info and XMP; the packet's other properties stay | `Metadata` | no |
| `UpdateSaveMetadata` | Added by the planner at save time, never by a caller: `xmp:MetadataDate`, `/ModDate`, `xmp:ModifyDate` and `xmpMM:InstanceID` | `Metadata` | no |
| `SetInfo` | Unchanged on the wire. It stays readable for logs already written (the frozen v1 sample) and now goes through the same writer, so it updates XMP too. Deprecated in favour of `SetMetadata` | `Metadata` | no |

A save stamps the save-time metadata through `UpdateSaveMetadata`, from values the shell passes in `SaveOptions`: `metadata_date` (the clock reading) and `instance_id`. The core never reads a clock. Without a date the dates stay as they were, and without an ID the planner derives `InstanceID` from the content hash, so saves stay deterministic. The op is classified like any other: a save whose constraint it would break skips it and says so in `SaveReport::metadata_not_updated`. A save that changed nothing stamps nothing.

**How Info and XMP stay together.** One table maps each field to its Info key and its XMP property, and the builder, the reader and the ops share it ([04 §5](../../research/pdf/04-interactive-security.md)):

| Field | Info | XMP |
| --- | --- | --- |
| title | `/Title` | `dc:title` (Alt, `x-default`) |
| authors | `/Author` (joined with `; `) | `dc:creator` (Seq) |
| subject | `/Subject` | `dc:description` (Alt, `x-default`) |
| keywords | `/Keywords` | `pdf:Keywords` |
| creator | `/Creator` | `xmp:CreatorTool` |
| producer | `/Producer` | `pdf:Producer` |
| created | `/CreationDate` | `xmp:CreateDate` |
| modified | `/ModDate` | `xmp:ModifyDate` |

An edit writes both sides from one value, at the same precision and time zone. A file without a packet gets a minimal one; a file with one keeps every property Lamina PDF doesn't map, in place (§4). In a PDF 2.0 file, where ISO 32000-2 §14.3.3 deprecates every Info entry but the two dates, an edit writes the other fields to XMP only and leaves the old Info entries as they were.

**Reading.** When Info and XMP disagree, `metadata()` reports the XMP value in a PDF 2.0 or PDF/A file, and otherwise the side whose modification date is newer, with Info winning a tie. It also reports that they disagree, so the Properties dialog can say so.

## 3. Conformance the edits must keep

- **PDF/A.** The `pdfaid:` part and conformance, and any extension schema, stay in the packet whatever the edit (ISO 19005-2 §6.6.4, veraPDF rules 6.6.4-1 to 6.6.4-7). PDF/A-1 requires `dc:creator` to have exactly one entry (ISO 19005-1 §6.7.3, checked by veraPDF's 6.7.3 rules). An edit that gives a PDF/A-1 file two authors therefore raises a `ConformanceWarning` before the save. veraPDF must still pass on every edited PDF/A fixture.
- **PDF/UA-1.** `dc:title` is required (ISO 14289-1 §7.1, veraPDF rule 7.1-9), and so is `/DisplayDocTitle` (7.1-10). Removing the title of a file that claims PDF/UA-1 raises a `ConformanceWarning`, and the edit keeps `/DisplayDocTitle`.
- **The packet.** The writer emits no `bytes` or `encoding` attribute in the `xpacket` header and writes UTF-8 (ISO 19005-2 §6.6.2.1, veraPDF rules 6.6.2.1-2, -3 and -5), unfiltered, with about 2 KB of padding and `end="w"`.

The planner returns these warnings with `planner.conformance_warnings()`. They warn and never block, since the user may mean to drop the claim.

## 4. Crate and package changes

- **`laminapdf-model`: an `xmp` module** ([E1b](../../todos/ready/E1b-xmp-packet-model.md)). A tolerant reader of a packet's RDF/XML into an ordered tree that keeps every node, so a packet read and written unchanged is byte-identical. It handles properties in attribute and element form, several `rdf:Description` elements, `rdf:parseType="Resource"`, `rdf:Alt`, `Seq` and `Bag` with `xml:lang`, and the `xpacket` wrapper. It refuses a DTD. Typed accessors read the mapped properties, and `pdfaid`/`pdfuaid` read-only. The writer changes only the properties Lamina PDF owns and keeps the rest verbatim, inside the existing padding when the new packet fits. A packet it can't edit in place, such as a hand-written one it can't map back to its text, is rewritten in canonical form with every property kept, and the save says so.
- **XML.** The reader and writer are our own, over `quick-xml` (MIT), which keeps namespaces and comments for a lossless round trip. The other choices fail a requirement. Adobe's `xmp_toolkit` 1.12 wraps the C++ XMP SDK and lists no wasm32 target (its README's supported platforms). `xmp-writer` 0.3 only writes, so other tools' properties would be lost. `gufo-xmp` 0.5 is MPL-2.0 or LGPL-2.1 (its crates.io licence field), outside ADR 0019's list. The wasm size the module adds is measured against the base module's budget in E1b.
- **`laminapdf-ops`:** `SetMetadata`, `RemoveMetadata`, the shared mapping table and `SetInfo` routed through the new writer ([E1c](../../todos/ready/E1c-metadata-ops.md)).
- **`laminapdf-save`:** `SaveOptions::{metadata_date, instance_id}`, `SaveReport::metadata_not_updated`, `planner.conformance_warnings()`.
- **The façade and bindings:** `Document::metadata()` and the two ops, with API docs and the runnable examples `examples/edit-metadata.rs` and `packages/sdk/examples/edit-metadata.ts` ([full-editor §5](../full-editor.md#5-requirements-that-hold-for-every-round), requirement 7: library parity). The save constraint becomes a record (`plan`), so the interface can explain a save before it happens. In `@laminapdf/sdk`: `doc.metadata()`, `doc.setMetadata(patch)`, `doc.removeMetadata(keys)`, `doc.plan(options)`, all served by the worker client.
- **`@laminapdf/viewer`:** the edit session in the document worker (§5), and redrawing after an edit (§6).
- **`@laminapdf/ui`:** the interaction modes (§5), Save, Save as, Download a copy, Undo, Redo, the history rail tab, the dirty indicator, the close guard, the strip (§7.29), the editable Properties dialog and the capability page.

## 5. The edit session and the modes

The document worker owns one edit session per open document: the Lamina PDF model, its op log and its undo history. The interface sends ops and receives the outcome, the touched pages and a summary of the history; it never holds the document. The session's methods are `apply(ops)`, `undo()`, `redo()`, `history()`, `plan(options)` and `save(options)`, with a `dirty` event that carries the touched pages. History entries carry a label and the outcome, so `Partial` and `Skipped` undos show as toasts ([ADR 0006 §4](../../decisions/0006-op-log-and-undo.md)). The history keeps 500 transactions and drops the oldest after that.

**How modes are typed.** Two registries already exist, and later rounds add to both without touching E1's code:

- **Shell modes** (Read, Comment, Organise, Edit, Fill & Sign) stay `ModeRegistry` entries ([ADR 0024 §2](../../decisions/0024-app-shell-ux.md)). Each owns one secondary toolbar. E1 adds none; Organise arrives with E2.
- **Interaction modes** (`select`, `hand`, `textSelect` now; `annotate`, `formFill`, `formDesign`, `contentEdit`, `redact` and `measure` later) become a registry of tool modules keyed by name. The names form a TypeScript interface that each round extends by declaration merging, so `InteractionMode` is the union of its keys, and the reader machine's `set-mode` event takes any registered name. A tool module supplies its pointer handlers, its cursor and the layer it draws into; the machine knows nothing else about it.

## 6. Redrawing after an edit

PDFium draws the pages from the bytes it opened, so the model's edits must reach it as bytes. After each committed transaction that touches pages, the worker writes an incremental update of the model to memory, reopens PDFium's document from the new bytes and tells the viewer which pages to redraw. The viewer gains a way to drop one page's bitmaps and to change a page's size, since a rotation swaps its width and height. Metadata edits touch no page, so they redraw nothing. Undo and redo redraw the pages the undone transaction touched.

This reload is the [two parsers path](../architecture.md#6-risks) the architecture plan names as a risk. E1 measures the time from `apply` to the first redrawn tile, per document size, in the viewer's performance harness. The proposed budget is 150 ms for a 100-page file, which E1g confirms or adjusts from what it measures. A miss on large files is acceptable in E1 if it is measured and reported, and E2 then decides whether to reload only touched pages or to render from Lamina PDF's own model. No E1 todo owned this step before the plan, so it is a new todo ([E1g](../../todos/ongoing/E1g-redraw-after-edit.md)), which E1d builds on.

**Measured (E1g).** The worker saves only the appended revision, and the model rebases onto the file with it, so a save copies nothing. PDFium then opens the file with every appended revision over a source that joins the two, and reads only the blocks it needs. The viewer's performance harness turns the page in view a quarter five times per file and reports the median. On an Apple M1 Pro in Chromium 156 (2026-11-10), the edit's call answered in 7, 5 and 18 ms for 10, 100 and 1,000 pages. The page's first new pixels came 19, 20 and 36 ms after the call, and the page was sharp again after 48, 49 and 65 ms. Reopening the file grows with its size, but slowly: 1,000 pages cost 16 ms more than 100.

**The budget stays at 150 ms for 100 pages.** The measured 20 ms leaves room for a phone about four times slower than this laptop, and for the content edits of later rounds, whose pages draw more slowly than a turned one. Reloading on every edit is fast enough at 1,000 pages too, so E2 needn't reload only the touched pages for speed alone.

## 7. Saving and the save constraint

Every save plans first ([full-editor §5](../full-editor.md#5-requirements-that-hold-for-every-round), requirement 2). The plan's kind decides what Save does and what the strip says.

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update; Save as optimised rewrites | Nothing |
| `AppendOnly`: approval signatures | Signed, but not certified | An incremental update, so the signatures stay valid and show the change as made after signing | An accent strip: "Signed: your changes are added after the signatures" |
| Usage rights (UR3) | Extended by Adobe Reader rights | UR3 grants only form filling, annotations and signing, so a metadata edit is a violation and the constraint becomes `Conflict`, as for certified files | The same warning and confirmation as a certified file |
| `AppendOnly`: a FieldMDP lock | Locked form fields | A FieldMDP lock restricts only the fields it names, and a metadata edit changes no field, so the edit saves as an incremental update | The same accent strip |
| `AppendOnly` with DocMDP P1, P2 or P3 | Certified | A metadata edit violates every level, since the changes each level allows (ISO 32000-2 §12.8.2.2) don't include metadata. The edit applies, which makes the constraint `Conflict`; it saves only through the explicit override that removes the signatures | Before the edit applies, the Properties dialog warns with `would_violate`'s verdict, naming the signature. Saving then asks for confirmation, naming the signatures that will be removed |
| `RequiresRewrite` | A repaired file, or a document with no base | A full rewrite | A neutral strip that says the file will be rewritten and why |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11 can encrypt on write; Download a copy of the original still works | A warning strip that names the reason and offers saving without the password, after a confirmation |
| `Conflict` | Ops in the session that the constraint forbids | Save is refused; Save as invalidating the signatures, after a confirmation | A danger strip with the conflicting changes and a way to undo them |

This table corrects [full-editor §E1](../full-editor.md#e1-editing-foundation), which said metadata edits are allowed on signed files as incremental updates. That holds for approval signatures and FieldMDP locks, not for certified files. The certification-attack research (Rohlmann, Mladenov, Mainka and Schwenk, IEEE S&P 2021) shows how far viewers' readings of the allowed changes differ, so the planner follows the specification's list strictly.

**Where the bytes go.** The `Platform` decides, never the interface ([ADR 0010](../../decisions/0010-platform-adapter.md)):

- **The web with the File System Access API** (Chromium): Save writes in place through a writable stream; Save as asks for a place.
- **The web without it** (Firefox, Safari): Save becomes Download, and the session rebases onto what was downloaded, so the next save is again incremental against that copy.
- **The desktop** writes in place, appends an incremental update, rewrites through a temporary file and a rename, and watches for changes made outside. That is [E1i](../../todos/ready/E1i-desktop-save-in-place.md).

A failed write leaves the document dirty and the original file as it was.

## 8. Differences between the shells

| Area | Web | Desktop |
| --- | --- | --- |
| Session | In the document worker | In the engine process ([E1h](../../todos/ready/E1h-desktop-edit-session.md), after D4b) |
| Undo and Redo | Toolbar, shortcuts, palette | The same, plus the native Edit menu, generated from the command registry (D4b) |
| Close guard | `beforeunload` | Tauri's `CloseRequested` (E1h) |
| Save | In place with the File System Access API, else Download | In place, with conflict detection (E1i, after D3b) |
| Capability flags | `canWriteInPlace`, `canSaveAsDialog` | `canWriteInPlace`, `canWatchFiles` |

No server feature: every edit and save happens on the device, and nothing leaves it ([ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md)).

## 9. Todos

| ID | Todo | Depends on |
| --- | --- | --- |
| [E1b](../../todos/ready/E1b-xmp-packet-model.md) | The XMP packet model: a lossless reader and writer, the mapping table, a minimal packet | E1a |
| [E1c](../../todos/ready/E1c-metadata-ops.md) | Metadata ops with Info and XMP in step, save metadata, conformance warnings, bindings | E1b |
| [E1g](../../todos/ongoing/E1g-redraw-after-edit.md) | Redraw after an edit: the worker's incremental reload of PDFium, per-page invalidation and page sizes in the viewer, measured | E1a |
| [E1d](../../todos/ready/E1d-edit-session-undo-ui.md) | The edit session in the document worker, the interaction modes, undo and redo, history, dirty state and the web close guard | E1g |
| [E1e](../../todos/ready/E1e-save-and-constraint-banner.md) | Save, Save as and Download a copy on the web, and the save-constraint strip | E1c, E1d |
| [E1f](../../todos/ready/E1f-metadata-ui-capability-page.md) | The editable Properties dialog and the capability page | E1c, E1e |
| [E1h](../../todos/ready/E1h-desktop-edit-session.md) | Desktop: the session in the engine process, the native Edit menu, the close guard | E1d, D4b |
| [E1i](../../todos/ready/E1i-desktop-save-in-place.md) | Desktop: save in place, watcher suppression and the conflict dialog for outside changes | E1e, E1h, D3b |

E1b and E1g can start as soon as this plan merges, in parallel. The round's web and shared work, E1b to E1g, waits on no desktop todo.

## 10. Fixtures and tests

**Fixtures** (provenance and licence in `fixtures/README.md`):

- **XMP packets** from the producers that write most of the world's PDFs (E1b): Acrobat, Microsoft Word, LibreOffice, pdfTeX and Ghostscript, as small files made with each tool's free version or found in the existing corpus. A packet with properties in attribute form, one with several descriptions, one with an extension schema, one with no padding, and one invalid packet.
- **Signed files** made with pyHanko (E1c, `fixtures/signed/`): an approval signature, certification at P1, P2 and P3, two signatures, and a FieldMDP lock.
- **Conformance:** the PDF/A-2b, 2u and 3b and the PDF/UA-1 examples Lamina PDF already builds, and a PDF 2.0 file with metadata in XMP only (E1c).

**Tests per op** (`SetMetadata`, `RemoveMetadata`, the save stamp):

- **Unit:** the mapping table both ways; the patch semantics (absent, `null`, a value); the read precedence; the PDF 2.0 rule; each conformance warning.
- **Golden:** an unchanged packet round-trips byte for byte over the whole corpus; a title edit changes only the title on each producer's packet.
- **Round-trip:** each edit saved, reopened by Lamina PDF, pdf.js, PDFium and qpdf, which must report the new values; the capability tags `J.01`–`J.03` and `J.10` on these tests in `crates/laminapdf-save/tests/roundtrip_suite.rs` flip the matrix.
- **Conformance:** veraPDF passes on every edited PDF/A and PDF/UA fixture.
- **Signatures:** the planner's verdict for each signed fixture; an approval-signed file edited and saved keeps a valid signature in pyHanko's validator.
- **Differential:** values read against `exiftool -j` over the corpus (E1b, nightly lane) and against pypdf's metadata facts (E1c).
- **Fuzz:** a target over the XMP reader, 10 minutes in the nightly lane.
- **Undo:** each edit undone saves byte-identical to the original.
- **e2e (Playwright, three browsers):** rotate a page, undo and redo, checking what the page shows and the dirty indicator at each step. Edit the title, save and reopen. Check the strip on each signed fixture, and that Chromium saves in place while Firefox and WebKit download.
- **e2e (WebdriverIO, three OSes):** the desktop's session, Edit menu and close guard (E1h), and saving in place with outside changes (E1i), under `apps/desktop/e2e/specs/`.

No E1 op draws anything, so the appearance-stream requirement ([full-editor §5](../full-editor.md#5-requirements-that-hold-for-every-round), requirement 3) doesn't apply until E4.

## 11. Decisions on the open questions

- **How modes are typed:** two registries, as §5 says; later rounds register modes and tools without changing E1's code.
- **The XMP library:** our own reader and writer over `quick-xml`, as §4 says.
- **PDF 2.0 Info keys:** only `/CreationDate` and `/ModDate` are written in a PDF 2.0 file; the other entries are read and shown, marked as kept for older readers, and left as they were. An edit writes their fields to XMP.
- **Which side wins on read:** XMP in a PDF 2.0 or PDF/A file, otherwise the newer modification date, Info on a tie; a disagreement is reported.
- **Custom properties:** user-defined name and value pairs, which Acrobat keeps as extra Info keys mirrored under `pdfx:`, are not in E1. They belong with J.04, Manage XMP, in E3.
