# E3: navigation, attachments, inspection and extraction

**Status:** Draft

The third editor round lets people change how a document is navigated and what it carries, and take content out of it. They can edit bookmarks, named destinations and links, make a contents page, and set the view the file opens in. They can manage the XMP packet and attached files, look inside the file's objects, fonts and claims, and export pages, text, images, links and bookmarks. This plan fixes the ops, the functions behind the exports, how the save planner gates each op, and the 11 pull requests that build the round.

The hard part is keeping each edit valid in a file that is tagged, signed or claims PDF/A. Most edits are small dictionary changes, and most exports only read. The plan calls the PDF's tree of bookmarks the outline and its entries bookmarks; in the ops and types, a bookmark is an outline item (ISO 32000-2 §12.3.3).

- **Round:** [full-editor §E3](../full-editor.md#e3-navigation-attachments-inspection-and-extraction), phase 2, milestone M5 Organiser
- **Feature-plan todo:** [E3a](../../todos/done/E3a-navigation-attachments-inspection-feature-plan.md)
- **Follows:** ADR [0005](../../decisions/0005-document-model.md) (the overlay and `LaminaPdfId`s), [0006](../../decisions/0006-op-log-and-undo.md) (ops and undo), [0007](../../decisions/0007-save-planner.md) (the save planner), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (façade rules and jobs), [0030](../../decisions/0030-extraction-output-and-model-plugins.md) (extraction output), [0031](../../decisions/0031-cli-and-mcp-contract.md) (versioned JSON)
- **Sequence:** five todos start as soon as this plan merges; the bookmark, link, attachment and contents-page todos wait on E2's writers and blob store; the desktop part is its own todo that waits on the desktop track ([full-editor §7](../full-editor.md#7-phases-and-rounds))
- **Builds on:** [E2](e2-page-organisation.md): its tree writers and page reference index (E2j), its blob store and source documents (E2p), its cross-document pages (E2c) and its folder output (E2m)
- **Design:** [O04 Document info and XMP](../../design/paper/edit-session-organise.md#o04-document-info-and-xmp-editor), [O11 Bookmarks](../../design/paper/pages-navigation-extraction.md#o11-bookmarks-editor), [O12 Links](../../design/paper/pages-navigation-extraction.md#o12-link-editor), [O13 Initial view and threads](../../design/paper/pages-navigation-extraction.md#o13-initial-view-and-article-threads), [O14 Attachments](../../design/paper/pages-navigation-extraction.md#o14-attachments-panel), [O15 Object inspector](../../design/paper/pages-navigation-extraction.md#o15-object-inspector), [O16 Export and extraction](../../design/paper/pages-navigation-extraction.md#o16-export-and-extraction)

## 1. Where things stand

The library reads most of what E3 edits and the builder writes most of it into new documents, but no op changes any of it in an open file.

- **Outline and destinations.** `laminapdf-model` reads the outline (`outline.rs`: `/First`, `/Next`, `/Count`, a `/Dest` or a `GoTo`'s `/D`, other actions as `ActionKind`), but not an item's colour `/C` or style flags `/F`. Named destinations are read from the `/Names /Dests` tree, then the PDF 1.1 catalog `/Dests` dictionary; `tree.rs` reads name and number trees. The façade's `outline()` and `named_destination()` serve the viewer. The builder writes outlines with colour and flags, and a one-leaf destinations tree. No name-tree writer exists for open documents: E2j adds it.
- **Links.** `link.rs` reads Link annotations: the rectangle, `/QuadPoints` (else the rectangle), a `/Dest` or an action (`GoTo`, `URI`, `GoToR`, others). The viewer opens only `http:`, `https:` and `mailto:` links (`link-url.ts`). The builder draws links, tagged as `Link` elements with an `OBJR` in tagged documents.
- **Initial view and threads.** The builder writes `/PageMode`, `/PageLayout` and `/ViewerPreferences` (`meta/prefs.rs`). Nothing reads them, and the viewer ignores them on open. `/OpenAction` is read only by the disclosure scan, as a trigger. Nothing reads `/Threads`.
- **XMP.** `xmp::Packet` parses every property in any namespace and reads them back (`properties()`, `property(ns, name)`), but sets only the 11 Info-mapped fields of `MAPPINGS`. The PDF/A and PDF/UA identification properties are read-only getters. The builder writes one fixed PDF/A extension schema, for `pdfuaid:part`.
- **Attachments.** `Builder::attach` writes an embedded file (`/Subtype` the MIME type, `/Params` with `/Size` and `/ModDate`), its file specification with `/F`, `/UF` and `/AFRelationship`, the `/EmbeddedFiles` tree and the catalog `/AF`. The disclosure scan lists every embedded file with its holders (the tree, `/AF` arrays, FileAttachment annotations), and `laminapdf reveal` reports them. The scan follows `/AF` on every dictionary, pages included, but only to find files; nothing returns an attachment's bytes, and nothing reads `/Collection` or `/CheckSum`.
- **Inspection.** XMP gives the PDF/A and PDF/UA claims. `Pages::fonts()` lists the fonts of a set of pages. `laminapdf_cos::Document::revision_index()` gives each revision's xref section and the objects it changed or freed (T2c). No public API iterates objects, reads output intents or ICC profiles, or says which objects refer to an object.
- **Rendering and export.** `laminapdf-render` draws tiles into RGBA buffers through hayro or PDFium. `laminapdf render` writes PNG through the `png` crate (MIT OR Apache-2.0); no JPEG or TIFF encoder is in the tree. On the web, the viewer's print path encodes PNG with `OffscreenCanvas.convertToBlob`.
- **Images and extraction.** The builder places JPEG (passed through), PNG (its own decoder, alpha as `/SMask`) and raw RGBA images; it ignores EXIF orientation, and no function makes one page per image. `laminapdf-extract` gives plain text, Markdown and `laminapdf.extract/1` JSON with positions, through the façade, the SDK and `laminapdf extract`. Nothing walks a page's images to export their bytes, and no export writes links or bookmarks.
- **Ops and saving.** The op set is `SetInfo`, `RotatePage`, `SetMetadata`, `RemoveMetadata` and the planner's `UpdateSaveMetadata`; E2 adds the page ops. The blob store and source documents exist: `Document::add_source` and `BlobStore` ([E2p](../../todos/done/E2p-source-documents-and-copier.md)). The planner lets DocMDP P2 permit form filling and signing, P3 also annotations, and UR3 only the rights it grants; `Metadata`, `Document`, `PageStructure` and `Content` are permitted under no certification (`crates/laminapdf-save/src/constraint.rs`, `detect.rs`).
- **The matrix.** Every row in scope is in `docs/capabilities/manifest.toml` with `round = "E3"`. The builder's tagged tests already prove `I.02`, `I.03`, `I.04`, `A.09` and `J.04` at core `write`, and the reading tests `I.01` and `A.10` at core `read`.

## 2. The operations

The round implements 24 rows of the [feature inventory](../../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms).

| ID | What it means | Ops or function (§2.1–§2.3) | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| I.01 | Create and edit bookmarks; nested trees | the outline ops | round-trip | E3b · E3b · E3k |
| I.02 | Internal page links | `AddLink`, `UpdateLink`, `DeleteLink` | round-trip | E3c · E3c · E3k |
| I.03 | External hyperlinks | the link ops | round-trip | E3c · E3c · E3k |
| I.04 | Named destinations | the named-destination ops | round-trip | E3b · E3b · E3k |
| I.05 | Clickable table of contents | `create_contents_page` | round-trip | E3l · E3l · E3k |
| I.06 | Initial view and zoom; page display mode | `SetInitialView` | round-trip | E3d · E3d · E3k |
| I.09 | Article threads | `threads()` | read | E3d · E3d · E3k |
| I.11 | Outline and navigation preferences | `SetInitialView` | round-trip | E3d · E3d · E3k |
| A.09 | Add hyperlinks to text | `AddLink` with quads | round-trip | E3c · E3c · E3k |
| J.04 | Manage XMP | the XMP ops | round-trip | E3e · E3e · E3k |
| J.05 | Attach, extract and delete embedded files | the attachment ops, `attachment_bytes` | round-trip | E3f · E3f · E3k |
| J.06 | Portfolios and collections | `collection()` | read | E3f · E3f · E3k |
| M.09 | Associated files and structured metadata | `AttachFile` with an `/AF` target | round-trip | E3f · E3f · E3k |
| J.07 | Inspect version and conformance | `conformance()` | read | E3g · E3g · E3k |
| J.08 | Identify fonts and colour profiles | `fonts()`, `color_profiles()` | read | E3g · E3g · E3k |
| J.09 | Inspect internal objects, streams, references | `objects()`, `object()`, `references_to()`, `revisions()` | read | E3g · E3g · E3k |
| H.02 | Images → PDF | `images_to_pdf` | write | E3h · E3j · E3k |
| H.06 | Pages → PNG, JPEG and TIFF | `export_pages` | read | E3h · E3j · E3k |
| H.11 | Render pages to images | `export_pages` | read | E3h · E3j · E3k |
| A.10 | Extract text with position and formatting | `laminapdf-extract` (exists) | read | E3i (`pdf`; core is proved) · E3j · E3k |
| H.07 | Extract text and images | `laminapdf-extract`, `extract_images` | read | E3i · E3j · E3k |
| B.03 | Extract original embedded images | `extract_images` | read | E3i · E3j · E3k |
| H.08 | Extract hyperlinks and bookmarks | `links()`, `outline()` export | read | E3i · E3j · E3k |
| H.09 | Extract attachments | `attachment_bytes` | read | E3f · E3f · E3k |

Each op's class is in §2.1, and no op of the round needs a full rewrite (`requires_rewrite` is false for all of them). The export and extraction rows target `read`: they take content out of the file and change nothing, and the matrix defines `read` as "displayed or extracted faithfully". The matrix keeps `write` for rows that produce a valid PDF, so only `H.02` (images → PDF) targets `write`.

`I.11`, navigation preferences, is `/ViewerPreferences` and `/PageMode`, which `SetInitialView` writes, so it moves from E3b to E3d.

### 2.1 Ops

Every edit of the open document is an `Op` ([full-editor §5](../full-editor.md#5-requirements-that-hold-for-every-round), requirement 1). Ops name pages by `PageId`, outline items by `OutlineItemId` and annotations by `AnnotId`, and carry every value they write: a link's `/NM`, an attachment's dates and every rectangle come in the payload, never from a clock or the view. Rectangles and quads are `UserSpaceRect` and `UserSpaceQuad`, so a later rotation or crop of the page leaves the op's meaning unchanged; the façade takes page space and converts (ADR 0027). An optional field is absent to leave a value alone and `null` to remove it.

| Op | Payload | Class | Todo |
| --- | --- | --- | --- |
| `AddOutlineItem` | `{ parent: Option<OutlineItemId>, at: SiblingPosition, title, target: OutlineTarget, style: OutlineStyle, open: bool }`; `OutlineTarget` is a `Destination { page: PageId, view: View }`, a named destination or a URI; `OutlineStyle { color: Option<Rgb>, bold, italic }` | `Document` | E3b |
| `UpdateOutlineItem` | `{ item, title?, target?, style?, open? }`; a target the item had as another action (JavaScript, `Launch`) is replaced only when `target` is given | `Document` | E3b |
| `MoveOutlineItem` | `{ item, parent: Option<OutlineItemId>, at: SiblingPosition }`; the item keeps its children and its ID | `Document` | E3b |
| `DeleteOutlineItem` | `{ item, children: delete \| promote }` | `Document` | E3b |
| `SetNamedDestination` | `{ name, destination: Destination }`: adds the name, or retargets it | `Document` | E3b |
| `RenameNamedDestination` | `{ from, to, links: keep \| follow }`: `follow` (the default) also renames the uses in this file's outline items, links and `GoTo` actions | `Document` | E3b |
| `DeleteNamedDestination` | `{ name }`; uses in this file are reported, not removed | `Document` | E3b |
| `AddLink` | `{ id: AnnotId, nm, page: PageId, rect: UserSpaceRect, quads: Vec<UserSpaceQuad>, target: LinkTarget, contents: Option<String> }`; `LinkTarget` is a `Destination`, a named destination or a URI | `Annotation` | E3c |
| `UpdateLink` | `{ link, rect?, quads?, target?, contents? }` | `Annotation` | E3c |
| `DeleteLink` | `{ link }` | `Annotation` | E3c |
| `SetInitialView` | `{ open: Option<OpenView>?, page_mode?, page_layout?, preferences?: ViewerPreferences }`; `OpenView { page: PageId, view: View }` | `Document` | E3d |
| `SetXmpProperty` | `{ namespace, prefix, path: XmpPath, value: XmpValue }`; `XmpValue` is text, a URI, an integer, a real, a boolean, a date, a language alternative, or an ordered or unordered array of those | `Metadata` | E3e |
| `RemoveXmpProperty` | `{ namespace, path }` | `Metadata` | E3e |
| `ReplaceXmpPacket` | `{ xml: String }`: the whole packet, parsed before it applies and refused if it doesn't parse | `Metadata` | E3e |
| `AttachFile` | `{ name, file: BlobId, mime, description?, created?, modified?, attach_to: AttachTarget, relationship: AfRelationship }`; `AttachTarget` is the `/EmbeddedFiles` tree only, or also the `/AF` of the catalog, a page or an annotation | `Document` | E3f |
| `UpdateAttachment` | `{ attachment: AttachmentId, name?, description?, mime?, relationship? }` | `Document` | E3f |
| `DeleteAttachment` | `{ attachment }`: removes it from the tree and from every `/AF` array that names it | `Document` | E3f |

Three types are new: `AttachmentId`, a `LaminaPdfId` kind for a file specification; `UserSpaceQuad`, beside `UserSpaceRect` in `laminapdf-model`; and `SiblingPosition` (`first`, `last`, `{ before }`, `{ after }`), the outline's counterpart of E2's `PagePosition`. `AddLink` and its siblings are the only way to make a link: when E4b adds the generic annotation ops, they wrap these and keep their wire form (E3c).

**Rules every op keeps:**

- **The outline.** The writer keeps the doubly linked list whole after each op: `/First`, `/Last`, `/Next`, `/Prev` and `/Parent`. Each `/Count` is the number of visible descendants for an open item, and minus the number that would show if opened for a closed one (§12.3.3). An outline read with repairs (a cycle, a wrong `/Last`) is rewritten whole by the first op that touches it, and the op reports `OutlineRepaired`.
- **Named destinations.** A name stays in the form it was found in: a string in the `/Names /Dests` tree, written through E2j's writer, or a name object in the catalog's `/Dests` dictionary, which is the only place a name object resolves (§12.3.2.4). New names go into the tree, as the builder already writes them. Moving a name between the two would break every use that names it in the other form.
- **Links.** A URI must be absolute, with the scheme `http`, `https` or `mailto`; anything else, `javascript:` first, fails with `link-uri-refused`. A link on text gets `/QuadPoints` from the glyph quads of the selection and a `/Rect` that bounds them (§12.5.6.5). In a tagged document the link becomes a `Link` structure element with an `OBJR`, a `/StructParent` and a `/ParentTree` entry. Its parent is the element that owns the marked content under its quads, else the document element, and a link that can't be placed raises `link-not-tagged` on a file that claims PDF/UA-1. These structure entries go with the annotation, so the op stays `Annotation`; whether a certifier's P3 check accepts them is E3c's to test (§6).
- **XMP.** A property of `MAPPINGS` goes through E1's metadata writer, so Info stays in step. The `pdfaid:` and `pdfuaid:` properties are refused (`xmp-claim-read-only`), and so is a `ReplaceXmpPacket` that changes them (plan §11). On a file that claims PDF/A, a property outside the schemas PDF/A predefines gets a generated `pdfaExtension:schemas` entry (ISO 19005-2 §6.6.2.3); `ReplaceXmpPacket` on such a file adds the entries its custom properties lack, and reports them. Struct-valued properties stay readable and editable only through the whole packet.
- **Attachments.** The file specification has `/F` and `/UF`, the embedded file `/Subtype` (the MIME type) and `/Params` with `/Size`, the dates given and an MD5 `/CheckSum` (§7.11.4). On a file that claims PDF/A-1 or PDF/A-2, attaching raises `pdfa-attachment-forbidden`: PDF/A-1 allows no embedded files and PDF/A-2 only PDF/A ones, which the op can't confirm (ISO 19005-2 §6.8). On PDF/A-3, an attachment without an `/AF` target or a `modified` date raises `pdfa-attachment-incomplete`, since ISO 19005-3 §6.8 requires both. Both warnings, like E1's conformance warnings, never block the op. The ops can't reuse the builder's `embedded-file-forbidden`, which is an error that refuses a build, so they report their own codes in `opWarnings`.
- **Attachments held by annotations.** `DeleteAttachment` removes the file from the tree and from every `/AF` array; a FileAttachment annotation that holds the same file specification keeps it, and the op reports `AttachmentStillHeld` with the annotation. Removing the annotation is an annotation op (E4), so the class stays `Document`.
- **Removed things stay in earlier revisions.** `DeleteAttachment`, `RemoveXmpProperty` and `ReplaceXmpPacket` set `Edited::still_in_earlier_revisions`, and the interface says so, as it does for metadata (E1).
- **Undo.** Each op has an inverse; apply then undo leaves the overlay empty (ADR 0006 §3).

### 2.2 The contents page

A contents page is not an op: an op would have to draw text inside `laminapdf-ops`, which can't depend on the builder. Instead, `create_contents_page(&mut doc, &ContentsOptions)` runs one transaction of existing ops, so it is one undo step, as O11 wants. First it builds the page or pages as a new document with `laminapdf-build`: titles, dot leaders, and the page labels readers will see after the insert. Then it adds that document as a source (E2p), inserts its pages with `InsertPagesFrom` (E2c), adds an `AddLink` per entry and, when asked, a bookmark for the contents page. The op log holds those ops and the source's bytes, so a replay draws the same page on every device.

On a tagged file the page is built tagged (`TOC` and `TOCI` elements) and joins the structure tree through E2o. `ContentsOptions` names the font. The app passes its bundled font, and the library's default, standard 14 Helvetica, raises `font-not-embedded` on a file that claims PDF/A or PDF/UA-1.

### 2.3 Functions that only read or write new files

These change nothing in the open document; long ones are jobs with a run-to-completion form that takes a `CancelToken` (ADR 0028 F10).

- **Inspection** (E3g): `doc.conformance()` (header and catalog versions, the claims of `pdfaid:`, `pdfuaid:` and PDF/X's `GTS_PDFXVersion`, each "claimed, not verified"), `doc.fonts(pages)` (type, embedded or subset, encoding, ToUnicode coverage), `doc.color_profiles()` (output intents and `ICCBased` spaces with each profile's header), `doc.objects()` and `doc.object(ref)` (any object, a stream raw or decoded within `Limits`), `doc.references_to(ref)` (built on first use, then kept), and `doc.revisions()` over `revision_index()`.
- **Export** (E3h): `images_to_pdf(images, &ImagesOptions) -> Document`, one page per image: JPEG passed through, PNG through the builder, the EXIF orientation applied by the drawing transform, and the page sized from the image's resolution or fitted to a paper size. `export_pages(&render_doc, pages, &ExportOptions)` encodes PNG, JPEG or TIFF at a resolution, one page at a time.
- **Extraction** (E3i): `extract_images(&doc, pages)` walks each page's image XObjects, inline images and nested forms; it passes DCT and JPX data through as `.jpg` and `.jp2`, and decodes the rest to PNG with `/Decode`, masks, `Indexed`, 16-bit and CMYK honoured (CMYK by a simple conversion until E16d's colour management). `doc.links()` and `doc.outline()` export as JSON (`laminapdf.links/1`, `laminapdf.outline/1`, schemas committed as ADR 0031 asks) and as CSV. Text comes from `laminapdf-extract`, as it does now.
- **Attachments** (E3f): `doc.attachments()` lists them with their holders and relationships; `attachment_bytes(id)` returns the file decoded, its size checked against `Limits` first, and its checksum compared when there is one; `doc.collection()` reads a portfolio's schema, sort and files.

## 3. Conformance the edits must keep

- **Tagged PDF and PDF/UA-1.** Links join the structure tree (§2.1). A contents page is tagged. A bookmark has no tag of its own, so editing bookmarks changes nothing that veraPDF checks. After each op, veraPDF must report nothing new on the tagged fixtures.
- **PDF/A.** Custom XMP properties get extension schemas; attachments raise their warnings; nothing edits a claim. A contents page in an embedded font keeps a PDF/A file's fonts embedded.
- **Links are safe.** No op writes a script link, and links read from a file with another scheme stay as they are and are flagged in the link editor, never followed (O12).
- **Version.** `/AF` is a PDF 2.0 key (ISO 32000-2 §14.13) that PDF/A-3 allows at PDF 1.7 (ISO 19005-3 §6.8). The ops write it at any version and never raise `/Version` (a recommendation): raising it would break a PDF/A-2 file's claim and change how E1's metadata ops write Info, and earlier readers ignore a key they don't know.
- **No attachment runs.** Saving out writes the file's exact bytes and never opens it; a program (by MIME type or extension) gets O14's warning first.

## 4. Crate and package changes

- **`laminapdf-model`**: the outline view reads `/C` and `/F`, and an outline writer over `CosMut` keeps the list and counts (E3b); `link.rs` resolves a URI against `/URI /Base`; a catalog view reads `/OpenAction` as a destination, `/PageMode`, `/PageLayout` and `/ViewerPreferences` (E3d), and `threads()` reads `/Threads` and their beads (E3d); `xmp::Packet` sets and removes any property, and generates extension schemas (E3e); file specifications, embedded files, `/AF` at every level and `/Collection` (E3f); output intents and ICC headers, and a reference index (E3g).
- **`laminapdf-ops`**: the ops of §2.1, `AttachmentId`, and `Applied`'s reports (`OutlineRepaired`, renamed destination uses, `still_in_earlier_revisions`).
- **`laminapdf-build`**: `images_to_pdf` and EXIF orientation (E3h); the contents page's layout (E3l).
- **`laminapdf-render`**: `export_pages` and the encoders (E3h): PNG through `png`, JPEG through `jpeg-encoder` (MIT OR Apache-2.0) and TIFF through `tiff` (MIT), each checked by cargo-deny.
- **`laminapdf-content` and `laminapdf-extract`**: the image walk and its decoders (E3i).
- **The façade and bindings.** Each op is a method on `laminapdf::Document` and on `@laminapdf/sdk`'s `Document`, with docs, a runnable example in Rust and TypeScript, tsify types and a frozen sample in `fixtures/ops/v1/` (requirement 7). In TypeScript they group as `doc.outline`, `doc.destinations`, `doc.links`, `doc.xmp` and `doc.attachments`, beside `doc.setInitialView`, `doc.createContentsPage` and the read-only functions of §2.3.
- **`@laminapdf/viewer`**: the worker applies a document's initial view on open (§5); an edit that changed the outline or the destinations tells the client so, and the outline and destination caches refresh; link edits redraw their page's link layer through `viewer.invalidate` (E1g).
- **`@laminapdf/ui`**: the editable Outline tab and named destinations dialog (E3b), the link tool and popover (E3c), the initial view dialog and the Threads tab (E3d), the XMP editor in Document properties (E3e), the Attachments tab (E3f), the object inspector (E3g), the contents page dialog (E3l) and the export dialog (E3j). Every command is in the command registry with its O-screen ID.

## 5. The viewer and the initial view

A file's initial view applies when it opens, before the first page draws, in the reader and the editor; a reading position the reader saved (T5b) wins over it, since that is where the person left off.

- **Page and zoom.** `/OpenAction` as a destination sets the first page and zoom through `goToDestination`. An `/OpenAction` that is another action (JavaScript, a named action) is never run; it is shown in the initial view dialog and kept, and editing it is E17b's.
- **Page layout.** `/SinglePage` and `/OneColumn` map to the viewer's single and continuous layouts. The two-page layouts map to its spreads, with the first page alone for `TwoPageRight` and `TwoColumnRight`. A layout the viewer lacks falls back to continuous, with a note in Document properties.
- **Page mode.** `/UseOutlines`, `/UseThumbs` and `/UseAttachments` open the rail on Outline, Pages or Attachments, `/FullScreen` starts focus mode, and `/UseOC` opens nothing until E17c's layers panel.
- **Preferences.** `/DisplayDocTitle` puts the title in the tab or window; `/HideToolbar`, `/HideMenubar` and `/HideWindowUI` are written and kept but not obeyed, since hiding the app's own chrome would strand the person. The dialog says so.

## 6. Saving and the save constraint

Every save plans first, and the strip says what saving will do ([E1 plan §7](e1-editing-foundation.md#7-saving-and-the-save-constraint)). The round's ops are `Document`, `Annotation` and `Metadata`; the contents page adds `PageStructure` through `InsertPagesFrom`. Under a certification, an E3 op is allowed only if it is a link op and the level is P3. DocMDP P1 permits no change, P2 permits form filling, page templates and signing, and P3 also permits annotations (ISO 32000-2 §12.8.2.2).

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update | Nothing; after deleting an attachment or XMP, the note that it stays in the file's earlier versions until Save as optimised |
| `AppendOnly`: approval signatures | Signed, not certified | An incremental update for every E3 op | The accent strip "Signed: your changes are added after the signatures" |
| `AppendOnly` with a FieldMDP lock | Locked form fields (§12.8.2.4, or a signature field's `/Lock`) | An incremental update: no E3 op changes a form field or a widget | The accent strip |
| `AppendOnly` with DocMDP P3 | Certified, annotations allowed | Link ops save incrementally; every other E3 op, the contents page included, is a violation | Before a violating op applies, `would_violate`'s verdict refuses it, with the banner naming the certification: Save a copy · Apply anyway… · Details |
| `AppendOnly` with DocMDP P1 or P2 | Certified | Every E3 op violates; applied through Apply anyway…, it makes the constraint `Conflict` | The same banner and confirmation |
| Usage rights (UR3) | Extended by Adobe Reader rights | Links where `/Annots` grants create, modify or delete (§12.8.2.3); attachment ops where `/EF` grants them, once E3f maps them; everything else is a violation | The same banner and confirmation |
| `RequiresRewrite` | A repaired file, or a new document (images → PDF) | A full rewrite | A neutral strip saying why; none for a new document |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11, as in E1 | The warning strip |
| `Conflict` | Ops in the session the constraint forbids | Save is refused; Save as invalidating the signatures, after a confirmation | The danger strip with the conflicting changes and a way to undo them |

**Usage rights for embedded files.** UR3 can also grant `/EF` rights to create, delete, modify and import embedded files (§12.8.2.3, Table 260), which the planner doesn't read yet: its classes can't tell an attachment from another `Document` change. E3f maps `/EF` rights to the attachment ops in `would_violate`, by op rather than by class; until then attachments count as violations under UR3, which is stricter than the standard.

**Tagged links under P3.** A link in a tagged file also adds a structure element and a `/ParentTree` entry. ISO 32000-2 permits annotations at P3 without saying whether their structure entries count, so E3c checks the certified-P3 fixture's saved file with pyHanko's DocMDP check. If pyHanko counts them as a change, E3c adds links untagged on certified files and raises `link-not-tagged`.

The save-time metadata stamp is `Metadata` too, so a certified file saves without it, as E1 decided. Inspection, export and extraction change nothing and work on every file, signed or encrypted once opened. The table corrects [full-editor §E3](../full-editor.md#e3-navigation-attachments-inspection-and-extraction), which said attachments may be added to signed files "where permitted": no certification level permits them.

## 7. Differences between the shells

The ops, panels and dialogs are the same on both shells; the differences are where files come from and where exports go, and both are capability flags ([ADR 0010](../../decisions/0010-platform-adapter.md)).

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Attach a file | none: both shells | The file picker, or a drop on the Attachments tab | The same, through the dialog plugin |
| Save an attachment out | `canSaveAsDialog` (exists) | A save dialog in Chromium, else a download | A save dialog |
| Export one file | `canSaveAsDialog` | A save dialog or a download | A save dialog |
| Export several files | `canWriteDirectory` (E2m) | A folder in Chromium; else a zip download, written as each page finishes | A folder, with Show in Finder or Explorer |
| Encode PNG and JPEG | none | The browser's canvas encoder in the document worker | `png` and `jpeg-encoder` in the engine, pages in parallel |
| Encode TIFF | none | The `tiff` crate in WebAssembly (§11) | The same crate, natively |
| Edit session and save | as E1 | The document worker | The engine process (E1h, E1i) |

The web zip stores entries without compressing them again, since PNG, JPEG and TIFF are already compressed, so it streams into a `Blob` one page at a time within `Limits`. The writer is about 150 lines of our own (a recommendation): a dependency would buy nothing, since stored entries need only a CRC-32 and the headers. The object inspector is on both shells and hidden below 600 px wide (O15).

No server feature: every edit, inspection and export happens on the device, and attachments never leave it ([ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md)).

## 8. Todos

Each todo is one reviewable pull request of at most L (400–800 changed lines) on the [todo template](../../todos/TEMPLATE.md)'s scale, and each says why its size fits. The web and shared todos flip the `core`, `pdf` and `web` columns; E3k flips `desktop`.

| ID | Todo | Depends on |
| --- | --- | --- |
| [E3b](../../todos/ready/E3b-bookmarks-and-destinations.md) | Outline and named-destination ops, the editable Outline tab and the named destinations dialog | E2j |
| [E3c](../../todos/ready/E3c-links.md) | Link ops, tagged links, the link tool and popover | C5b, E2j |
| [E3d](../../todos/ready/E3d-initial-view-and-threads.md) | `SetInitialView`, the initial view on open, and threads read | E3a |
| [E3e](../../todos/ready/E3e-xmp-editor.md) | XMP property ops, extension schemas and the XMP editor | E1c, B8b, X3a |
| [E3f](../../todos/ready/E3f-attachments-and-associated-files.md) | Attachment ops, `/AF`, portfolios read, and the Attachments tab | E2i, E2j, E2p, B8b, X3a |
| [E3g](../../todos/ready/E3g-inspectors.md) | Conformance, fonts, colour and object inspection, and the inspector | T2c |
| [E3h](../../todos/done/E3h-images-to-pdf-and-page-export.md) | Images → PDF and page export with the three encoders (the Rust core) | C2b |
| [E3m](../../todos/ongoing/E3m-web-image-export.md) | Images → PDF and page export in TypeScript and the web reader: the binding, the worker's export call and the TIFF module | E3h |
| [E3i](../../todos/ready/E3i-extraction.md) | Image extraction, and links and bookmarks as JSON and CSV | B9c |
| [E3j](../../todos/ready/E3j-export-dialog.md) | The export dialog (O16): pages, text, images, links and bookmarks; images → PDF in the interface | E2m, E3i, E3m |
| [E3l](../../todos/ready/E3l-contents-page.md) | The contents page: `create_contents_page` and its dialog | E2c, E2o, E3b, E3c |
| [E3k](../../todos/ready/E3k-desktop-navigation-and-export.md) | Desktop: the round's panels and dialogs in the engine, exports to a folder, attachments to disk; the round's close | E3b–E3j, E3l, E1i, E2i, E2n |

Every todo also depends on this plan (E3a). E3d, E3e, E3g, E3h and E3i can start as soon as it merges; E3b, E3c and E3f wait on E2's tree writers and blob store, E3j on E2's dialogs, and E3l on E2's cross-document pages. E3k completes the desktop column when the desktop track is ready.

## 9. Fixtures

Every new fixture is written by Lamina PDF's own scripts and licensed like the rest of Lamina PDF, `MIT OR Apache-2.0`; provenance goes in `fixtures/README.md`.

- **`fixtures/navigation/`** (new, E3b, from a dependency-free `make.py`): `outline.pdf` (three levels, closed and open items, colours and styles, a `URI` and a JavaScript action), `outline-broken.pdf` (a cycle, a wrong `/Last`, wrong counts), `dests.pdf` (names in the tree and in the PDF 1.1 dictionary, used by bookmarks and links), `links.pdf` (links with quads, a `/URI /Base`, a `javascript:` link and a `GoToR`), `initial-view.pdf` (an `/OpenAction` destination, page modes and layouts, every preference) and `threads.pdf` (two threads across three pages).
- **Tagged files:** the builder's `golden/tagged-report.pdf` for tagged links and the contents page.
- **XMP:** `fixtures/metadata/` gains a PDF/A-2b file with a custom property and its extension schema, and a packet with struct values (E3e).
- **Attachments** (E3f): a Factur-X-style PDF/A-3 invoice built by the builder (an XML attachment with `/AFRelationship /Alternative`), a page-level `/AF`, a FileAttachment annotation, and a portfolio with a `/Collection` schema, written by `make.py`.
- **Images** (E3h, E3i): JPEGs with each of the eight EXIF orientations (`tools/fixtures/images.py`), and `fixtures/images/extract.pdf` with an image of each kind E3i decodes (DCT, JPX, Flate RGB, `Indexed`, CMYK, 16-bit, `/Decode` inverted, `/SMask`, `/Mask` stencil, an inline image, a nested form).
- **Signed files:** the pyHanko files in `fixtures/signed/` (approval, certified at P1, P2 and P3, a FieldMDP lock), with the usage-rights file E2i adds.

## 10. Tests

Each op gets the tests its todo lists, and the table says which tiers apply ([ADR 0018](../../decisions/0018-test-strategy.md)). "Round-trip" means Lamina PDF, pdf.js, PDFium and qpdf (`--check` clean) each reopen the saved file and report the op's change and nothing else. The tagged tests in `crates/laminapdf-save/tests/roundtrip_suite.rs` flip the matrix, and `cargo xtask roundtrip` gains the ops.

| Op or function | Unit | Golden | Round-trip | Differential | Fuzz | e2e |
| --- | --- | --- | --- | --- | --- | --- |
| Outline ops (E3b) | property: random edits keep the list and counts valid; repaired outlines rewritten | — | pdf.js's `getOutline` and PDFium's bookmarks read the same tree | — | `ops_json` gains the ops | Playwright (E3b) |
| Named destinations (E3b) | rename follows uses; PDF 1.1 names moved | — | pdf.js resolves each name to the same page | — | `ops_json` | Playwright (E3b) |
| Link ops (E3c) | URI refusals; quads from glyphs; structure placement | an incremental save on `links.pdf` | pdf.js and PDFium report the same targets; veraPDF no worse on tagged files | — | `ops_json` | Playwright (E3c) |
| `SetInitialView`, threads (E3d) | every layout and mode; threads read | — | pdf.js reads the open action, mode and layout | — | `ops_json` | Playwright: the initial view on open (E3d) |
| XMP ops (E3e) | paths, value kinds, Info sync, extension schemas | — | ExifTool reads the properties; pdf.js and PDFium read the packet; `qpdf --check` clean | `--live --xmp` covers the edited fixtures | `xmp_parse` gains packets with struct values | Playwright (E3e) |
| Attachment ops (E3f) | `/Params`, checksum, `/AF` targets, PDF/A warnings | — | pdf.js's `getAttachments` returns the same bytes; veraPDF passes the PDF/A-3 invoice | — | `ops_json` | Playwright: attach, save out and compare bytes (E3f) |
| Inspection (E3g) | each API on the corpus | — | — | objects, streams and revisions vs `qpdf --json` on 20 corpus files | — | Playwright (E3g) |
| `images_to_pdf`, `export_pages` (E3h) | orientations, sizes; encoders' headers | `images-to-pdf` example output | qpdf `--check`; PDFium draws each page as its image | exported pixels vs `render-golden` within its tolerance | — | Playwright (E3j) |
| `extract_images`, links and outline export (E3i) | each decode path; schemas validate | the extracted images of `extract.pdf` | — | images vs pdfimages' (poppler, run only as an outside tool, ADR 0019) | new `extract_images` | Playwright (E3j) |
| Contents page (E3l) | entries, labels, leaders | the page for `outline.pdf` | the links land on the bookmarked pages in pdf.js | — | — | Playwright (E3l) |

Across every op:

- **Undo:** apply then undo leaves the overlay empty and the next save byte-identical; redo writes the same bytes (ADR 0006 §3).
- **Determinism:** the same state saves to the same bytes on native and wasm32 (ADR 0007 §6).
- **Signatures:** `would_violate` on every signed fixture in `fixtures/signed/`, asserted per op for each row of §6; E3k runs the whole table, the usage-rights row included, once E2i adds that file.
- **e2e:** Playwright in Chromium, Firefox and WebKit drives each command as a user does and checks what the page shows, then saves, reopens and checks the file; WebdriverIO on three systems repeats the suites and the folder exports in `apps/desktop/e2e/specs/` (E3k). Each test carries its capability tag.

A new fuzz target's `fixtures/regressions/<target>` folder must also join `laminapdf-model`'s regression guard list. Links need no appearance stream, since §12.5.5 exempts Link annotations from the requirement, so no E3 op draws an appearance; that work starts in E4.

## 11. Decisions on the open questions

These are the round's defaults, and the owner is asked to confirm them in [issue #511](https://github.com/laudebugs/laminapdf/issues/511); the todos follow any change.

- **The contents page comes from bookmarks only.** E3l builds it from the outline, one or two levels deep. A contents page from the headings of a tagged file's structure tree comes with [E15b](../../todos/ready/E15b-structure-tree-editing.md), as O11 already says for "Bookmarks from headings". Headings guessed from font sizes are not offered. Reasoning (a recommendation): `laminapdf-extract` does detect headings by size and weight, but a guessed heading that is wrong puts a wrong entry on a page the person then saves, where a missed bookmark only leaves one out.
- **TIFF export uses the `tiff` crate, with multi-page files on both shells.** It is pure Rust and MIT-licensed, writes Deflate- and LZW-compressed files, and writes several pages into one file. Multi-page TIFF is offered on the web too, held to the export's memory budget: a file that would exceed it is written as one file per page instead, and the dialog says so. On the web the encoder is a small WebAssembly module of its own, loaded only when a TIFF export starts, so the reader's startup size doesn't change; PNG and JPEG need no module, since the browser's canvas encodes them.
- **The object inspector is read-only, and no round edits raw objects.** A raw edit could change anything, so no single `ModificationClass` fits it. The planner would have to treat it as every class at once, needing a full rewrite, and that breaks every signature. Raw editing would need an ADR of its own, and no round plans one. The inspector copies an object as PDF syntax and saves a raw stream, which is enough for the developers O15 serves.
- **Article threads stay read-only; editing them goes to E15.** Threads describe a reading order across columns, which E15's reading-order work owns; its feature plan (E15a) decides whether to edit threads or to map them onto the structure tree.
- **Identification claims are not editable.** `pdfaid:` and `pdfuaid:` say a file conforms; changing them without making it conform would make the claim false. E1 keeps them, and E3e's ops refuse them. Converting to PDF/A is E16's.
- **The plan's location:** `docs/plans/editor/`, as the plans index, E1 and E2 use.
