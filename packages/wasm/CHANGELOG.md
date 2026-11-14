# @laminapdf/wasm

## 0.4.0

### Minor Changes

- 690f036: Crop pages and set page boxes. `doc.cropPages(ids, { top, right, bottom, left })` crops pages by margins of the page as it shows, so one set crops pages of mixed sizes and rotations by the same visible amount. `doc.setPageBoxes(ids, { media, crop, bleed, trim, art })` writes or removes (`null`) any of a page's five boxes in its own coordinates; each must lie inside the media box. A crop only hides content: the edit's `croppedContentRemains` lists the pages whose hidden text is still in the file, where extraction finds it.
  
  In Rust: the ops `CropPages` (with `CropMargins`) and `SetPageBoxes` (with `PageBoxes`), and `Applied::cropped_content_remains` and `Edited::cropped_content_remains`.
- 4037b81: Extract a document's images, links and outline as files:
  
  - `doc.extractImages(options)` (`extract_images` in Rust) gives the images the pages draw. JPEG and JPEG 2000 images come out as stored, byte for byte; every other image is decoded to a PNG with its `/Decode`, its colour space (grey, RGB, CMYK, Lab, Indexed, ICC-based, Separation) and its mask applied. Each image comes out once, on the first page that draws it, and one that can't be decoded is listed in `skipped` with why. In TypeScript it needs the extract or the text module; JBIG2 images, which the WebAssembly modules don't decode, are skipped there.
  - `doc.linksJson()` and `doc.outlineJson()` give the links and the outline in the formats `laminapdf.links/1` and `laminapdf.outline/1`, with their JSON Schemas in the `laminapdf` crate; `linksCsv()` and `outlineCsv()` give them as CSV. In Rust, `Document::links()` lists every page's links.
  - Streams filtered with `LZWDecode`, `ASCIIHexDecode`, `ASCII85Decode` or `RunLengthDecode` now decode: a page whose content uses one now has text, and its images come out.
- 1d12ce8: Add, change and delete links: `addLink` makes a link on an area or on lines of text, going to a page, a named destination or an `http`, `https` or `mailto` address, and `updateLink` and `deleteLink` change and remove one. `linkOnArea(page, area, target)` and `linkOnText(page, lines, target)` turn a drawn box or a selection's line boxes in page space into that edit. In a tagged document a new link joins the structure tree around the text it covers, and a file that claims PDF/UA-1 but isn't tagged refuses one (`link-not-tagged`). `pageLinks()` now gives each link's `id`, and an edit's `createdAnnotations` lists the links it added. The Rust crates have the same ops (`laminapdf::AddLink` and the rest) and methods.
- 2a11350: Highlight, underline, strike out and squiggle text, leave sticky notes and mark insertions with carets.
  
  `addAnnot` makes each mark with an appearance Lamina PDF draws itself, so every reader that draws stored appearances shows it the same. `markText(page, kind, lines)`, `noteAt(page, at, comment, icon)` and `caretAt(page, area)` turn a selection's line boxes, a click or a box in page space into that edit, and `replaceText(page, lines, replacement)` proposes a replacement as a strike-out and a caret grouped in one undo step. In a tagged document a mark joins the structure tree after its text, and its description is the text it marks. `updateAnnot` now changes a mark's `color`, `opacity` and geometry (`spec`), drawing it again. Saving warns when a mark breaks a PDF/A or PDF/UA claim: `annot-without-contents`, `annot-not-tagged`, `pdfa-annot-flags` and `pdfa-transparency`. The Rust crates have the same ops (`laminapdf::AddAnnot` and the rest) and methods; `laminapdf-build`'s `AnnotationAppearances` draws the appearances for code that uses `laminapdf-ops` directly.
- e183d4c: Copied and merged pages keep their tags. When you copy pages into a tagged document with `insertPagesFrom` or `replacePages`, or merge into one with `mergeDocuments`, the elements that hold their content join the document's structure tree, so a screen reader still reads them in order. A merge puts each file's elements under a `Part` element named after it, and `Document.merge` makes a tagged file when every input is tagged. A role, class or element ID the document already uses another way comes in as `<name>#2`, listed in the edit's `structureRenamed`. Copying pages into a file that claims PDF/UA-1 from one that doesn't warns with the new code `imported-pdfua-unchecked`. `extractPages` and `split` keep the tags of a tagged document too.
  
  In Rust: `Applied::structure_renamed` and `Edited::structure_renamed` with `StructureRename` and `StructureName`, `PendingChanges::pdfua_unchecked`, and `ConformanceWarning::ImportedPdfuaUnchecked`. `laminapdf-model`'s `StructTree` reads `/ClassMap`, `/IDTree` and `/Namespaces`, and each `StructElem` its `/ID` and classes.
- 6d048db: Resize pages and turn them to portrait or landscape. `doc.resizePages(ids, { width, height })` puts pages on new paper, scaling the page to fit and centring it, or with `{ mode: "extend", anchor }` keeping the content's size and placing it at an anchor. `doc.setOrientation(ids, "portrait" | "landscape", method)` turns each page shown the other way a quarter turn, by `/Rotate` (`"rotate"`, the default) or by swapping the paper's sides and turning the content into it (`"swapBox"`). The content moves as one piece, and everything that points into the page moves with it: annotations and form fields, outline items, named destinations, links' destinations and `/OpenAction`, and structure elements' Layout `/BBox`. A page whose content uses a filter Lamina PDF can't decode yet is refused with `unsupported-filter`.
  
  In Rust: the ops `ResizePages` (with `ResizeMode` and `Anchor`) and `SetOrientation` (with `PageOrientation` and `OrientationMethod`).

### Patch Changes

- 64bcf88: A stream encoded with `ASCIIHexDecode` or `ASCII85Decode` that holds characters outside the filter's alphabet now decodes: Lamina PDF skips those characters, as other PDF readers do, and raises the `filter-characters-skipped` warning. Before, the whole stream failed, so a page drawn from it had no text.
  
  When a text string, such as an outline title, holds a byte that PDFDocEncoding (PDF's default text encoding) leaves undefined, such as 0x11, Lamina PDF now reads it as that control character (U+0011), as pdf.js, qpdf and pypdf do, instead of U+FFFD.
  
  Lamina PDF now reads a dictionary that has a stray token, such as a number or `)`, where a key belongs. It skips the token, as pdf.js and PDFium do, and raises the `dictionary-key-skipped` warning. Before, the object failed to parse, and a damaged `/Encrypt` dictionary or page could stop the whole file from opening.
  
  An AES-256 file with a damaged `/OE` or `/UE` entry now opens with the password that doesn't use that entry: the user password when `/OE` is damaged, the owner password when `/UE` is.

## 0.3.0

### Minor Changes

- bc1bff5: Copy pages between documents. `doc.addSource(bytes)` opens another PDF as a source and returns its ID and page IDs. `doc.insertPagesFrom(id, pages, at)` copies its pages in, and `doc.replacePages(targets, id, pages)` puts them in other pages' places, keeping the bookmarks, links and named destinations that pointed at the old pages. A copy keeps its annotations and form fields. A field whose name is taken becomes `<name>#2`, or shares the value with `{ fields: "link" }`. A link to a page left behind is removed (`linksRemoved`). Copies come without tags (`tagsDropped`), and on a PDF/UA-1 file saving then warns with the new code `imported-tags-dropped`. `doc.extractPages(ids)` makes a new document from pages, with their labels, and `doc.exportRanges(ranges)` makes one per range.
  
  In Rust: `InsertPagesFrom`, `ReplacePages`, `Carry`, `FieldClash`, `Document::add_source`, `source_pages`, `extract_pages`, `export_ranges`, `export_ranges_with` and `ExportJob`, with `Edited::links_removed` and `Edited::tags_dropped`. `laminapdf-model` gains `FieldNames::clashes`, `Document::field_by_name` and `Outline::named_destination_value`.
- 2948e3f: Duplicate pages: `doc.duplicatePages(ids, at)` copies pages to a place in the document, as one edit that undo reverts. A copy shares its page's content but gets annotations of its own, and each form field it shows becomes a new field named `<name>#2` (or the next free number) with its own value. A signature's field isn't copied. The edit's `fields` lists each field the copy renamed or left out. In a tagged document the copy gets tags of its own. Content of an element that continues onto another page stays untagged on the copy, and the edit's `tagsIncomplete` lists that copy. On a PDF/UA-1 file, the save plan then warns with the new code `duplicate-tags-incomplete`.
  
  In Rust: `DuplicatePages`, with `Edited::fields` (`FieldChange`, `FieldOutcome`) and `Edited::tags_incomplete`. `laminapdf-model` gains `Document::field_name`, `Document::field_names` and `FieldNames::unique_partial`, the rule that names a copied field (a copy of `total#2` is `total#3`).
- 6e9dd39: Turn images into a PDF and export pages as PNG, JPEG or TIFF from TypeScript, as Rust already can:
  
  - `imagesToPdf(images, options)` makes a document with one page per image, as `laminapdf::images_to_pdf` does: JPEGs embedded as they are, each image drawn upright, and pages the image's size or a paper size (`fit: "A4"`, or `[width, height]` in points) with the image fitted inside the margins. It needs the extract or the text module (`init({ extract: true })`).
  - In the web reader's worker, `ViewerClient.exportPage(index, { format, dpi, quality })` draws a page as PNG or JPEG, and `exportTiff(indexes, { dpi, multiPage })` as TIFF, every page in one file or one file each. The resolution is recorded in each file. The TIFF writer is a WebAssembly module of its own, `@laminapdf/wasm/tiff`, loaded the first time a TIFF export starts.
  - In Rust, `laminapdf_render::TiffWriter` (feature `tiff`) writes a TIFF from pixels you drew yourself, a page at a time.
- d9b6acf: Choose how a PDF opens, and read its article threads. `doc.setInitialView(patch)` sets the page and zoom a file opens at, the panel beside the page (bookmarks, thumbnails, attachments, layers or full screen), the page layout and the reader window's requests, such as the title in the title bar. Every PDF reader reads these choices. `doc.initialView()` reads them back, page targets in page space, and `doc.threads()` lists the file's article threads with each part's page and rectangle. An open action that runs a script is reported, never run, and never replaced: an edit that names the opening page fails with `open-action-not-destination`. In Rust: the `SetInitialView` op, `Document::initial_view` and `Document::threads`.
- e416ac6: Merge and split documents. `Document.merge(files)` makes one document from several files. Each file gets a bookmark named by its `title`, holding the file's own bookmarks, and `{ outline: "keep" }` or `"none"` changes that. `doc.mergeDocuments(sources, at)` merges source documents into an open one as one edit. The files' form fields, named destinations, layers and page labels come along; a destination whose name is taken becomes `<name>#2` (`renamedDestinations`), and bookmarks to pages left behind are dropped (`bookmarksDropped`). Merging into a file that claims a PDF/A level from one that doesn't claim it warns with the new code `imported-pdfa-unchecked`. `doc.split({ count })`, `split({ ranges })` and `split({ bookmarks: level })` cut a document into new ones, each with the bookmarks that go to its pages.
  
  In Rust: `MergeDocuments`, `MergeSource`, `MergeOptions`, `OutlineMerge`, `MergeInput`, `RenamedDestination`, `SplitBy`, `Document::merge`, `merge_with`, `merge_job` and `MergeJob`, `Document::split`, `split_with` and `split_job`, with `Edited::renamed_destinations` and `Edited::bookmarks_dropped`. `Document::extract_pages` now carries the bookmarks that go to its pages and the layers.
- 2081146: Edit bookmarks and named destinations: `addOutlineItem`, `updateOutlineItem`, `moveOutlineItem` and `deleteOutlineItem` add, change, move and delete bookmarks, with their colour and bold or italic style, and `setNamedDestination`, `renameNamedDestination` and `deleteNamedDestination` manage the names links jump to. A rename follows the bookmarks and links that use the name. The outline's links and counts stay consistent after every edit, and a broken outline is rewritten whole by the first edit. `outline()` now gives each bookmark's `id`, the name or web address it goes to, and its style; `namedDestinations()` lists every name; `destinationAt(page, view)` turns a place in the viewer into a destination. The Rust crates have the same ops (`laminapdf::AddOutlineItem` and the rest) and methods.
- 8f21b98: Set a document's page labels, the names viewers show for its pages ("Cover", "iv", "A-3"):
  
  - `doc.setPageLabels(ranges)` replaces them in one edit, and `Transaction.setPageLabels` adds the same edit to a transaction. Each range names its first page by ID, with a style (`decimal`, `upperRoman`, `lowerRoman`, `upperLetters`, `lowerLetters`), a prefix and a first number; no ranges remove the labels.
  - Ranges that don't start on the first page, are out of page order or number from 0 fail with `invalid-label-ranges`, whose `data.what` says which rule.
  - `doc.pages[i].label` reads the new labels.
  
  In Rust: the `SetPageLabels` op with `PageLabelRange`, `LabelStyle` exported in every build, and in `laminapdf-model` `LabelRange::new`, `PageLabels::from_ranges`, `PageLabels::is_written` and the `tree-write` feature.
- bab4c9f: Organise a document's pages: insert blank pages, delete pages, move them, rotate them (to an angle, or by an angle from where each one stands) and reverse their order, each as one edit that undo reverts. Pages keep their IDs while they move. Deleting a page sends the bookmarks, links, named destinations and opening view that went to it to the nearest page left, showing the whole page. Ask instead and they lose their destination. Either way, the edit lists each one. The accessibility tags and form fields that only a deleted page used go with it; signature fields stay, so their signatures still count when you save. Page labels stay with their pages. Deleting every page is refused with the new code `no-pages-left`. A blank-page count or size out of range is refused with `invalid-op`.
  
  In Rust: `InsertBlankPages`, `DeletePages`, `MovePages`, `RotatePages` and `ReversePages`, applied with `Document::apply`; `Edited` gains `page_order` and `references`. `PageSize` moves to `laminapdf-model`, and `laminapdf-build` re-exports it, so its path through the builder and the façade is unchanged. Rotating the page that holds a signed signature's widget now makes the save a conflict, as deleting that page does.

## 0.2.0

### Minor Changes

- abd2520: `builder.flow(document, { data, fonts, images })` lays out a `laminapdf.document/1` document as tagged pages that flow. It needs `init({ text: true })`. The document is plain objects, typed by `FlowDocument`, so your editor catches a misspelt key or an unknown block `type`. A template with placeholders is a `FlowTemplate`: given `data`, `flow` fills it first, and a value the data lacks fails with `template-data-missing` and its pointer. The fonts and images the document names come by name as `Font`s and `Image`s. The new `invoice-from-json` example fills a two-page invoice from JSON.
- 97cbf89: In TypeScript, `export-failed` is a new error code: a page export's encoder failed. The functions below are Rust only for now; their TypeScript versions follow.
  
  In Rust, turn images into a PDF, and export pages as PNG, JPEG or TIFF:
  
  - `laminapdf::images_to_pdf(&images, &ImagesOptions)` makes a document with one page per image. JPEGs are embedded as they are, and each image is drawn upright, as its EXIF orientation says. Pages take the image's size at its resolution, or a paper size with the image fitted inside the margins (`ImagePages::Fit`). `Image::orientation()` reads the orientation.
  - `laminapdf_render::export_pages` (feature `export`) draws pages one at a time and encodes them as PNG, JPEG or TIFF at a resolution you choose, with the resolution recorded in each file. A multi-page TIFF holds every page in one file.
- 755d6ec: Edit a document's metadata with the Info dictionary and the XMP metadata kept in step:
  
  - `doc.setMetadata(patch, { at, replaceMalformed })` sets or removes the title, the authors, the subject, the keywords, the creating and producing applications and the dates. In PDF 2.0 files only the dates go to the Info dictionary.
  - `doc.removeMetadata(keys)` removes fields from both.
  - `doc.metadata()` reads both copies together and says where they disagree.
  - `doc.plan(options)` says how a save would go, without saving: the save constraint, the signatures, the refusal code, and the PDF/A and PDF/UA claims the edits break.
  - `doc.wouldViolate(op)` checks one edit against the signatures.
  - Saves take `metadataDate`, the time of the save, to stamp it as the modification date, and `instanceId` to set the XMP instance ID; either option turns the stamp on. The report says whether it happened (`metadataUpdated`, `metadataNotUpdated`).
  - `Transaction.setMetadata` and `removeMetadata` take the same options.
  - The `setInfo` op is deprecated in favour of `setMetadata`. It still applies, and now updates existing XMP metadata too.
  
  In Rust: `SetMetadata`, `RemoveMetadata`, `Document::metadata`, `Document::plan`, `Document::would_violate` and `SaveOptions::with_metadata_date`; `SetInfo` is `#[deprecated]`.
- 9f555a2: Groundwork for copying pages between documents:
  
  - `unknown-source` is a new error code: an op or a call named a source document that wasn't added to the session.
  
  In Rust: `laminapdf_ops::Document::add_source(bytes, password)` opens another PDF as a source document and returns its `SourceId`, the SHA-256 of its bytes, so the same file has the same ID on every device. `source_document(id)` returns it, and `blobs()` the session's `BlobStore`. `laminapdf_model::ObjectCopier` copies pages, and everything they reach, from one document into another, sharing what several pages use, within `Limits`. The ops that use them, inserting pages from another file and merging, come next.

### Patch Changes

- e5fa57d: `builder.markdown` and `builder.flow` now lay a tab in a code block out as spaces to the next multiple of four columns, so the extracted text lines up as drawn. A tab anywhere else in the text becomes one space. Before, a font with CFF outlines (an `.otf` font) raised a `tounicode-conflict` warning for each code block line with a tab.
- 2e74d7d: Every error's and warning's `docsUrl` now opens its page on the docs site, `https://docs.laminapdf.com/reference/codes/<code>/`, instead of the Markdown file on GitHub. The code still ends the path.
- 8ae1bae: `FlowTemplate` is now an interface rather than a type alias, with the same keys and the same checks, so the API reference names it where `builder.flow()` takes it instead of spelling out every key of every block. Its reference page explains the placeholders, with an example. Every type declaration in the TypeScript reference is now a code block, one key or union case to a line.
- cafed9f: `0.2.0` is the first release that isn't an alpha. A plain `npm install @laminapdf/sdk` or `cargo add laminapdf` now installs it, and from here on releases follow SemVer as the [stability policy](https://docs.laminapdf.com/reference/stability/) says: a breaking change raises the minor version (`0.2` to `0.3`), so an upgrade within `0.2.x` needs no change to your code. The `alpha` npm tag stays on `0.2.0-alpha.6`.
- bd0148e: The packages' descriptions, keywords and README openings now say what each package does for you, and each package links to the issue tracker.
- 82d3c07: In Rust, `Reveal::redaction_warnings()` gives the warnings `Document::redaction_warnings` would (`covered-text`, `redaction-not-applied` and `text-in-earlier-revision`, in the same order and with the same counts) from what `Document::reveal` already found. When you need both, call `reveal` and read the warnings from its result. The file's pages and earlier revisions are then read once instead of twice.

## 0.2.0-alpha.6

### Minor Changes

- 810d0d4: Rust callers can now describe flowing pages as JSON (TypeScript support follows), in the format `laminapdf.document/1`, and lay them out with `Builder::document`. A document names its page, its text styles and its blocks (paragraphs, headings, lists, tables, figures, a contents page and more), and its fonts and images by name, with their bytes given beside it. Its JSON Schema ships in `docs/schemas/document.schema.json`.
  
  A template is a document with placeholders that `flow::document::fill` replaces from data: `$data` for a value, `$each` to repeat a block or table row per item, `$item` for a value of the current item, and `$if` to keep a block only when a value is there. Templates have no expressions, so the data arrives formatted.
  
  A document with an unknown key, a missing one or a value of the wrong kind fails with `invalid-document`, which names the value's place in `data.pointer` and suggests the closest key in `didYouMean`. A template whose data lacks a value fails with `template-data-missing`, and data of the wrong kind with `template-data-type`.
- 64f7860: Turn Markdown into a PDF from TypeScript: `builder.markdown(text, options)` turns CommonMark (with tables, footnotes, strikethrough and task lists) into tagged pages that flow, and takes the title and language from the front matter. Fonts come by role (`regular`, `bold`, `italic`, `boldItalic`, `mono`, `fallbacks`) and images by the name the Markdown uses; nothing is fetched. `options.page` sets the page size, margins, header and footer, and `contents` adds a contents page. `Font.fromCollection(bytes, index)` loads one face of a font collection (`.ttc`).
- 70eb584: Undo and redo say which pages to draw again. `doc.undo()` and `doc.redo()` now return `{ outcome, touchedPages }` instead of the outcome alone, so a viewer can redraw only the pages the undone or redone edit changed. `doc.apply(ops)` also says whether the edit `changed` anything: an edit that only sets values the document already had leaves nothing to undo. `doc.isEncrypted` tells you whether the file is encrypted. The Rust `Document::undo` and `redo` return the same `Reverted` record, and `Edited` has `changed`.

## 0.2.0-alpha.5

### Minor Changes

- 526aae3: Reader mode for every PDF. When a file has no tags, or its tags fail reader mode's checks, `doc.reflow()` reads the page layout instead of refusing: `source` is `"approximate"` and `reason` says why the tags weren't used. A scanned page, or one whose column order extraction doubts, comes back as a `page-image` block with `why` (`"scanned"` or `"uncertain"`) and the page's text for searching. Tables and pictures are crops, and their captions are kept with them. Only a file with nothing to show has `available: false`, with the reason `"nothing"`.
- 771fafd: Pages can be drawn in Sepia, Dark or High contrast instead of their own colours. Images and annotations keep theirs, and so does text under a highlight.
  
  - **Render request:** `RenderRequest.appearance` is `"original"` (the default), `"sepia"`, `"dark"` or `{ kind: "high-contrast", ink, paper }`, for both backends.
  - **Colour maps:** `recolourRgb` and `recolourTile` in `@laminapdf/render` give the same bytes as the Rust crate's.
  - **Viewer:** `setAppearance()` redraws the pages in view in the new colours, keeping the scroll position. The thumbnails follow.
- 5fc1f96: `doc.properties()` reads what a properties view shows about a document in one call. It returns the Info entries, with the creation and modification dates as ISO 8601, and the XMP packet as text. It also gives the PDF version, page count, file size, whether the file is linearized and tagged, and the fonts the pages use (name, type, embedded, subset, encoding). Encryption comes with the author's permissions, and signatures are counted, never verified. `contains` says whether the file holds JavaScript, attachments or a form. It needs the extract or text module. For Rust callers, `Document::properties()` returns the same as `DocumentProperties`, and `laminapdf-model` gains `Pages::fonts()` and `Document::xmp_packet()`.
  
  The viewer's client gains `properties()`, `printPage(index, { dpi, annotations })` and `fileBytes()`. `printPage` draws a page in the document worker for printing, with annotations as their print flags say, and returns it as a PNG. `fileBytes` returns a linked file's bytes as opened.
- 95d2a70: Reader mode for tagged PDFs. `doc.reflow()` returns the document's blocks in its structure tree's order: headings, paragraphs, list items, quotes, captions and notes as text, and each figure, formula and table as a crop of its page with its alternate text, never as reflowed text. It checks the tree first: a file whose tree it can't trust comes back with `available: false` and the reason. It needs the extract module. The viewer's worker serves it as `client.reflow()`.
- 513aeb3: Links in PDFs now work in the viewer. `doc.pageLinks(index)` lists a page's links in page space, each with what following it does: a place in the document, a web address, another file, a named action, or an action such as JavaScript, which Lamina PDF never runs. The viewer draws an anchor over each link. Links inside the document go there, web links ask the app to confirm first, and anything else does nothing. `viewer.history` records each jump, so Back returns to where the reader was, and the web reader keeps those jumps in the browser's history.

## 0.2.0-alpha.4

### Minor Changes

- 00bf63b: Extraction now leaves out text painted over by a later shape, image or annotation, as a reader can't see it either. A black box drawn over two words of a line takes those words out and keeps the rest of the line, so `Pay [box] now.` reads `Pay now.`. Covered text is listed under `dropped` with the new reason `covered`, its box and how many characters it held, never its text; `drop: { keepSpans: true }` shows it to an auditor. Only opaque paint drawn after the text covers it. A translucent box, a highlighter, shading under the text, a box in an optional layer and a scanned page's image over its own text layer all leave the text in. For Rust callers, `Flags` gains `covered`, `DropReason` gains `Covered`, and `BlendMode` now lives in `laminapdf-model` (`laminapdf::BlendMode` is unchanged).
- 915c253: Running headers, footers and watermarks can now say what they are. `surface.artifact("footer", s => …)` marks its content as a footer attached to the bottom of the page (`/Subtype /Footer /Attached [/Bottom]`), `"header"` as a header attached to the top, and `"watermark"` as a watermark. `"pagination"` still writes the plain form. In Rust it is `Artifact::footer()`, `Artifact::header()` and `Artifact::watermark()`, with `with_attached` for other edges.
  
  A paragraph split by a page break can now stay one paragraph for screen readers. In Rust, `surface.open_element(TagKind::P)` places it, `surface.resume(id, |s| …)` draws its content on that page or a later one, and `builder.close_element(id)` writes it.
  
  `surface.measure_text_box(…)` (Rust) returns the layout `text_box` would draw, without drawing it.
  
  Arabic text with diacritics now builds under PDF/UA-1 and PDF/A-2u: a letter the font draws as a body and a dot no longer fails with `glyph-without-unicode`.
- 4fdbfd4: Rust flows can now have footnotes, a contents page and bookmarks. A footnote is a raised number in the text, numbered through the flow, with its note at the foot of the same page; a line whose notes don't fit moves to the next page with them. A contents page lists the headings with dot leaders, the page each one starts on and a link to it. The flow is laid out a second time to fill in the page numbers, and the new error code `flow-layout-unstable` reports a fault if they ever fail to settle. Every heading gets a bookmark in the document's outline, nested by level, which `Flow::with_outline(false)` turns off. Tagged documents can also use the `TOC`, `TOCI` and `Reference` structure types, which `@laminapdf/sdk` now accepts as tag names.
- f69228f: Rust callers can now pour a document into as many pages as it needs. `Builder::flow` takes blocks (headings, paragraphs of styled text and links, lists, figures, spacers, page breaks and groups kept together) and a page template. The template sets the margins, a header, a footer and an optional different first page. Paragraphs split only between lines, leaving at least two lines at the foot and the head of a page. Headings stay with what follows them, and figures move whole. Headers and footers can show `{page}`, `{pages}`, `{section}` and `{title}`, so "Page 3 of 12" is right on every page. The output is tagged: each paragraph stays one element across pages, and headers and footers are marked as page furniture. New codes: `flow-content-too-large` for a line or figure taller than a page, and the warning `flow-keep-broken` for blocks kept together that can't fit on one page. The TypeScript package doesn't have flows yet.
- 2254184: Rust flows can now hold tables that run across pages. A table's header rows repeat at the top of every page it continues on, no body row is split between pages, and its footer rows, such as a total, stay on the page of the last body row. Each column takes a fixed width, a share of the free width, or its widest cell's width, and the widths are worked out once so the columns line up on every page. Cells hold paragraphs, can span columns and align their text; a table sets its cells' padding and can have a frame, rules and a shaded header. The output is tagged as one table across all its pages, and the repeated header rows are marked as page furniture, so screen readers and text extraction read the header once. Tagged documents can also use the `THead`, `TBody` and `TFoot` structure types, which `@laminapdf/sdk` now accepts as tag names.
- c74d78f: Rust callers can now turn Markdown into flowing, tagged pages: `Builder::markdown(text, &MarkdownBuildOptions)` reads CommonMark with tables, footnotes, strikethrough and task lists, takes the title, language, author and subject from front matter, and lays the document out with the flow layer. Links to `#a-heading` go to that heading, and a table runs across pages with its header repeated. Images come only from the caller, through `MarkdownBuildOptions::with_image`; nothing is fetched. Raw HTML and images that weren't passed are left out with the warnings `markdown-html-ignored` and `markdown-image-missing`, which give the position in the Markdown, never its text.
  
  If you pass no fonts, the builder sets the document in Helvetica and Courier, which flows can now use. They set Latin text only, and a PDF/UA-1 build refuses them.
  
  Flows also gain code spans and code blocks, struck-out text, block quotes (`BlockQuote`), thematic breaks and lists that start at any number. The `laminapdf` crate's new `markdown` feature is on by default.
- cc7f9ce: Text runs and text boxes now say where the next element can start. What `surface.text()` and `surface.textBox()` return has a `bottom`: the bottom of the last line that draws something, where the next text box's top can go. In Rust it is `TextRun::bottom()` and `TextLayout::bottom()`. It is left out when nothing is drawn, such as empty text, text of only white space, or a box too short for one line, so an empty section adds no gap. A clipped box gives its last drawn line; check `overflow !== undefined` before treating the text as placed.
- 703c2d8: The Rust façade reads a document's outline (`Document::outline()`) and named destinations (`Document::named_destination(name)`), each target a page index and a view in page space. The extract and text modules give them to JavaScript as `WasmDocument.outline()` and `namedDestination(name)`.
- 60495bf: `doc.structureTree()` reads a tagged document's structure tree: its elements in reading order, down to the marked content each owns on a page. Each element gives its standard type (`H1`, `P`, `LI`, `Figure` …), alternate text, language and actual text; custom types resolve through the role map. A text run's `mcid` links it to its element, so a viewer can give assistive technology a document's headings, paragraphs, lists and tables.
  
  `doc.isTagged` says whether the document claims to be tagged. Both need the extract or text module, as `page.text()` does. For Rust callers, `Document::structure_tree()` returns `StructureNode`s.
- 2886263: Rust callers can now lay out a paragraph whose words change font, size or language: a bold word inside a sentence, a superscript, an Arabic name in an English line. `laminapdf_text::break_spans` takes the text and a list of `StyledSpan`s, each covering a stretch of it with its own fonts, size, baseline rise, language and OpenType features. Lines still break only where the text allows, so a style change inside a word never splits it. Spans of one style are shaped together, so kerning carries across them. Each line reports its ascent and descent from its tallest run, and `pdf_line_run` draws a superscript's rise. `break_lines` is unchanged. Spans that don't cover the text once, in order, that split a letter from its accent, or that name no font or a font past the list fail with the new error code `invalid-spans`.

## 0.2.0-alpha.3

### Minor Changes

- 110d8b9: `doc.extract({ format })` gives a document's text for an LLM or a search index: Markdown or plain text in reading order, the `laminapdf.extract/1` JSON with a box for every block (the same bytes as Rust's `extract_json`), or chunks sized for retrieval. Tables drawn with lines come out as tables, and hidden text is left out. It needs the extract or text module.

## 0.2.0-alpha.2

### Minor Changes

- 32a4963: The WASM module's `beginTag` and `beginArtifact` now take a structure type, table scope and artifact kind by name instead of by index, which breaks code that calls the module directly; `@laminapdf/sdk` already calls them that way. `@laminapdf/sdk` also exports the `LineCap`, `LineJoin` and `TableScope` types that `StrokeStyle` and `TagSpec` use, and `laminapdf-build`'s `FillRule`, `LineCap`, `LineJoin`, `BlendMode`, `ArtifactKind` and `TableScope` gain an `ALL` list of their variants.

## 0.2.0-alpha.1

### Minor Changes

- 5c2624e: The library is called Lamina PDF, and its public names follow. `@laminapdf/sdk`'s error class is `LaminaPdfError`, checked with `isLaminaPdfError`, and its warnings are `LaminaPdfWarning`. The error object's JSON Schema is `laminapdf.error/1`. `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro` (`.js`, `.d.ts` and `.wasm`).

## 0.2.0-alpha.0

### Minor Changes

- 2b4384a: Accessible and archival PDFs. A `profiles` option on the builder (`PDF/UA-1`, `PDF/A-2b`, `PDF/A-2u`, `PDF/A-3b`; `BuildOptions::with_profile` in Rust) checks each machine-checkable rule the document breaks when it finishes, with a coded error per rule: `missing-title`, `missing-lang`, `untagged-content`, `missing-alt-text`, `heading-level-skipped`, `unnumbered-heading-misused`, `invalid-structure-nesting`, `link-without-structure`, `link-without-contents`, `font-not-embedded`, `glyph-without-unicode`, `device-color-without-output-intent`, `embedded-file-forbidden`, `embedded-file-without-mime-type` and `profile-version-conflict`. The builder writes the XMP identification, `/DisplayDocTitle` for PDF/UA-1 and, for PDF/A, an sRGB output intent with a 480-byte embedded ICC profile. `attach(name, bytes, mime, { relationship })` adds associated files for PDF/A-3b, such as an e-invoice's XML. Every example that claims a profile is validated by veraPDF each night.

## 0.1.0-alpha.0

### Minor Changes

- First alpha, `0.1.0-alpha.0`, versioned and tagged but not published (ADR 0017). The `lamina` façade and its core crates at the same version: open, read, edit and save PDFs (incremental and full rewrites, the save planner, undo/redo), generate them (pages, vectors, colour, images, standard 14 and embedded fonts with shaping, bidi and line breaking, links, outlines, metadata, tagged PDF), and extract page text; `@laminapdf/sdk` over the base, text and extract WebAssembly modules in browsers, Node, Deno, Bun and Cloudflare Workers.
