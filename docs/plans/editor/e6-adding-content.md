# E6: adding content to pages

**Status:** Draft

The sixth editor round puts new things on existing pages as page content, not as comments: text, symbols, images, shapes, logos, stamps, watermarks, backgrounds, headers, footers, page numbers and Bates numbers. This plan fixes the ops, how each added object keeps its identity so it can be changed after a save, how the save planner gates content changes, and the 7 pull requests that build the round after this one.

Two parts are hard. An object added today must still be found, moved and removed after the file is saved and reopened, without parsing the page's own content. And every change to a page must reach the screen quickly, through a save and a reload in a second parser (PDFium).

- **Round:** [full-editor §E6](../full-editor.md#e6-adding-content-to-pages), phase 4, milestone M7 Content editor
- **Feature-plan todo:** [E6a](../../todos/done/E6a-adding-content-feature-plan.md)
- **Follows:** ADR [0005](../../decisions/0005-document-model.md) (the overlay, one builder path), [0006](../../decisions/0006-op-log-and-undo.md) (ops, undo, the blob store), [0007](../../decisions/0007-save-planner.md) (the save planner), [0018](../../decisions/0018-test-strategy.md) (test tiers), [0024](../../decisions/0024-app-shell-ux.md) (modes and commands), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space), [0032](../../decisions/0032-flowing-layout-and-document-inputs.md) (page tokens)
- **Builds on:** [E2e](../../todos/done/E2e-content-append-api.md) (the content append API), [E4](e4-markup-and-drawing-annotations.md) (E4c's Form XObjects in the builder, E4e's fonts in the extract module), [B4](../pdf-library.md#b4-fonts-and-text) and [B8](../pdf-library.md#b8-tagged-pdf-pdfua-and-pdfa) (fonts, tagging)
- **Design:** [A01 Add text box](../../design/paper/add-edit-content.md#a01-edit-mode-add-text-box), [A02 Place image and shapes](../../design/paper/add-edit-content.md#a02-place-image-and-shapes), [A03 Watermark, header and footer dialog](../../design/paper/add-edit-content.md#a03-watermark-header-and-footer-dialog)

## 1. Where things stand

- **Writing into a page.** `laminapdf_content::PageContentWriter` (E2e) wraps a page's content in `q … Q` once (`wrap_existing`), adds streams before (`prepend`) and after (`append`) it, and registers XObjects, graphics states and fonts under unused names (`add_xobject`, `add_extgstate`, `add_font`). It never parses the page's own operators, and every write goes through `CosMut`, so a transaction undoes it.
- **Drawing.** `laminapdf-build`'s `Surface` draws paths, colours, opacity, images and shaped text in page space, but only on pages the builder makes: `Builder::on` appends new pages to an open document and can't draw on an existing one. The builder writes image XObjects; Form XObjects arrive with E4c's appearances.
- **Payloads.** `laminapdf_ops::BlobStore` holds images and fonts by SHA-256 (`BlobId`), so an op names its bytes instead of carrying them.
- **Showing an edit.** The viewer's `applyEdit` (E1g) saves the appended revision into memory and reopens the render document over it. Nobody has measured how long that takes on a large file.
- **The matrix.** A.01, A.12, B.04, B.05, B.09, B.10 and C.14 have `round = "E6"`. B.01, B.02 and B.12 are E7's rows ([E7 plan §1](e7-editing-existing-content.md#1-where-things-stand)): E6 takes B.01 and B.02 to `write` on the objects it adds, and E7 edits existing ones.

## 2. The operations

| ID | What it means | Ops (§2.1) | Target | Todos (core and `pdf` · web · desktop) |
| --- | --- | --- | --- | --- |
| A.01 | Add text anywhere on a page | `AddText` | round-trip | E6c · E6e · E6e, E6h |
| A.12 | Insert special characters and symbols | `AddText` with the fallback chain | round-trip | E6c · E6e · E6e, E6h |
| B.01 | Insert, delete, replace images | `AddContentObject`, `UpdateContentObject`, `DeleteContentObject` | write | E6b · E6e · E6e |
| B.02 | Resize, rotate, crop, reposition | `UpdateContentObject` | write | E6b · E6e · E6e |
| B.04 | Adjust transparency/opacity | `opacity` on every added object | round-trip | E6b · E6e · E6e |
| B.05 | Draw lines, rectangles, circles, polygons | `AddContentObject` with `Shape` | round-trip | E6b · E6e · E6e |
| B.09 | Add logos, icons, stamps | `AddContentObject` with `Image` or `Form` | round-trip | E6b · E6e · E6e |
| B.10 | Background images and watermarks | `AddWatermark`, `AddBackground` | round-trip | E6d · E6g · E6g |
| C.14 | Labels, page numbers, headers/footers | `SetHeaderFooter`, `ApplyBates` | round-trip | E6d · E6g · E6g |

B.01 and B.02 stay at `write`: E6 changes only what Lamina PDF added, and E7d's editing of existing images takes them to `round-trip`. E6b also moves B.12 to `write` for added objects, ahead of its round.

### 2.1 Ops

Ops name pages by `PageId`, carry every value they write, and take geometry in page space (ADR 0027), converted once through the page's `PageFrame`. Every adding or changing op carries `modified: PdfDate`. It goes into the form's `/LastModified` and its page-piece dictionary's, both required when `/PieceInfo` is present (ISO 32000-2 §14.5), so no op reads a clock and the output stays byte-deterministic. Every op is class `Content`; none needs a full rewrite.

| Op | Payload | Todo |
| --- | --- | --- |
| `AddContentObject` | `{ id: ContentId, page: PageId, object: ContentObjectSpec, matrix, opacity, layer: Front \| Behind, alt: Option<Alt>, modified }`; `Alt` is `Text(String)` or `Decorative`; `ContentObjectSpec` is `Image { blob: BlobId, crop: Option<RectPt> }`, `Shape { path: PathSpec, fill, stroke }` or `Form { blob: BlobId, page: u32 }` (a page of a PDF in the blob store, drawn as a form) | E6b |
| `UpdateContentObject` | `{ id: ContentId, page: PageId, modified, matrix, opacity, crop, fill, stroke, blob, alt, order: Option<i32> }`, each field but `id`, `page` and `modified` optional; `null` removes a crop, a fill or a stroke; a replaced image or page is a new `blob`; `order` is the object's index among its layer's, negative from the front | E6b |
| `DeleteContentObject` | `{ id: ContentId, page: PageId }` | E6b |
| `AddText` | `{ id: ContentId, page: PageId, rect: RectPt, runs: Vec<TextSpan>, align, fonts: Vec<AnnotFont>, size, color, tag: Option<TagChoice>, modified }`; `TextSpan` (E4e's) is `{ text, bold, italic }`, an `AnnotFont` is `helvetica`, `times`, `courier` or a font's bytes by their `BlobId`; the object it makes is `ContentObjectSpec::Text(TextBlock)`, a box in its own space with the `rect`'s size, which `UpdateContentObject`'s `text` replaces and its `matrix` moves; editing and deleting added text is `UpdateContentObject` and `DeleteContentObject` | E6c |
| `AddWatermark`, `AddBackground` | `{ id: ContentId, pages: Vec<PageId>, mark: Text { text, fonts: Vec<AnnotFont>, size, color } \| Image { blob: BlobId }, opacity, angle, hideable: bool, modified }` | E6d |
| `SetHeaderFooter` | `{ id: ContentId, pages: Vec<PageId>, slots: [Option<String>; 6], fonts: Vec<AnnotFont>, size, color, margins, date: String, file: String, modified }`; tokens are expanded when the op applies (§2.3) | E6d |
| `ApplyBates` | `{ id: ContentId, pages: Vec<PageId>, prefix, start: u64, digits: u8, suffix, slot, fonts: Vec<AnnotFont>, size, color, margins, modified }` | E6d |
| `UpdatePageMark`, `DeletePageMark` | `{ id: ContentId, pages: Vec<PageId>, mark: PageMark, modified }` and `{ id: ContentId, pages: Vec<PageId> }`: a changed mark is the new payload in place of the old, of the same kind (`PageMark` is `Watermark`, `Background`, `HeaderFooter` or `Bates` with that op's payload) | E6d |

**No `AppendContent` op.** E2e left an `AppendContent` op to this round. A wire op that carries raw content bytes would let any caller write unbalanced or unsafe operators, so there is none: every op is typed. The ops share one private function in `laminapdf-ops`, `place_form(page, form, matrix, layer)`. It wraps the page once with E2e's `PageContentWriter` and appends the object's drawing stream, so `laminapdf-ops` gains a dependency on `laminapdf-content`. E5e's flattening, E8g's form flattening and E2s's page resizing call it from their own ops; whichever of them lands before E6b writes it, and E6b reuses it. Images and fonts reach it as `BlobId`s from the blob store.

**Drawing the form goes through a trait.** `laminapdf-build` depends on `laminapdf-ops`, so no op can call the builder's `Surface`, as the [E4 plan §2.2](e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated) found for appearances. E6b extends E4's `Appearances` trait, which `laminapdf-ops` defines and `laminapdf-build` implements, with a method that draws a `ContentObjectSpec`, a text block or a page mark into a new Form XObject through the op's `CosMut`; the façade passes the builder's implementation in, as it does for annotations. `laminapdf-text` stays out of `laminapdf-ops`: shaping happens inside the implementation.

**One object, one form, one stream.** Each added object is its own Form XObject (ISO 32000-2 §8.10), drawn by a stream of its own in the page's `/Contents`, after the wrapped content for `Front` or before it for `Behind`. The stream has exactly this grammar, and the scanner accepts nothing else:

```
q a b c d e f cm [/GSn gs] [/Tag <<…>> BDC] /FmN Do [EMC] Q
```

`gs` sets the opacity; `BDC … EMC` is the artifact or the structure element's marked content (§3). Moving, resizing or fading an object rewrites that one small stream; deleting it removes the stream from `/Contents` and the name from the resources. Nothing reads the page's own operators. Stacking order (Bring forward and the rest) reorders added streams among themselves; nothing goes behind original content except a `Behind` object, and E7 handles stacking among existing objects.

**Each kind's space** (E6b). An image and a page of another PDF fill the unit square, `(0, 0)` its top-left corner and y down as page space runs, so `matrix = [w, 0, 0, h, x, y]` draws one `w` by `h` points at `(x, y)`; an image's `crop` is a rectangle of that square, which the form's `/BBox` clips to, so the part that shows stays where it was. A shape's path is in points, y down, so the identity draws it where its points are. The update and delete ops name the page as every other op does, so finding an object reads one page's streams, not the document's.

**Identity after a save.** Each form carries a page-piece dictionary (ISO 32000-2 §14.5): `/PieceInfo << /LaminaPDF << /LastModified … /Private << /Id (…) /Kind /Image /Spec (…) >> >> >>`, `/Spec` the op's object as JSON (and `/Alt` or `/Decorative`), so a shape's fill or stroke can be drawn again after a reopen; an image's crop needs only the form's `/BBox`. When a file is reopened, the scanner matches each stream of `/Contents` against that grammar and reads the `/PieceInfo` of the form it draws. A stream or form someone else changed no longer matches, so it is left alone as original content. Acrobat marks the watermarks and headers it adds the same way (`/ADBE_CompoundType` in `/PieceInfo`). `laminapdf reveal` lists these entries under metadata, as it lists anyone's `/PieceInfo`.

### 2.2 Text

`AddText` lays out its runs through the text box generator E4e built (`laminapdf-text`: `break_spans`, shaping, bidi), inside one form, embeds a subset of each font given by its bytes with a ToUnicode map, and draws each character in the first of `fonts` that has it: the first font, then the fallbacks. A character no font has is `content-glyph-missing`, never a silent `.notdef`; a font whose `fsType` forbids embedding is `content-font-not-embeddable`; bytes that aren't a font are `content-font-invalid`. The codes are the content ops' own: ADR 0026 §3 gives one code to one layer, and the text layer's `missing-glyph` and `font-not-embeddable` stay the builder's.

The text is laid out in a box its own size, wrapped at the box's width, starting at its top and clipped to it. It has one size and one colour, as a text box has; bold and italic are per run. Per-run sizes and colours, with the font controls of the inspector, come with E7f's text styling. Only the first font may be a standard family, since every fallback is embedded.

Amended by E6c: the runs are E4e's `TextSpan` and the fonts E4e's `AnnotFont`, not a `TextRunSpec` and a `FontRef` of their own, so E6d's page marks take `Vec<AnnotFont>` too. That leaves out two things the plan's first draft had. `AnnotFont` names `helvetica`, `times` and `courier` only, not Symbol or ZapfDingbats, whose encodings E4e's generator doesn't draw; symbols come from an embedded font (Noto Sans Symbols 2, [E6i](../../todos/done/E6i-bundled-fonts.md)). And a tagged file gets a `P` element or an artifact, never a `Span` inside a paragraph. A new height in an `UpdateContentObject` text patch keeps the box's top edge where it was, as a text box's resize does, unless the patch carries a `matrix`. A file that claims PDF/UA-1 needs every font embedded (ISO 14289-1 §7.21.4.1), and `AddText` doesn't check that its standard fonts are; the docs say so.

The base module has no text layout, so `AddText` there fails with `appearance-needs-text`, E4's code. The standard families work in the extract module, as E4e's text boxes do (the viewer opens documents there); a font given by its bytes needs the text module. E6c reuses E4e's generator rather than a second one, so a text box and added text lay out the same.

### 2.3 Page marks

- **Watermark and background.** A watermark is drawn in front of the content and a background behind it, each as a form on every chosen page. In a tagged file the drawing is an artifact (`/Artifact << /Type /Pagination /Subtype /Watermark >>`, §14.8.2.2). With `hideable` the form gets an `/OC` entry naming an optional content group with a `/Usage << /PageElement << /Subtype /FG >> >>` (or `/BG`; §8.11.4.4), so a reader can switch it off. Watermark annotations (§12.5.6.22) are not used: a background can't be one, and one kind of mark keeps Manage page marks simple.
- **Header and footer.** Six slots (left, centre, right; top and bottom). Tokens follow T4b's syntax, which landed first: `{page}`, `{pages}`, plus `{label}` (the page label), `{date}` and `{file}`, whose values the payload carries. The dialog shows them as `[Page]`, `[Pages]` and the rest and writes the braces. They are expanded when the op applies, so a later page move doesn't renumber them; the dialog says so.
- **Bates numbers.** Prefix, start, digits and suffix, in one slot; across files (E14d) each file's op carries its own start.
- **Amended by E6d.** A mark is one container Form XObject a page, in page space, that draws the mark's text boxes and pictures through E6c's generator, so a mark is its own kind of added object: the same stream grammar, an ID unique among a page's added objects and marks (`content-id-in-use`), and `/PieceInfo` that also holds the mark as JSON, which `UpdatePageMark` replaces (a Bates mark's `start` on a page is that page's own number). `added_objects` and the content ops leave marks alone; `page_marks` lists them, and a page without the mark is `unknown-page-mark`, the round's one new code. A header slot's box is as wide as the page between its margins, aligned by its column, so the three slots on an edge overlap and a long text runs past its third. Text a mark draws is one line to a slot and a watermark's box is 90 % of the page's diagonal wide; a longer line wraps, then what doesn't fit is clipped. A watermark image shows whole inside 80 % of the page and a background image covers it, both centred and turned by `angle`; the generator gains `image_size` to scale them. `color` and `margins` join the header, footer and Bates payloads, and `UpdatePageMark` and `DeletePageMark` name their pages. An artifact says what it is: `/Type /Pagination` with `/Subtype /Watermark`, `/Header`, `/Footer` or `/Bates` (a mark in both bands has none on the page, and each part inside the form is a header or a footer), `/Type /Background` for a background. A hideable mark has one optional content group for all its pages (`/Usage /PageElement /Subtype /FG` or `/BG`), listed in `/OCGs` and the default configuration's `/Order`, and taken out when no page names it. PDF/A-1 forbids optional content, so a hideable mark on such a file saves with the warning `pdfa-optional-content` (never a refused edit, as for transparency, §3); the dialog (E6g) is to leave the choice out. A group joins `/OCProperties` through the merge's code, which names the default configuration (PDF/A-2 and -3 want `/Name`) and lists the group in `/ON` when the base state is off; leaving takes it out of every configuration's lists. A mark's ID is one kind on every page: adding it where another page has it as another kind is `invalid-op`. A Bates number is padded to six digits unless `digits` says otherwise, in the ops, on the wire and in the SDK. A font given by its bytes is embedded as one subset for each distinct text a mark draws, which a text that differs on every page makes a page: the generator embeds a subset for each box and can't share a font between boxes ([issue #837](https://github.com/laudebugs/laminapdf/issues/837) proposes a design that can). Tagged files: a mark in both bands is `/Type /Pagination` on the page with each part inside the form an artifact of its own, a header or a footer. Round trip, in the same cases as the other content ops: Lamina PDF finds each mark on the first page after reopening, and pdf.js and PDFium read the text each draws.
- **Overlap check.** Before Apply, A03's notice comes from `laminapdf-extract`'s runs in the header and footer bands (the top and bottom 10 % used for running headers), so it needs the extract module, where the viewer already opens documents. E6d ships it as the façade's `Document::page_bands(page)`, the runs whose middle is in either band with their boxes; the SDK and the dialog read the same runs from `content.textRuns` and the page's size until E6g asks for a binding.

## 3. Conformance the edits must keep

- **Tagged PDF and PDF/UA-1.** Added text becomes a `P` element, or an artifact when the user chooses. Its marked content sits around the `Do` in its drawing stream, so its MCID is the page's and the page's `/StructParents` entry in the parent tree lists it. Marked content inside the form would need the form's own `/StructParents` and MCRs with `/Stm`, which the one-stream grammar avoids. An image gets a `Figure` with its alt text, or is an artifact when marked as decoration; shapes are artifacts unless given alt text. On a file that claims PDF/UA-1, an image or a page of a PDF whose `alt` is `None` is refused with `content-needs-alt-text`, an op error of its own, since one code belongs to one layer (ADR 0026 §3) and B8b's `missing-alt-text` is the builder's; `Decorative` makes it an artifact. Page marks are always artifacts.
- **PDF/A.** Every font added is embedded, colours use the file's output intent or carry an ICC profile, and transparency breaks PDF/A-1 (ISO 19005-1 §6.4): an opacity below 1 or an image with an alpha channel makes the save plan warn `pdfa-transparency`, E4c's warning, which now lists the pages too, as a translucent annotation is warned of rather than refused. A page of another PDF whose fonts aren't embedded warns `pdfa-font-not-embedded` with its page. On PDF/A-2 and -3 an alpha channel becomes an SMask, which they allow. The owner settled [#749](https://github.com/laudebugs/laminapdf/issues/749): on PDF/A-1, see-through added content stays a warning at save, never a refused edit. An image, an image stamp or a page of another PDF that paints in a device colour space the output intent doesn't allow warns `pdfa-color-space` with its page or stamp. That warning closed follow-up [#750](https://github.com/laudebugs/laminapdf/issues/750) ([bug notes](../../bugs/pdfa-added-color-space/detail.md)).
- **Earlier revisions.** An incremental save keeps the page as it was in the previous revision, so `DeleteContentObject` and `DeletePageMark` set `still_in_earlier_revisions`, as every delete does.

## 4. Crate and package changes

- **`laminapdf-build`**: a `Surface` over a Form XObject in an existing document, sharing the page surface's paths, colours, images and text, behind the `Appearances` implementation (E6b, E6c).
- **`laminapdf-ops`**: `place_form`, the ops of §2.1, `ContentId`, the `Appearances` method, and `Document::added_objects(page)`, the scanner over the drawing streams and the forms' `/PieceInfo`, all in one module (E6b–E6d); a new dependency on `laminapdf-content`.
- **The façade and bindings.** Each op is on `laminapdf::Document` and `@laminapdf/sdk`'s `Document`, with docs, a Rust and TypeScript example and a frozen sample in `fixtures/ops/v1/` (requirement 7); in TypeScript they are `doc.content.add`, `update`, `remove`, `addText`, `addWatermark`, `addBackground`, `setHeaderFooter`, `applyBates` and `pageMarks`.
- **`@laminapdf/ui`**: the Edit-mode tools, handles, snapping and inspectors (E6e); the Page marks dialog (E6g).
- **`apps/desktop`**: fonts on the computer in the font picker, through the engine's existing `SystemFonts` (E6h).
- **Size.** The content ops join every module, since each can edit; E6b and E6c measure their growth against `packages/wasm/size-budgets.json` and ask the owner before raising a budget (ADR 0016 §6).

## 5. Saving and the save constraint

Every op is class `Content`. DocMDP permits no content change at any level: P1 allows none, P2 form filling and signing, P3 also annotations (ISO 32000-2 §12.8.2.2).

| Constraint | When | What Save does | What the user sees |
| --- | --- | --- | --- |
| `Free` | No signature, a sound base | An incremental update | Nothing; after a delete, the note that earlier versions stay in the file |
| `AppendOnly`: approval signatures | Signed, not certified | An incremental update after the signature, which stays valid | The status-bar note "Signed file. Saving adds your changes after the signature" |
| `AppendOnly` with a FieldMDP lock | Locked form fields | An incremental update: no E6 op touches a field or widget | The note |
| `AppendOnly` with DocMDP P1, P2 or P3 | Certified | Every op violates, so the session becomes `Conflict` | The banner naming the certifier from the first edit; on P1, from entering Edit mode |
| `RequiresRewrite` | A repaired file | A full rewrite | A neutral strip saying why |
| `RequiresRewrite`: encrypted base | Password-protected | Save is refused (`encrypted-base`) until E11 | The warning strip |
| `Conflict` | Ops the constraint forbids | Save is refused; Apply anyway… invalidates the named signatures after a confirmation (`with_invalidate_signatures`) | The danger strip listing the changes, with Save a copy |

## 6. Differences between the shells

| Area | Flag | Web | Desktop |
| --- | --- | --- | --- |
| Fonts | `systemFonts` (new, E6h) | Bundled fonts and font files the user adds | Also fonts on the computer, read by the engine and passed in as bytes |
| Images in | `clipboardImages` (E6e) | Picker, drop, and paste where the browser allows | Also the OS clipboard and drag from Finder or Explorer |
| Formats | | JPEG and PNG | Also what the OS can decode, converted to PNG at the edge |

No server feature: everything runs on the device.

## 6a. Reload after a content edit: measured and gated

A content edit reaches the screen through the two-parsers path ([architecture §6](../architecture.md#6-risks)): the model writes the changes since the file was opened as one appended revision (`saveCopy` with `AppendedOnly`), PDFium opens the file joined with that revision, and the viewer draws the touched pages again. [E1g](../../todos/done/E1g-redraw-after-edit.md) built this path and measured it on small files; [E6f](../../todos/done/E6f-reload-budget.md) measures it on files of about 1, 10 and 100 MB.

**The files.** `packages/viewer/perf/reload.json` is the one list of them, read by the web harness, the desktop bench and the desktop gate: `ten` (10 pages, 1.0 MB), `mb10` (40 pages, 10.1 MB) and `mb100` (100 pages, 99.7 MiB), each page text with one image of seeded noise that Flate can't shrink. Scripts write them (never committed); the harness builds the two large ones only for a run that measures the reload, and the desktop bench stamps each file with its recipe. The same file holds the baselines.

**How it is measured.** *Web* (`pnpm --filter @laminapdf/viewer perf --only reload` runs just this): each run opens the file afresh and places a 64 by 64 PNG on the page in view (`addContentObject`, after the picture's bytes are added to the document, as a picker has read them by then). The figure is the median of three runs, from the viewer's `applyEdit` call until the page in view shows what the edit drew (`whenDrawn`; not the page's render state, which an edit that keeps the page's size sets to "low", with its old pixels, in the same task the call answers). The harness also times `saveCopy` of the same edit with the SDK in the page (`saveCopyMs`), so the split below is reproducible. *Desktop* (`cargo xtask desktop-reload`, `crates/laminapdf-desktop-engine/benches/reload.rs`): the built engine under the supervisor with the pinned native PDFium, a new engine process per run, the median of three of the edit call plus one 892 by 800 px tile of the page in view drawn again. The desktop engine is built without the builder's appearance writer, so it refuses `addContentObject` (`appearance-unavailable`, [#834](https://github.com/laudebugs/laminapdf/issues/834)); the bench turns the page a quarter, which writes the same appended revision and reopens PDFium the same way.

**Measured** on an Apple M1 Pro (8 cores, 16 GB), macOS, 2026-11-27, nothing else running; web is Chromium 156 with PDFium-WASM. This is a laptop, so these figures are a reading, not a baseline.

| File | Web: call answered | Web: new pixels drawn | Web: sharp | of the call, `saveCopy` | Desktop: edit call | Desktop: page drawn again |
| --- | --- | --- | --- | --- | --- | --- |
| 1 MB | 37 ms | 56 ms | 60 ms | 4 ms | 1 ms | 1 ms |
| 10 MB | 72 ms | 91 ms | 94 ms | 40 ms | 6 ms | 7 ms |
| 100 MB | 431 ms | 460 ms | 462 ms | 400 ms | 94 ms | 96 ms |

A first version of the web harness read the page's render state and reported the call's own time as the first pixels (43, 87 and 545 ms in one run); the table above uses the corrected measure. The `saveCopy` column runs in the page's main thread; in the document worker the same step took 8, 52 and about 510 ms.

**What the time is.** The reopen of PDFium is about 2 ms at every size (the worker's timings, taken with temporary instrumentation: `saveCopy` 8, 52 and 510 ms, reopen 2 ms): PDFium reads the file through the joined source block by block, and the revision is a few kilobytes. The cost that grows with the file is the save. The incremental writer reads the whole base and hashes it with SHA-256 to derive the trailer's `/ID[1]` (`laminapdf-save`, `IncrementalWriter`): about 4 ms per MB in WebAssembly and about 1 ms per MB natively. The viewer already invalidates only the touched pages and the reload is already append-only, so the optimisations the todo named are in place; what is left changes shared save code and is [#834](https://github.com/laudebugs/laminapdf/issues/834).

**The gate** ([ADR 0018](../../decisions/0018-test-strategy.md), amended for this): a figure fails when its median is over its **baseline** by more than 20 % and at least 10 ms (the floor keeps a 1 ms figure from failing on noise). A baseline is the median of runs on the CI runner, recorded in `reload.json`; none is taken from a laptop, and a figure with none is reported and never fails.

**CI measurements** (desktop bench, hosted Ubuntu 24.04, three runs on 2026-11-27, page drawn again, medians of three opens each): 1 MB 3, 6 and 4 ms; 10 MB 8, 13 and 10 ms; 100 MB 109, 219 and 158 ms. The baselines recorded are the medians of those: **4, 10 and 158 ms**. The runs differ by up to 2x on the 100 MB file, more than the gate's 20 %, so a gate at 158 ms + 20 % would have failed one of its own three runs: that is why the desktop job stays advisory, and why the first nightly runs (more points) should decide whether the baseline or the 20 % needs to move before the step is made to fail. The web has no CI measurement: its gate runs in the release lane only, so its baselines are empty.

- *Desktop:* the `desktop-reload` job of `nightly.yml` (hosted Ubuntu, like the benchmarks; the engine and the pinned PDFium run there) runs `cargo xtask desktop-reload --gate`. It is **advisory** until the baselines are recorded: a miss sets the job's `regression` output and the report job opens the nightly issue. Deleting `continue-on-error` from the step enforces it.
- *Web:* the reader's release gate (`cargo xtask viewer-perf --gate`, `web-deploy.yml`) holds the reload figures through `reloadGates` in `perf/gate.ts`, which gates a size only once it has a baseline in `reload.json`. Until the release lane's first run is recorded there, the web reload is **report-only**, so a flaky figure cannot block the production deploy; recording the baselines turns the gate on. The nightly history keeps `reload-first-pixels-<n>mb` and flags a regression over 10 % as for every figure.

Neither runs on pull requests (ADR 0018 §2).

## 7. Todos

| Todo | What | Depends on |
| --- | --- | --- |
| [E6b](../../todos/done/E6b-place-images-and-shapes.md) | `place_form`, `AddContentObject`, `UpdateContentObject`, `DeleteContentObject`, identity, `added_objects`, `pdfa-transparency` | E2e, E4c, E6a |
| [E6c](../../todos/done/E6c-add-text.md) | `AddText`, fallback, tagging, web font sources | E6b, E4e |
| [E6d](../../todos/done/E6d-watermarks-headers-footers.md) | The four page-mark ops with update and delete, tokens, OCG, overlap check | E6c |
| [E6e](../../todos/ongoing/E6e-content-tools-ui.md) | Edit-mode tools, handles, snapping, inspectors | E6b, E6c, E6i |
| [E6f](../../todos/done/E6f-reload-budget.md) | Touched-page reload time measured and budgeted | E6b |
| [E6g](../../todos/ready/E6g-page-marks-dialog.md) | The Page marks dialog and Manage page marks | E6d, E6e |
| [E6h](../../todos/ready/E6h-desktop-system-fonts.md) | Fonts on the computer in the desktop font picker | E6c, E6e |
| [E6i](../../todos/done/E6i-bundled-fonts.md) | Newsreader, a monospace and Noto Sans Symbols 2 as bundled fonts (pinned sources, lock entries, service-worker caching) | E6c |

## 8. Fixtures

`tools/fixtures/content_add.py` (no dependencies) writes `fixtures/content/add/`: a page turned 90° with a MediaBox off the origin and a `/UserUnit`; a page whose content ends with unbalanced `q`; a tagged page; a page with a running footer ("Page 3 of 24") for the overlap check; and a file with optional content already. The signed files in `fixtures/signed/` cover approval and P1–P3 certification. E6b checks its ops' outputs by reopening them (Lamina, qpdf, pdf.js, PDFium) and by the `place_content` example's byte golden, not a byte golden per op and fixture, whose bytes no reader looks at.

## 9. Tests

- **Unit:** each op's apply and undo, identity found after a save and reopen, a changed stream left alone, stacking order, tokens and Bates padding, fallback and `missing-glyph`.
- **Golden:** the example's byte golden; render goldens in PDFium, pdf.js and hayro (the parity matrix's content pages, every turn included), and a hayro check that no pixel outside an added object changes. (Amended by E6b: no byte golden per op and fixture.)
- **Round trip:** `cargo xtask roundtrip` gains the content ops; qpdf, pdf.js and PDFium open every output, and pdf.js and pdftotext read added text back.
- **Conformance:** veraPDF on PDF/A-2b, -2u and PDF/UA-1 files after each op.
- **Fuzz:** the content ops join the `ops_json` target.
- **End to end:** Playwright drives E6e's tools and E6g's dialog in the reader; WebdriverIO checks E6h's fonts on each desktop OS.
- **Performance:** E6f's budgets in the viewer harness.

## 10. Decisions on the open questions

1. **Are added objects editable after save?** Yes, through each form's `/PieceInfo` (§2.1). A marked-content tag such as `/LaminaPdfObj` would need a content parse to find, which the append API avoids.
2. **Watermarks: content form, annotation or both?** A content form with an artifact tag and an optional content group when the user asks (§2.3); no Watermark annotation in this round.
3. **Font sources on web: bundled or user-supplied only?** Decided by the owner on 2026-11-14 ([issue #532](https://github.com/laudebugs/laminapdf/issues/532)); the registry, the fallback chain and the symbol picker's data are E6c's, and the three OFL files are [E6i](../../todos/done/E6i-bundled-fonts.md)'s ([#831](https://github.com/laudebugs/laminapdf/issues/831)). The reader offers three sources. The standard 14 need no bytes and work in every module. A small set of OFL fonts (Lamina Sans, Newsreader, JetBrains Mono as the monospace, and Noto Sans Symbols 2 for A.12) is fetched from the reader's own origin when the font picker opens, each file the first time a text needs it, and the service worker keeps it (Newsreader, JetBrains Mono and Noto Sans Symbols 2 are not in the install's precache, so the 660 KB of Noto Sans Symbols 2 download only when a symbol needs it, and the picker names the font and its size first). Noto Sans Symbols 2 has the dingbats, shapes and some arrows and currency signs but not the mathematical operators or ₿; those draw in JetBrains Mono, the last resort in the chain, and nine symbols no bundled font has (⇐ ⇒ ⇔ ↩ ↪ ∆ ₪ ⅓ ⅔) are left out of the picker until Noto Sans Math or another font is added ([#839](https://github.com/laudebugs/laminapdf/issues/839)); bold and italic faces of Lamina Sans are [#838](https://github.com/laudebugs/laminapdf/issues/838). And the user can add their own files. The UI's WOFF2 files can't be used: they are Latin subsets in a format `laminapdf-text` doesn't read.
