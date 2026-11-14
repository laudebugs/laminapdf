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
| `UpdateContentObject` | `{ id: ContentId, modified, matrix, opacity, crop, fill, stroke, alt, order: Option<i32> }`, each field optional; a replaced image is a new `blob` | E6b |
| `DeleteContentObject` | `{ id: ContentId }` | E6b |
| `AddText` | `{ id: ContentId, page: PageId, rect: RectPt, runs: Vec<TextRunSpec>, align, fonts: Vec<FontRef>, tag: Option<TagChoice>, modified }`; a `FontRef` is `Blob(BlobId)` or `Standard(Standard14)`; editing and deleting added text is `UpdateContentObject` and `DeleteContentObject` | E6c |
| `AddWatermark`, `AddBackground` | `{ id: ContentId, pages: Vec<PageId>, mark: Text { text, font: FontRef, size, colour } \| Image { blob: BlobId }, opacity, angle, hideable: bool, modified }` | E6d |
| `SetHeaderFooter` | `{ id: ContentId, pages: Vec<PageId>, slots: [Option<String>; 6], font: FontRef, size, margins, date: String, file: String, modified }`; tokens are expanded when the op applies (§2.3) | E6d |
| `ApplyBates` | `{ id: ContentId, pages: Vec<PageId>, prefix, start: u64, digits: u8, suffix, slot, font: FontRef, size, modified }` | E6d |
| `UpdatePageMark`, `DeletePageMark` | `{ id: ContentId, … }`: a changed mark is the new payload in place of the old | E6d |

**No `AppendContent` op.** E2e left an `AppendContent` op to this round. A wire op that carries raw content bytes would let any caller write unbalanced or unsafe operators, so there is none: every op is typed. The ops share one private function in `laminapdf-ops`, `place_form(page, form, matrix, layer)`. It wraps the page once with E2e's `PageContentWriter` and appends the object's drawing stream, so `laminapdf-ops` gains a dependency on `laminapdf-content`. E5e's flattening, E8g's form flattening and E2s's page resizing call it from their own ops; whichever of them lands before E6b writes it, and E6b reuses it. Images and fonts reach it as `BlobId`s from the blob store.

**Drawing the form goes through a trait.** `laminapdf-build` depends on `laminapdf-ops`, so no op can call the builder's `Surface`, as the [E4 plan §2.2](e4-markup-and-drawing-annotations.md#22-where-appearances-are-generated) found for appearances. E6b extends E4's `Appearances` trait, which `laminapdf-ops` defines and `laminapdf-build` implements, with a method that draws a `ContentObjectSpec`, a text block or a page mark into a new Form XObject through the op's `CosMut`; the façade passes the builder's implementation in, as it does for annotations. `laminapdf-text` stays out of `laminapdf-ops`: shaping happens inside the implementation.

**One object, one form, one stream.** Each added object is its own Form XObject (ISO 32000-2 §8.10), drawn by a stream of its own in the page's `/Contents`, after the wrapped content for `Front` or before it for `Behind`. The stream has exactly this grammar, and the scanner accepts nothing else:

```
q a b c d e f cm [/GSn gs] [/Tag <<…>> BDC] /FmN Do [EMC] Q
```

`gs` sets the opacity; `BDC … EMC` is the artifact or the structure element's marked content (§3). Moving, resizing or fading an object rewrites that one small stream; deleting it removes the stream from `/Contents` and the name from the resources. Nothing reads the page's own operators. Stacking order (Bring forward and the rest) reorders added streams among themselves; nothing goes behind original content except a `Behind` object, and E7 handles stacking among existing objects.

**Identity after a save.** Each form carries a page-piece dictionary (ISO 32000-2 §14.5): `/PieceInfo << /LaminaPDF << /LastModified … /Private << /Id (…) /Kind /Image >> >> >>`. When a file is reopened, the scanner matches each stream of `/Contents` against that grammar and reads the `/PieceInfo` of the form it draws. A stream or form someone else changed no longer matches, so it is left alone as original content. Acrobat marks the watermarks and headers it adds the same way (`/ADBE_CompoundType` in `/PieceInfo`). `laminapdf reveal` lists these entries under metadata, as it lists anyone's `/PieceInfo`.

### 2.2 Text

`AddText` lays out its runs with `laminapdf-text` (`break_spans`, shaping, bidi) inside the form, embeds subset fonts with ToUnicode maps, and falls back along `fonts` for characters the first font lacks; a character no font has is `missing-glyph`, never a silent `.notdef`. A font whose `fsType` forbids embedding is refused with `font-not-embeddable`. Shaping needs the text module's code, so `AddText` in a document opened in the base module fails with `appearance-needs-text`, E4's code. The viewer opens documents in the extract module, which E4e gives the builder's fonts; E6c reuses them.

### 2.3 Page marks

- **Watermark and background.** A watermark is drawn in front of the content and a background behind it, each as a form on every chosen page. In a tagged file the drawing is an artifact (`/Artifact << /Type /Pagination /Subtype /Watermark >>`, §14.8.2.2). With `hideable` the form gets an `/OC` entry naming an optional content group with a `/Usage << /PageElement << /Subtype /FG >> >>` (or `/BG`; §8.11.4.4), so a reader can switch it off. Watermark annotations (§12.5.6.22) are not used: a background can't be one, and one kind of mark keeps Manage page marks simple.
- **Header and footer.** Six slots (left, centre, right; top and bottom). Tokens follow T4b's syntax, which landed first: `{page}`, `{pages}`, plus `{label}` (the page label), `{date}` and `{file}`, whose values the payload carries. The dialog shows them as `[Page]`, `[Pages]` and the rest and writes the braces. They are expanded when the op applies, so a later page move doesn't renumber them; the dialog says so.
- **Bates numbers.** Prefix, start, digits and suffix, in one slot; across files (E14d) each file's op carries its own start.
- **Overlap check.** Before Apply, A03's notice comes from `laminapdf-extract`'s runs in the header and footer bands (the top and bottom 10 % used for running headers), so it needs the extract module, where the viewer already opens documents.

## 3. Conformance the edits must keep

- **Tagged PDF and PDF/UA-1.** Added text becomes a `P` element, or an artifact when the user chooses. Its marked content sits around the `Do` in its drawing stream, so its MCID is the page's and the page's `/StructParents` entry in the parent tree lists it. Marked content inside the form would need the form's own `/StructParents` and MCRs with `/Stm`, which the one-stream grammar avoids. An image gets a `Figure` with its alt text, or is an artifact when marked as decoration; shapes are artifacts unless given alt text. On a file that claims PDF/UA-1, an image whose `alt` is `None` is refused (`missing-alt-text`, B8b's code); `Decorative` makes it an artifact. Page marks are always artifacts.
- **PDF/A.** Every font added is embedded, colours use the file's output intent or carry an ICC profile, and transparency is refused on PDF/A-1 (ISO 19005-1 §6.4): an opacity below 1 or an image with an alpha channel raises `pdfa-transparency`, a new code E6b adds to `codes.json` with its `docs/errors` page. On PDF/A-2 and -3 an alpha channel becomes an SMask, which they allow.
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

## 7. Todos

| Todo | What | Depends on |
| --- | --- | --- |
| [E6b](../../todos/ready/E6b-place-images-and-shapes.md) | `place_form`, `AddContentObject`, `UpdateContentObject`, `DeleteContentObject`, identity, `added_objects`, `pdfa-transparency` | E2e, E4c, E6a |
| [E6c](../../todos/ready/E6c-add-text.md) | `AddText`, fallback, tagging, web font sources | E6b, E4e |
| [E6d](../../todos/ready/E6d-watermarks-headers-footers.md) | The four page-mark ops with update and delete, tokens, OCG, overlap check | E6c |
| [E6e](../../todos/ready/E6e-content-tools-ui.md) | Edit-mode tools, handles, snapping, inspectors | E6b, E6c |
| [E6f](../../todos/ready/E6f-reload-budget.md) | Touched-page reload time measured and budgeted | E6b |
| [E6g](../../todos/ready/E6g-page-marks-dialog.md) | The Page marks dialog and Manage page marks | E6d, E6e |
| [E6h](../../todos/ready/E6h-desktop-system-fonts.md) | Fonts on the computer in the desktop font picker | E6c, E6e |

## 8. Fixtures

`tools/fixtures/content_add.py` (no dependencies) writes `fixtures/content/add/`: a page turned 90° with a MediaBox off the origin and a `/UserUnit`; a page whose content ends with unbalanced `q`; a tagged page; a page with a running footer ("Page 3 of 24") for the overlap check; and a file with optional content already. The signed files in `fixtures/signed/` cover approval and P1–P3 certification. Reopened outputs of each op become byte goldens through `golden --update`.

## 9. Tests

- **Unit:** each op's apply and undo, identity found after a save and reopen, a changed stream left alone, stacking order, tokens and Bates padding, fallback and `missing-glyph`.
- **Golden:** byte goldens of each op on the fixtures; render goldens in PDFium and hayro, the rotated page included.
- **Round trip:** `cargo xtask roundtrip` gains the content ops; qpdf, pdf.js and PDFium open every output, and pdf.js and pdftotext read added text back.
- **Conformance:** veraPDF on PDF/A-2b, -2u and PDF/UA-1 files after each op.
- **Fuzz:** the content ops join the `ops_json` target.
- **End to end:** Playwright drives E6e's tools and E6g's dialog in the reader; WebdriverIO checks E6h's fonts on each desktop OS.
- **Performance:** E6f's budgets in the viewer harness.

## 10. Decisions on the open questions

1. **Are added objects editable after save?** Yes, through each form's `/PieceInfo` (§2.1). A marked-content tag such as `/LaminaPdfObj` would need a content parse to find, which the append API avoids.
2. **Watermarks: content form, annotation or both?** A content form with an artifact tag and an optional content group when the user asks (§2.3); no Watermark annotation in this round.
3. **Font sources on web: bundled or user-supplied only?** Decided by the owner on 2026-11-14 ([issue #532](https://github.com/laudebugs/laminapdf/issues/532)). The reader offers three sources. The standard 14 need no bytes and work in every module. A small set of OFL fonts (Lamina Sans, Newsreader, a monospace and Noto Sans Symbols 2 for A.12) is fetched from the reader's own origin when the font picker opens. And the user can add their own files. The UI's WOFF2 files can't be used: they are Latin subsets in a format `laminapdf-text` doesn't read.
