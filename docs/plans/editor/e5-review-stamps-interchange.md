# E5: review, stamps and interchange

**Status:** Draft

The fifth editor round turns marks on a page into a review. People reply to comments and set each thread's status, filter and search them, place stamps and attach files, read audio comments, move comments in and out of the file as XFDF, FDF or Markdown, flatten comments into the page, and print a summary of every comment. This plan fixes the ops, the read and export functions behind the comments panel, how flattening uses the content append API, how the save planner gates each op, and the 9 pull requests that build the round.

Two parts are hard. Comments from other apps arrive as tangled threads, with replies to missing parents and cycles, and the panel must show them without losing any. Flattening is the round's one change to page content, so it is the one op a certification refuses at every level.

- **Round:** [full-editor §E5](../full-editor.md#e5-review-stamps-and-interchange), phase 3, milestone M6 Annotator
- **Feature-plan todo:** [E5a](../../todos/done/E5a-review-stamps-interchange-feature-plan.md)
- **Follows:** ADR [0005](../../decisions/0005-document-model.md) (the overlay and `LaminaPdfId`s), [0006](../../decisions/0006-op-log-and-undo.md) (ops and undo), [0007](../../decisions/0007-save-planner.md) (the save planner), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0024](../../decisions/0024-app-shell-ux.md) (modes and commands), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (jobs)
- **Sequence:** the thread model and the review ops first, then stamps and attachments, interchange, flattening and the summary side by side, then the interface; the desktop part and the round's close are their own todo ([full-editor §7](../full-editor.md#7-phases-and-rounds))
- **Builds on:** [E4](e4-markup-and-drawing-annotations.md) (the annotation core, `page_annotations`, the appearance generator and Comment mode), [E3](e3-navigation-attachments-inspection.md) (E3f's file-specification writer) and [E2](e2-page-organisation.md) (the content append API, E2e, and the object copier and blob store, E2p)
- **Design:** [C04 Comments panel](../../design/paper/comment-review.md#c04-comments-panel), [C05 Stamps and file attachments](../../design/paper/comment-review.md#c05-stamps-and-file-attachments), [C06 Import and export comments](../../design/paper/comment-review.md#c06-import-and-export-comments), [C07 Flatten and comment summary](../../design/paper/comment-review.md#c07-flatten-and-comment-summary), [C08 Phone annotate](../../design/paper/comment-review.md#c08-phone-annotate) (the comment sheet)

## 1. Where things stand

After E4, the library reads every annotation and makes the round's markup and drawings, but nothing reads a thread, a stamp or an exchange file.

- **Annotations.** E4b's `page_annotations` returns each annotation's `/IRT` and `/RT`, and `DeleteAnnot` removes replies and groups with their parent. Nothing reads `/StateModel` and `/State`, gathers replies into threads, or writes a reply.
- **Appearances.** E4b adds the `Appearances` trait and the builder's first Form XObjects; before it, the builder writes image XObjects only (`resources.rs`). No Stamp or FileAttachment annotation is written anywhere, and the standard stamp names (Approved, Draft and the rest) appear nowhere.
- **Files.** The disclosure scan records a FileAttachment's `/FS` as a holder of its embedded file (`disclosure/forms.rs`). The builder writes file specifications for new documents (`attach.rs`), and E3f's `AttachFile` writes them into open ones, leaving FileAttachment annotations to this round. Nothing reads a Sound annotation's samples.
- **Interchange.** Nothing reads or writes FDF or XFDF. `laminapdf-model` reads XML with quick-xml (`xmp/tree.rs`, which refuses a DTD) and escapes it (`xmp::escape`, `is_xml_char`).
- **Flattening.** The content append API exists ([E2e](../../todos/done/E2e-content-append-api.md)): `PageContentWriter` wraps a page's content, appends streams and registers XObjects under unused names. The extractor already picks an annotation's normal appearance through `/AS` and computes the §12.5.5 matrix (`paint/annots.rs`), but both functions are private to `laminapdf-extract`.
- **Building a report.** `Builder::flow` lays out headings, paragraphs, lists and tables over pages, tagged by default. A figure takes a raster image only, so drawing a page beside its comments needs a page turned into a Form XObject, which nothing does yet.
- **Fixtures.** No fixture holds a reply, a review state, a stamp, a file attachment, a sound or an exchange file. The pinned pdf.js corpus has stamps (`annotation-stamp`, `stamps`, `red_stamp`), attachments (`annotation-fileattachment`, `attachment`), popups (`highlight_popup`), rich text (`rc_annotation`) and media (`multimedia_annotations`); PDFium's has `annotation_stamp_with_ap` and `annotation_fileattachment`.
- **The matrix.** Every row in scope is in `docs/capabilities/manifest.toml` with `round = "E5"`; D.09 and D.12 already target `read`.

## 2. The operations

The round implements 9 rows of the [feature inventory](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms).

| ID | What it means | Ops or function (§2.1–§2.3) | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| D.07 | Custom stamps | `AddAnnot` with `Stamp` | round-trip | E5c · E5h · E5j |
| D.08 | Attach files as annotations | `AddAnnot` with `FileAttachment` | round-trip | E5c · E5h · E5j |
| D.09 | Audio annotations | `page_annotations`, `sound_as_wav` | read | E5c · E5h · E5j |
| D.10 | Replies to comments | `AddReply` | round-trip | E5b · E5g · E5j |
| D.11 | Status and review properties | `SetReviewState` | round-trip | E5b · E5g · E5j |
| D.12 | Filter and search comments | `comment_threads` | read | E5b · E5g · E5j |
| D.13 | Import and export annotation data | `import_annotations`, `export_annotations` | round-trip | E5d · E5i · E5j |
| D.14 | Flatten annotations into content | `FlattenAnnots` | round-trip | E5e · E5i · E5j |
| D.15 | Delete or summarise annotations | `delete_annotations`, `comment_summary` | round-trip | E5b, E5f · E5g, E5i · E5j |

D.09 stays `read`: playing an audio comment is reading it, and recording one is E17d's. D.12 is `read` because filtering changes nothing.

### 2.1 Ops

Ops follow E4's rules ([E4 plan §2.1](e4-markup-and-drawing-annotations.md#21-ops)): they name pages by `PageId` and annotations by `AnnotId`, carry every value they write (the `/NM`, the author, both dates and every coordinate), and take geometry in user space. Every op but flattening is class `Annotation`; none needs a full rewrite.

| Op | Payload | Class | Todo |
| --- | --- | --- | --- |
| `AddReply` | `{ id: AnnotId, nm, parent: AnnotId, author, created, contents, rich: Option<RichText> }` | `Annotation` | E5b |
| `SetReviewState` | `{ id: AnnotId, nm, target: AnnotId, model: Review \| Marked, state, author, created }`; `state` is `None`, `Accepted`, `Rejected`, `Cancelled` or `Completed` for `Review`, `Marked` or `Unmarked` for `Marked` | `Annotation` | E5b |
| `AddAnnot` with `Stamp` | `{ rect, rotation, stamp: Standard(StampName) \| Image(BlobId) \| Dynamic { lines: Vec<String>, style: StampStyle } }`; a dynamic stamp's lines come resolved in the payload ("Approved by A. Mensah, 12 Nov 2026") | `Annotation` | E5c |
| `AddAnnot` with `FileAttachment` | `{ at: UserSpacePoint, icon: AttachmentIcon, file: BlobId, name, mime, description, created, modified }` | `Annotation` | E5c |
| `FlattenAnnots` | `{ annots: Vec<AnnotId>, intent: View \| Print }`; the façade picks the IDs from C07's scope (all, a page, shown, selected) | `Content` | E5e |

**Replies and states are annotations of their own.** A reply is a Text annotation with `/IRT` pointing at its parent and `/RT /R`. A review state is another Text annotation with `/IRT` pointing at its target, `/StateModel` and `/State` (ISO 32000-2 §12.5.6.2, §12.5.6.4). Each change of status adds a new state annotation, so the thread keeps its history, as Acrobat's do. Both kinds take their parent's `/Rect`, carry an appearance that draws nothing, and leave every hiding flag clear, so a PDF/A file stays valid (ISO 19005-2 §6.3.2) and no viewer draws a second icon over the parent. E5b checks this against how pdf.js, PDFium and Acrobat show replies, using Acrobat-made reference files requested in [issue #524](https://github.com/laudebugs/laminapdf/issues/524). Editing a reply's text and deleting a reply or a state are E4b's `UpdateAnnot` and `DeleteAnnot`.

**Deleting several comments is one transaction.** C04's "Delete 3…" runs one `DeleteAnnot` per selected comment in one transaction, so it is one undo step, through the façade's `delete_annotations(ids)`. Since `DeleteAnnot` already removes replies and groups, the helper first drops every ID the cascade of another selected ID covers, so selecting a comment and its reply deletes both and never fails on the second. C04 names a `DeleteAnnots` op; the plan uses a helper instead, so the wire format keeps one way to delete.

**Stamps.** The eight standard stamps of C05 are drawn by E4's generator from their names (ISO 32000-2 §12.5.6.12 lists the names). Their lettering is drawn as paths, from outlines of the bundled Lamina Sans (Source Sans 3, OFL-1.1) generated into `laminapdf-build` at build time, so they need no font: they work in the base module and keep a PDF/A file's fonts embedded, and their `/Contents` carries the name as text. An image stamp draws the image from the blob store inside its Form XObject, and a dynamic stamp lays out its resolved lines with `laminapdf-text`. A dynamic stamp needs fonts, so it fails with `appearance-needs-text` where FreeText does ([E4 plan §2.2](e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated)).

**File attachments.** The annotation's `/FS` is a file specification written by E3f's writer, with the embedded file's bytes from the blob store, so `DeleteAttachment` and the Attachments tab see it as one more holder of the file. On a file that claims PDF/A-1 or PDF/A-2, attaching raises E3's `pdfa-attachment-forbidden`. On PDF/A-3, the annotation's file specification also goes into the annotation's `/AF` array with `/AFRelationship /Unspecified` and the payload's modified date, which ISO 19005-3 §6.8 requires; without a modified date the op raises E3's `pdfa-attachment-incomplete`.

**Flattening.** For each annotation, `FlattenAnnots` picks the normal appearance (through `/AS` for a state dictionary), wraps the page's content once (`wrap_existing`), registers the appearance as a Form XObject (`add_xobject`) and appends `q <A> cm /FmN Do Q`, with `A` the matrix of §12.5.5 that maps the appearance's box onto `/Rect` ([04 §2.5](../../research/pdf/04-interactive-security.md#25-flattening)). It then removes the annotation, its Popup and its replies. The rules:

- Annotations hidden for the chosen intent are skipped: Hidden, NoView for `View`, a missing Print flag for `Print`. So are those in optional content that is off.
- An annotation without an appearance gets one first from E4's generator when its subtype is one E4 draws; any other is left in place and reported (`flatten-no-appearance`).
- Widgets are refused (`flatten-widget`): flattening form fields is [E8g](../../todos/ready/E8g-form-data-and-flatten.md)'s, and a signature field is never flattened.
- In a tagged file the drawn appearance is marked as an artifact, and its `Annot` structure element is removed. On a file that claims PDF/UA-1, the op raises `flatten-untagged`, because the comment's text leaves the structure tree.
- The extractor's appearance and matrix functions move to `laminapdf-model`, so the extractor and the flattener share one copy.

### 2.2 Reading comments

The thread model lives in `laminapdf-model`; the quoted text needs text extraction, which `laminapdf-model` can't reach, so the façade's `comment_threads` adds it through `laminapdf-extract` (feature `extract`). `doc.comment_threads(pages)` returns the comments as threads: each top-level markup annotation with its replies in order and its latest state per model, its author, dates, quoted text (from the text under its quads, for text markup), note and kind. The model tolerates what other apps write. A reply to a missing parent, or a cycle of `/IRT`s, makes a thread of its own with a `broken` flag, which C04's notice reports; nothing is dropped. States given as strings or names both read (04 §2.3).

Filtering and search run in the interface over this list: by status, author, kind, page and date, and search over note, replies, quoted text and author. `sound_as_wav(id)` returns a Sound annotation's samples as a WAV file (raw, signed, μ-law and A-law samples at their `/R`, `/C`, `/B` and `/E`), within `Limits`.

### 2.3 Interchange and the summary

These change nothing in the open document. Long ones are jobs with a run-to-completion form that takes a `CancelToken` (ADR 0028 F10).

- **Export** (E5d): `export_annotations(&ExportOptions)` writes XFDF (ISO 19444-1), FDF (ISO 32000-2 §12.7.8) or Markdown, for every page or a range. XFDF and FDF carry the file's `/ID` and name (`<f href>`, `<ids>`, `/F`), so an import can warn about a different file. Markdown lists each page's comments with their quoted text, replies and status, which C06's Copy and preview use.
- **Import** (E5d): `import_annotations(bytes, &ImportOptions)` reads XFDF or FDF into one transaction of `AddAnnot`, `UpdateAnnot`, `DeleteAnnot`, `AddReply` and `SetReviewState`, so an import is one undo step. Its three modes come from C06. Merge, the default, updates annotations matched by `/NM` on the same page and adds the rest. Replace first deletes every comment on the pages the file names. Add makes everything new, with fresh names from the payload. Appearances are regenerated by E4's generator. Acrobat writes a Popup for every markup comment, so E5d adds an optional `popup` (its rectangle and `/Open`) to `AnnotCommon`, an additive change to the wire form, and an import keeps every Popup it reads, as the [E4 plan §11](e4-markup-and-drawing-annotations.md#11-decisions-on-the-open-questions) promised. A subtype no spec covers is skipped and listed (`interchange-skipped`), and so are form fields, which are E8g's. A file whose `/ID` or name differs raises `interchange-document-mismatch`, which C06 shows as its warning.
- **Parsing exchange files safely.** XFDF is read with the same guarded quick-xml reader as XMP, which refuses a DTD and every entity but XML's five, within `Limits` on size, depth and count. That reader is private to `xmp/tree.rs` today, so E5d moves it to a public `laminapdf_model::xml` module that XMP and XFDF both use, keeping one home for the guards. FDF is PDF syntax, so `laminapdf-syntax`'s tolerant parser reads it. Both live in a new core crate, `laminapdf-interchange`, which E8g's form data reuses.
- **The summary** (E5f): `comment_summary(&doc, &SummaryOptions) -> Document` builds a new document with `Builder::flow`. The list layout gives each page a heading, then each comment with a number, its kind, author, date, status, quoted text, note and replies. The layout beside its page, C07's other choice and the default, draws each page small at the left, with its comments' numbers marked on it and the comments beside it. The page is copied as a Form XObject by `ObjectCopier` (E2p), so the summary stays vector and selectable. The summary is tagged and passes the PDF/UA-1 profile; each drawn page is a `Figure` whose alternative text names the page.

## 3. Conformance the edits must keep

- **Tagged PDF and PDF/UA-1.** Replies, states, stamps and attachments follow E4's rules: an `Annot` element with an `OBJR`, `/Contents` as the alternative description, and `/Tabs /S` ([E4 plan §3](e4-markup-and-drawing-annotations.md#3-conformance-the-edits-must-keep)). A stamp's `/Contents` defaults to its name or its lines, and an attachment's to its file name. Flattening removes elements and raises `flatten-untagged` (§2.1).
- **PDF/A.** Replies and states keep every hiding flag clear and draw nothing (§2.1). Stamps and attachments meet E4's flag and colour rules. Attaching on PDF/A-1 or -2 raises `pdfa-attachment-forbidden`. A flattened appearance keeps its colours and fonts, which already passed when the annotation was written. An appearance from another app may break the file's claim, though, so after flattening a PDF/A file the op re-checks the fonts it adds to the page and raises `font-not-embedded` for any that aren't embedded.
- **No attachment runs.** Saving an attachment out writes its exact bytes and never opens it; a program gets O14's warning first, as in E3.
- **Flattening keeps the comments in earlier revisions.** An incremental save keeps the annotations in the previous revision, so `FlattenAnnots` sets `still_in_earlier_revisions`, and C07's "remove from history" asks for a full rewrite.

## 4. Crate and package changes

- **`laminapdf-model`**: the thread model and `comment_threads`, the state reader, the Sound reader and `sound_as_wav`, and the appearance and matrix functions moved from `laminapdf-extract` (E5b, E5c, E5e).
- **`laminapdf-ops`**: `AddReply`, `SetReviewState`, the `Stamp` and `FileAttachment` specs, and `FlattenAnnots` (E5b, E5c, E5e).
- **`laminapdf-build`**: stamp appearances (E5c); the summary and pages as Form XObjects (E5f).
- **`laminapdf-interchange`** (new, core): XFDF and FDF readers and writers, Markdown export, and the import planner that turns an exchange file into ops (E5d).
- **The façade and bindings.** Each op and function is on `laminapdf::Document` and `@laminapdf/sdk`'s `Document`, with docs, a runnable example in Rust and TypeScript, tsify types and a frozen sample in `fixtures/ops/v1/` (requirement 7). In TypeScript they join `doc.annotations`: `threads`, `reply`, `setStatus`, `deleteMany`, `flatten`, `import`, `export` and `summary`.
- **`laminapdf-cli`**: `laminapdf comments export` and `import` are worth adding to [T6a](../../todos/ready/T6a-cli-page-commands.md)'s family later; this round only exposes the functions.
- **`@laminapdf/ui`**: the Comments panel and the phone's comment sheet (E5g); the stamp picker, the custom stamp library and the attachment inspector (E5h); the import, export, flatten and summary dialogs (E5i).

## 5. Saving and the save constraint

Every save plans first, and the strip says what saving will do ([E1 plan §7](e1-editing-foundation.md#7-saving-and-the-save-constraint)). The round's ops are class `Annotation`, except `FlattenAnnots`, which is `Content`. DocMDP P3 permits annotations but no content change; P1 and P2 permit neither (ISO 32000-2 §12.8.2.2).

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update; flattening too, with the comments kept in the earlier revision | Nothing; after flattening or deleting, the note that the comments stay in earlier versions until Save as optimised |
| `AppendOnly`: approval signatures | Signed, not certified | An incremental update for every op; flattening keeps the signatures valid but changes what the signed pages show | The status-bar note; before flattening, C07's note that signature panels will report changed pages |
| `AppendOnly` with a FieldMDP lock | Locked form fields | An incremental update: no E5 op touches a field or widget, and flattening refuses widgets | The note |
| `AppendOnly` with DocMDP P3 | Certified, annotations allowed | Every `Annotation` op saves incrementally; `FlattenAnnots` is a violation | C07's "Flatten in a copy…"; Apply anyway… makes the constraint `Conflict` |
| `AppendOnly` with DocMDP P1 or P2 | Certified | Every op violates | The banner from the moment Comment mode opens (C01) |
| Usage rights (UR3) | Extended by Adobe Reader rights | `Annotation` ops where `/Annots` grants each op's right, as E4b checks; imports need `/Create`, `/Modify` and `/Delete` for what they do; flattening is a violation | The note, or the banner |
| `RequiresRewrite` | A repaired file, or C07's "remove from history" | A full rewrite, every old revision dropped | A neutral strip saying why; "remove from history" on a signed file needs the explicit override (`with_invalidate_signatures`) |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11 | The warning strip |
| `Conflict` | Ops in the session the constraint forbids | Save is refused; Save as invalidating the signatures, after a confirmation | The danger strip with the conflicting changes and a way to undo them |

The comment summary and every export write new files and work on any document, signed or encrypted once opened.

## 6. Differences between the shells

The ops, the panel and the dialogs are the same on both shells; the differences are where files come from and go, and both use existing capability flags ([ADR 0010](../../decisions/0010-platform-adapter.md)).

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Custom stamps | none: platform storage | Kept in IndexedDB on this device | Kept in the app-data folder |
| Attach a file, import XFDF or FDF | none: the file picker | The picker, or a drop | The dialog plugin, or a drop |
| Save an attachment, export comments | `canSaveAsDialog` | A save dialog in Chromium, else a download | A save dialog |
| Play an audio comment | none | `sound_as_wav` in an `<audio>` element | The same in the webview |
| Open the summary | `multiWindow` | A new tab | A new window |
| Edit session and save | as E1 | The document worker | The engine process |

C05 draws Play for the desktop only, with "needs the desktop app" on the web. Because `sound_as_wav` turns every sample format PDF allows into WAV, which every browser plays, the plan offers Play on both shells (a recommendation, §10).

No server feature: comments, stamps and exports stay on the device ([ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md)). Sharing a review with other people is E18's.

## 7. Todos

Each todo is one reviewable pull request of at most L (400–800 changed lines) on the [todo template](../../todos/TEMPLATE.md)'s scale, and each says why its size fits. The core todos flip the `core` and `pdf` columns, the interface todos `web`, and E5j `desktop`.

| ID | Todo | Depends on |
| --- | --- | --- |
| [E5b](../../todos/ready/E5b-comment-threads-and-review.md) | The thread model, `comment_threads`, `AddReply`, `SetReviewState` and `delete_annotations` | E4b |
| [E5c](../../todos/ready/E5c-stamps-and-attachments.md) | Stamps, file-attachment annotations, and audio comments read | E3f, E4b, E4e |
| [E5d](../../todos/ready/E5d-xfdf-fdf.md) | `laminapdf-interchange`: XFDF, FDF and Markdown export, and import | E4c, E4d, E5b, E5c |
| [E5e](../../todos/ready/E5e-flatten-and-summary.md) | `FlattenAnnots` | E2e, E4c, E4d |
| [E5f](../../todos/ready/E5f-comment-summary.md) | The comment summary in both layouts, and pages as Form XObjects | E2p, E5b |
| [E5g](../../todos/ready/E5g-comments-panel.md) | The Comments panel and the phone's comment sheet | E4f, E4k, E5b |
| [E5h](../../todos/ready/E5h-stamps-and-attachments-ui.md) | The stamp picker, custom stamps, the attach-file tool, the attachment inspector and Play | E4f, E5c |
| [E5i](../../todos/ready/E5i-interchange-flatten-summary-ui.md) | The import, export, flatten and summary dialogs | E5d, E5e, E5f, E5g |
| [E5j](../../todos/ready/E5j-desktop-review.md) | Desktop: the round in the engine, native dialogs, WebdriverIO suites; the round's close | E4h, E5g, E5h, E5i |

Every todo also depends on this plan (E5a). E5b can start once E4b merges, and E5e once E4c and E4d have given every E4 subtype an appearance generator. E5c waits on E3f's file-specification writer and E4e's text layout for dynamic stamps.

## 8. Fixtures

Every new fixture is written by Lamina PDF's own scripts and licensed like the rest of Lamina PDF, `MIT OR Apache-2.0`; provenance goes in `fixtures/README.md`.

- **`fixtures/annotations/`** (E4b's folder) gains, from `annotations.py`: `review.pdf` (threads with replies, states of both models given as names and as strings, a reply to a missing parent and an `/IRT` cycle), `stamps.pdf` (the standard names, an image stamp and a rotated one), `attachments.pdf` (FileAttachment annotations with each icon, one sharing its file with the `/EmbeddedFiles` tree), `sound.pdf` (a Sound annotation in each sample format) and `flatten.pdf` (annotations with state appearances, hidden and print-only ones, one in optional content, one without an appearance, a widget, and a tagged page).
- **`fixtures/interchange/`** (new, E5d): XFDF and FDF files written by `interchange.py` for every subtype the specs cover, with replies and states, a file for another document, a malformed file with a known line number, and an XFDF with a DTD that must be refused. Acrobat-made exports join them when the owner supplies them ([issue #524](https://github.com/laudebugs/laminapdf/issues/524)).
- **The pinned corpus:** the pdf.js and PDFium stamp, attachment, popup and media files listed in §1, for reading and for the flatten corpus test.
- **Signed files:** the pyHanko files in `fixtures/signed/`, with the usage-rights file E2i adds.

## 9. Tests

Each op gets the tests its todo lists ([ADR 0018](../../decisions/0018-test-strategy.md)). "Round-trip" means Lamina PDF, pdf.js, PDFium and qpdf (`--check` clean) each reopen the saved file and report the change and nothing else.

| Op or function | Unit | Golden | Round-trip | Differential | Fuzz | e2e |
| --- | --- | --- | --- | --- | --- | --- |
| Threads, replies, states (E5b) | `review.pdf`'s threads, broken ones included; states as names and strings | — | pdf.js's `getAnnotations` reads each reply's `inReplyTo` and state | thread counts vs pdf.js over the corpus | `ops_json` gains the ops | — |
| Stamps, attachments, sound (E5c) | every standard name; image and dynamic stamps; WAV headers | `fixtures/golden/annots/stamps.pdf` | pdf.js and PDFium draw each stamp within E4g's threshold; pdf.js's `getAttachments` returns the attached bytes | — | `ops_json` | — |
| Interchange (E5d) | each mode; mismatch; skipped subtypes; DTD refused | the XFDF and FDF of `review.pdf` | export, import into a copy, export again: the same file | — | new `xfdf` and `fdf` targets | — |
| `FlattenAnnots` (E5e) | intents, flags, optional content, widgets refused, tagged pages | `flatten.pdf` flattened | PDFium's render of the flattened page matches its render of the annotated page within E4g's threshold | the pdf.js corpus's annotated files flattened and rendered | `save_roundtrip` gains the op | — |
| Summary (E5f) | both layouts, numbering, empty pages | the summary of `review.pdf` | veraPDF passes it as PDF/UA-1 | — | — | — |
| Interface (E5g–E5i) | Vitest for filters, search and the dialogs' state | — | — | — | — | Playwright per command, tagged with the row's ID |

Across every op: apply then undo leaves the overlay empty and the next save byte-identical (ADR 0006 §3); the same state saves to the same bytes on native and wasm32; `would_violate` on every signed fixture gives the verdicts of §5, asserted per op; veraPDF reports nothing new on the tagged and PDF/A fixtures. A new fuzz target's `fixtures/regressions/<target>` folder joins `laminapdf-model`'s regression guard list.

## 10. Decisions on the open questions

These are the round's defaults, and the owner is asked to confirm them in [issue #524](https://github.com/laudebugs/laminapdf/issues/524); the todos follow any change.

- **Deleting a parent deletes its replies.** E4 already decided it: `DeleteAnnot` removes the replies and the group with the parent, and C04's dialog says "Delete 3 comments and their replies?". Keeping orphaned replies would make broken threads the next reader has to make sense of.
- **The summary offers both layouts and is tagged.** Beside its page is the default, as C07 lists it first and Acrobat's summary looks; the list layout is for long reviews. Both are PDF/UA-1, since a summary is a document people read and send on. The page beside its comments is a vector copy, not a picture.
- **Exchange files live in their own crate.** `laminapdf-interchange` holds XFDF, FDF and Markdown, so E8g's form data shares its readers and the model crate stays free of XFDF's rules.
- **An import is a transaction of ordinary ops.** A composite `ImportAnnotations` op would need its own inverse and wire form, while the existing ops already undo and replay. Replace mode deletes before it adds, in the same transaction.
- **Play works on both shells.** `sound_as_wav` makes every PDF sample format playable in a browser, so the web plays audio comments too. This changes C05, which the design area records.
- **Markdown export belongs to E5d.** C06 offers it and no todo owned it.
- **The plan's location:** `docs/plans/editor/`, as E1–E4 use.
