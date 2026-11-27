# @laminapdf/sdk

## 0.8.0

### Patch Changes

- @laminapdf/wasm@0.8.0

## 0.7.0

### Minor Changes

- 1e05504: You can now move comments between Lamina PDF and other PDF apps. `doc.exportAnnotations({ format: "xfdf" })` (`export_annotations` in Rust) writes the document's comments as XFDF, which Acrobat and most PDF apps import. `"fdf"` writes FDF instead, and `"markdown"` writes Markdown for a notes app, each highlight's text as a quote. XFDF and FDF keep every reply with its comment, carry attached files, and record which PDF they were made for. Exporting changes nothing in the document.
  
  `doc.importAnnotations(bytes)` (`import_annotations` in Rust) brings an XFDF or FDF file's comments in as one change, so a single undo takes them all back. By default it leaves the comments the file names (by `/NM`) as they are and adds the file's other comments; `{ mode: "replace" }` replaces each comment the file names with the file's version, and `{ mode: "add" }` adds the file's comments as copies. Each comment gets an appearance and keeps the pop-up the file places. `doc.previewImport(bytes)` says what an import would do first: how many comments it adds and where, what it leaves out, such as a sound or a form field's value, and whether the file was made for another PDF.
  
  A file that can't be read fails with `interchange-malformed`, naming the line where reading stopped. An XFDF file that declares a DTD is refused with `interchange-dtd`, since a DTD can define entities that expand without end or read other files.
  
  In Rust, `AnnotCommon::popup` places a comment's pop-up, and `AnnotSpec::from_entries` and `AnnotCommon::from_entries` read what an annotation dictionary describes.
- d44da10: You can now flatten a filled form. `doc.flattenForm(undefined, "print")` (`flatten_form` in Rust) draws each field's answer into its page and removes the field, so the copy shows the same in every reader and the answers can no longer be changed. Pass field IDs from `formFields()` to flatten only those fields. One undo brings the fields back.
  
  Signature fields are never flattened, and neither are fields a signature locks when you leave the IDs out. A push button or radio button with no picture of its own stays a field, and `warnings` lists it as `flatten-no-appearance`. A form's XFA part goes too, and `warnings` says so with `xfa-removed` when fields other than signatures are left. `doc.flattenForm` needs the extract or text module; the `flattenForm` op, given field IDs, works with any module.
- fd817df: Forms now format, check and add up their values as they do in Acrobat, without running any JavaScript. Most form scripts only call Acrobat's built-in helpers. Lamina PDF reads those calls as data and does their work itself: a number field shows `$1,234.50` while its value stays `1234.5`, a date field shows its date in the form's pattern, and a total updates as soon as a field it adds up changes.
  
  `setFieldValue` now refuses text that breaks a field's format or range, with `field-value-invalid` and the reason `format` or `range`. The same edit updates every total that depends on the changed value, so one undo reverts the value and its totals. A total that a signature locks keeps its value, and the edit lists it in `skippedCalculations`. The new `setFieldHidden` edit hides a field or shows it again, as a form's Hide button does.
  
  `doc.fieldBehaviour(id)` says what a field's helpers do, `doc.validateField(id, text)` checks a value before you set it, and `doc.calculate()` brings stale totals up to date. `doc.formScripts()` lists every script in a file, never its text, each marked `handled`, `not handled` or `xfa`. The new `maxScriptLength` limit (64 KiB) caps the bytes of one script Lamina PDF reads. A longer script counts as not handled.
- d17a018: You can now fill a form from data, move its answers to and from other apps, and reset it. `doc.fillForm({ fields: { "customer.name": "Ada", paid: true } })` (`fill_form` in Rust) fills each field by its full name in one change, so a single undo takes every answer back. The form's calculations run once, after the last answer. A check box takes `true` or `false`, a list an array of its options, and `null` empties a field.
  
  What can't be filled is listed, and the rest is filled. The result names the fields the form doesn't have (`unknown`), the fields a signature locks (`locked`), and the values a field refuses (`refused`, each with its code, such as `field-value-invalid` for text that is too long).
  
  `doc.exportFormData("json")` (`export_form_data`) writes the form's answers in the same shape `fillForm` takes, so an export fills a copy of the form as it was. Its schema is `docs/schemas/form-data.schema.json` in the `@laminapdf/sdk` package. `"xfdf"` and `"fdf"` write the formats Acrobat and most PDF apps import, and `doc.importFormData(bytes)` (`import_form_data`) reads all three. `doc.resetFields()` (`reset_fields`) gives every field its default value, or empties it, as a form's Reset button does.
  
  `importFormData` reads the form fields in XFDF and FDF files, including those other apps write. `importAnnotations` still skips the fields and lists them, so form data and comments stay separate.
- 04aa17a: A filled form now shows its new values. The `setFieldValue` edit draws a text field's, combo box's or list box's widgets again in the field's own font, size, colour and alignment. Each widget keeps its own layout: text that fits its box, several lines, comb cells, a password's bullets, rows chosen in a list, or a turned widget. Check boxes and radio buttons switch between the states they already have, and one with no appearance gains a check mark or dot.
  
  A character the field's font can't draw needs a font of your own. Add its bytes with `doc.addBlob` and list the ID in the edit's new `fonts`; the font is embedded with the form. Without one, the edit fails with `annotation-glyph-missing`. On a PDF/A file, that font draws the whole value, so the file keeps its claim. The base WebAssembly module has no fonts, so it refuses every text, combo and list fill with `appearance-needs-text`; the extract and text modules draw them.
- 175fd59: You can now read a form's fields and fill them in. `doc.formFields()` (`form` in Rust) lists every field with its name, kind, value and options, and its widgets with their page and box. It also lists widgets that no field reaches and fields that no page shows. `doc.tabOrder(index)` (`tab_order` in Rust) gives the order Tab visits a page's widgets in, as the page's `/Tabs` asks.
  
  The new `setFieldValue` op, applied with `doc.apply`, gives a text field, check box, radio group, list or combo box a value, or clears it. It keeps each button's state and a list box's chosen options in step. The op fails with `field-value-invalid`, saying why, when the value doesn't fit the field. It fails with `field-is-signature` on a signature field, and with `unknown-field` on a field the form doesn't have. The widgets keep the appearance they had until a later release draws the new value.
  
  On a signed form, a signature's field lock now counts only against the fields it covers: filling a field outside the lock saves as an incremental update that keeps the signature valid.
- 7c344c5: Place images, shapes and pages of other PDFs on a page as page content, then move, fade, crop, restack or remove them, even after the file is saved and reopened. Unlike a stamp or a comment, the object is part of the page: it prints and every reader draws it. `doc.content.add(index, object)` adds one under an ID of your own: a JPEG or PNG (a PNG keeps its transparency), a line, rectangle, ellipse or polygon, or a page of another PDF, kept as vectors and text. `doc.content.update(index, id, changes)` and `doc.content.remove(index, id)` change and remove an object, and `doc.content.added(index)` lists what a page holds, with each object's box. In a tagged file an object becomes a figure with its alternate text, or an artifact when it is decoration.
  
  Four new codes say what went wrong: `content-id-in-use`, `unknown-content-object`, `content-object-invalid` (bytes that aren't a JPEG, a PNG or a PDF with the page named) and `content-needs-alt-text` (a picture without alternate text in a PDF/UA-1 file). The `pdfa-transparency` warning now also lists the pages with see-through objects you added.
- c465c12: `updateAnnot` can now replace an attached file in place. Give the `fileAttachment` patch a new `file` (its blob ID, name and type), and the annotation keeps its place, icon, replies, note, author and dates. Before, you had to delete the annotation, replies included, and attach the file again.
- 032996a: Stamp a page, attach a file to a spot on it, and play sound comments. `doc.stampAt(page, area, stamp)` gives the edit that adds a stamp. A stamp shows one of sixteen standard names such as `approved` or `confidential`, drawn with no font; a JPEG or PNG added with `addBlob`; or dynamic lines such as "APPROVED by Ada Lovelace" set in Helvetica. `doc.attachFileAt(page, at, name, bytes, mime)` gives the edit that attaches a file under an icon; the document's attachments list it and `attachmentBytes` gives its bytes back. Lamina PDF draws every stamp and icon itself, so pdf.js and PDFium show the same picture.
  
  `pageAnnotations` now reports each stamp's name, each attachment's file and each sound comment's format and length as `details`, and `doc.soundAsWav(id)` returns a sound comment's recording as a WAV file that any browser plays. An image stamp's bytes that aren't a JPEG or PNG fail with the new code `stamp-image-invalid`, and a sound that can't be played with `sound-unavailable`.
- d410bcb: Read a text box back and change what it holds. `doc.textBox(id)` returns any app's text box in the same form you write one in: its text with bold and italic, its font and styling, and where it sits on the page. Change it with a `freeText` patch to `updateAnnot`. When another app's box uses styles Lamina PDF doesn't keep, such as a coloured word, `simplified` is true, and changing the box writes it again without them.
  
  In `@laminapdf/viewer`, `client.textBox(id)` reads a box in the worker. A document that opened before the extract module loaded can't take text boxes: adding one fails with `appearance-needs-text`. Call `client.editInTextCopy()` first, or `client.editInTextCopy({ text: true })` for a font given by its bytes or fallback fonts. Undo and redo still reach every edit made before the call. To check first, `doc.hasText` says whether a document can lay out text in a font given by its bytes.

### Patch Changes

- dbc8e1f: A form whose field list (`/Fields`) names a field's widgets instead of the field now lists that field, where it used to leave it out. `doc.formFields()` (`form` in Rust) follows each such widget to its field, as pdf.js and qpdf do, and lists that field once with all its listed widgets.
- 53316d5: On a file that claims PDF/A, `doc.plan()` now warns `pdfa-color-space` when an image or page you added paints in a colour space the output intent doesn't allow. A CMYK photo in a file with an RGB intent is one example. The warning names the pages (`pages`) and the image stamps (`annotations`). Greyscale images, and images or pages whose colours carry an ICC profile, fit any intent. Like the other PDF/A warnings, it never blocks a save.
- 7b4b37b: pdf.js now shows bold and italic in text boxes whose text goes beyond ASCII, such as Arabic or accented words. Lamina PDF writes each such character in the box's rich text (`/RC`) as an XML character reference, such as `&#x645;`. It used to write UTF-16, which the standard allows but pdf.js can't read.
- Updated dependencies [1e05504]
- Updated dependencies [d44da10]
- Updated dependencies [fd817df]
- Updated dependencies [d17a018]
- Updated dependencies [04aa17a]
- Updated dependencies [175fd59]
- Updated dependencies [dbc8e1f]
- Updated dependencies [53316d5]
- Updated dependencies [7c344c5]
- Updated dependencies [c465c12]
- Updated dependencies [7b4b37b]
- Updated dependencies [032996a]
- Updated dependencies [d410bcb]
  - @laminapdf/wasm@0.7.0

## 0.6.0

### Minor Changes

- 1dfa7b6: Attached files: `attachments()` lists every file a document carries with what holds it (its list of attachments, the document, a page or an annotation it is associated with, a paperclip annotation), `attachmentBytes(id)` gives a file's bytes with its checksum checked, and `collection()` reads a portfolio's schema and files. `attachFile`, `updateAttachment` and `deleteAttachment` edit them, with `attachFileOp` for a transaction. Attaching to a file that claims PDF/A-1 or PDF/A-2 warns `pdfa-attachment-forbidden` in `plan()`, and on PDF/A-3 `pdfa-attachment-incomplete` until the file has an association, a modification date and a MIME type. A file over the document's `maxStreamDecoded` limit, which `attachmentLimit()` gives, is refused with `attachment-too-large`. A file with usage rights holds each attachment edit to its `/EF` right; attaching to a page needs a right usage rights never grant, and to an annotation also `/Annots /Modify`. `addBlob` now works in every module.
  
  The builder's attachments now record an MD5 `/CheckSum` too.
  
  In Rust: `AttachFile`, `UpdateAttachment`, `DeleteAttachment`, `AttachTarget`, `Document::attachments`, `attachment_bytes`, `collection`, `attachment_limit` and `attach_file`; `AfRelationship` is now `laminapdf_model`'s, with three more relationships.
  
  The viewer's worker gains `attachments`, `attachmentBytes`, `attachmentLimit`, `collection` and `addBlob`, and replays attached bytes after a restart.
- 6b13b29: You can now make a summary of a document's comments. `doc.commentSummary({ font })` returns a new PDF that lists every comment with its number, author, date, status, the text it marks, its note and its replies. By default each page is drawn small beside its comments, with their numbers marked on it; the drawing is a copy of the page, so its text stays text. `layout: "list"` leaves the pages out. The summary is tagged and conforms to PDF/UA-1, and the reviewed document doesn't change. A page set in a font the file doesn't embed is listed without its drawing, with the `summary-page-not-drawn` warning. In Rust, `Document::comment_summary` takes `SummaryOptions`.
- 698c058: You can now change the text a page already shows. `doc.content.textRuns(index)` (`text_runs` in Rust) lists a page's runs of text in the order the page draws them, each with its text, box, font, size and line, and a reason when it can't be edited. The new `editText` op (`doc.content.editText(index, run, text)`) gives a run new text in the font it is set in, and moves the text after it in the same text object to fit; empty text deletes the run. A character the font can't draw fails with `missing-glyph`. Some runs can't be edited yet, such as text in a Type 3 font or text written right to left: editing one fails with `content-not-editable` and the reason.
- fa10bae: You can now flatten comments into the page. `doc.flattenAnnotations(scope, intent)` draws each annotation's appearance into its page where it was, then removes the annotation with its pop-up and replies, as one undo step. The scope is `"all"`, `{ pages }` or `{ annotations }`, and the intent `"view"` or `"print"` decides which marks count as shown. The new `flattenAnnots` op does the same for annotations you name, in every module. A hidden mark stays as it is, and so does one that has no appearance and is of a kind Lamina PDF can't draw, such as a file attachment, which the edit's warnings list as `flatten-no-appearance`. Naming a form field's widget fails with `flatten-widget`, and a file that claims PDF/UA-1 gets `flatten-untagged` when you save, because the comments leave the structure tree.
- d50bcec: A comment summary can now be written in your readers' language. Pass `words` to `doc.commentSummary`, such as `page: "Seite {label}"`. It covers the summary's title, comment count, page headings, the alternative text of the drawn pages, the status line, quotation marks, and the names of annotation kinds and review states. Any word you leave out stays in English. In Rust, `SummaryOptions::with_words` takes a `SummaryWords`.
- 1c228eb: Editing a tagged PDF now keeps its tags up to date. When `doc.content.edit` deletes an object or `doc.content.editText` deletes a run of text, the content leaves the structure tree, and an element left empty leaves it too; rewritten text updates its element's `ActualText` and `Alt` when they hold the old words. A replaced figure image loses the old alternative text. Before you save, `doc.plan().conformanceWarnings` lists the claims content edits broke: `figure-without-alt` and `content-tags-broken` on a PDF/UA file, and `pdfa-font-not-embedded`, which gains a `pages` field, on a PDF/A file (once edits can add fonts). The new `doc.removeConformanceClaims({ pdfa, pdfua })` (op `removeConformanceClaims`) takes the PDF/A or PDF/UA claim out of the XMP metadata. It is the only edit that may.
- c8aa73e: A file with usage rights (UR3) now holds each annotation edit to the right it needs: adding an annotation needs `/Create`, changing one `/Modify`, and deleting one `/Delete`. `wouldViolate` and the save planner used to allow every annotation edit when the file granted any one of them, so a file that granted only `/Create` let a delete through without saying it would invalidate the usage rights.
  
  In Rust: `AnnotRight`, `Op::annot_right` and `PendingChanges::annot_rights`.

### Patch Changes

- cb85284: The annotation layer can now resize annotations. Pass `resize` (`{ can, ops }`) to `new AnnotationLayer(…)`, and one selected annotation that `can` accepts gets live handles. Each handle's hit area is 24 CSS px across. Dragging one resizes the annotation from that handle, and Shift keeps its proportions. Alt+arrows resize from the handle at the bottom right of the screen, by 1 pt or by 10 pt with Shift. The layer stretches the annotation with its box, and your `ops` turns the stretched copy into the edit, at once or through a promise. `layer.letPointerThrough(true)` lets the pointer through the annotations to a tool that draws over them. `AnnotationWords` gains `resize`, the history label of a resize.
  
  Each annotation's button is now at least 24 CSS px a side, so a small mark such as a caret still has a target the pointer can reach.
  
  A square's or circle's record now carries the box it runs along: `geometry` is `{ kind: "inner", rect }` (`AnnotationGeometry::Inner` in Rust), inside its rectangle by the stroke's margin (`/RD`). An editor can resize the shape from that box, so the margin doesn't grow with each resize. `pageToUser(frame, point)` maps a page-space point to user space as the Rust model's `PageFrame` does.
- 9846e46: Type on a page: `doc.textBoxAt` and `doc.calloutAt` give the edit that adds a text box, or a callout with a leader line, from a box and points in page space. The text can be plain or rich, with bold and italic words, and Lamina PDF draws the box into the file, upright as the page is shown. Helvetica, Times and Courier work in the extract and text modules. A font given by its bytes (`addBlob`) and fallback fonts for characters it lacks, such as Chinese or Arabic, need the text module and are embedded as subsets. `updateAnnot` changes a box's text, font or look, and warns with `rich-text-simplified` when a box made by another app loses styles Lamina PDF doesn't keep, such as a colour for one word. In Rust, the same edits are `Document::text_box_at` and `callout_at`.
- Updated dependencies [cb85284]
- Updated dependencies [81705ed]
- Updated dependencies [6b13b29]
- Updated dependencies [698c058]
- Updated dependencies [fa10bae]
- Updated dependencies [3b02d1b]
- Updated dependencies [4543a46]
- Updated dependencies [58a25ea]
- Updated dependencies [d50bcec]
- Updated dependencies [1c228eb]
- Updated dependencies [9846e46]
  - @laminapdf/wasm@0.6.0

## 0.5.0

### Minor Changes

- e31c820: A document now reads back its page boxes and page labels in the form the edits take, so an app can show the current values and change them. `doc.pageBoxes(index)` returns a page's media, crop, bleed, trim and art boxes in user space, as `setPageBoxes` takes them, and says which of the last three the page sets itself. `doc.labelRanges()` returns the page labels as ranges, in the form `setPageLabels` takes, so they can be changed and written back. Both need the extract or text module; in Rust they are `Document::page_boxes` and `Document::label_ranges`.
  
  The viewer's document worker serves both reads, as `client.pageBoxes(index)` and `client.labelRanges()`. An edit that sets the page labels, and its undo and redo, now gives every page its new label in `EditResult.relabelled`, and `client.document.pages` takes them. No page is drawn again, since labels draw nothing.

### Patch Changes

- 80c83fa: In Rust, `Document::adopt_copy_from` makes a saved copy the document's file when the copy is read through a byte source rather than held in memory. An app that appends an incremental save to a large file in place can then adopt the file without reading all of it.
- a593fdd: Read a review's comments as threads, reply to them and set their status.
  
  `doc.commentThreads(pages)` gives each comment with its replies in order, the annotations grouped with it, the review states set on it and the latest of each, and the text a highlight marks. A reply to a missing comment, or a ring of replies, comes back as a thread of its own marked `broken`. The `addReply` and `setReviewState` ops answer a comment or record its status as annotations of their own, as Acrobat does, so the thread keeps its history. `doc.deleteAnnotations(ids)` deletes a selection in one undo step. The Rust crates have the same: `Document::comment_threads`, `delete_annotations`, `AddReply` and `SetReviewState`.
- bd4b1e9: Add a clickable contents page made from a document's bookmarks.
  
  `doc.createContentsPage(options)` lays out a page that lists the bookmarks going to a page, one or two levels deep. Each entry shows the label its page will have once the contents page is in, and links to that page. It inserts the page where `at` says, can add a "Contents" bookmark for it, and does all of this as one undo step. In a tagged file, the page joins the structure tree as a `TOC` element. It needs the text module loaded, though the document can be open in any module, and a font you pass, since Helvetica isn't embedded and a PDF/A or PDF/UA-1 file refuses it.
  
  The Rust crates have the same as `Document::create_contents_page(&ContentsOptions, &ContentsFonts)`, and in three parts for a host whose document is open in a build without fonts: `Document::contents_listing` reads what the page lists, `ContentsListing::lay_out` lays it out with the `text` feature, and `Document::insert_contents_page` inserts it, refusing a listing the document no longer gives with `document-changed`. The builder's `Contents::listing` sets a contents block from given entries and labels.
- 9767f69: Draw on a page: `doc.inkAt`, `doc.shapeAt`, `doc.lineAt` and `doc.polyAt` give the edit that draws on a page from points in page space. They draw freehand ink, squares and circles, lines and arrows, and polygons and polylines. Lamina PDF draws each one into the file, with solid, dashed or cloudy borders, a fill colour, every line ending the PDF standard defines, and leader lines. Ink is smoothed through its points, and a stroke can carry one width per point for a pressure-sensitive pen. `updateAnnot` erases ink strokes and adds new ones. In Rust, the same edits are `Document::ink_at`, `shape_at`, `line_at` and `poly_at`.
- 2a447ca: Pages inserted before the first page of a labelled document no longer repeat its label. They join the start of the first label range: before a range starting at "i" or "1", the first new page takes that label and the range's own pages are numbered up by one, while later ranges, such as an appendix's "A-1", keep theirs. Before a range that starts higher, say at 5, new pages take the numbers below it and nothing else moves. This applies to inserted blank pages, pages inserted from another file and merges.
- Updated dependencies [a593fdd]
- Updated dependencies [91c927d]
- Updated dependencies [bd4b1e9]
- Updated dependencies [9767f69]
  - @laminapdf/wasm@0.5.0

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
- Updated dependencies [690f036]
- Updated dependencies [4037b81]
- Updated dependencies [64bcf88]
- Updated dependencies [1d12ce8]
- Updated dependencies [2a11350]
- Updated dependencies [e183d4c]
- Updated dependencies [6d048db]
  - @laminapdf/wasm@0.4.0

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

### Patch Changes

- a815a29: Documents can now list a page's annotations and change or delete them. `doc.pageAnnotations(index)` returns each annotation in page space: its ID, subtype, rectangle, text, author, dates, flags, colour, reply and group links, and its own geometry (highlight areas, ink strokes, a polygon's vertices, a line, a callout). The new `updateAnnot` op changes an annotation's text, author, subject, flags or position, keeping its appearance; `deleteAnnot` deletes it together with its pop-up, its replies and the rest of its group, and the edit lists what went with it in `alsoDeleted`. A locked annotation refuses the change.
- Updated dependencies [bc1bff5]
- Updated dependencies [2948e3f]
- Updated dependencies [6e9dd39]
- Updated dependencies [d9b6acf]
- Updated dependencies [e416ac6]
- Updated dependencies [2081146]
- Updated dependencies [8f21b98]
- Updated dependencies [bab4c9f]
  - @laminapdf/wasm@0.3.0

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

- 7a59051: Every export of `@laminapdf/sdk` now says which release added it: a `@since 0.2.0` tag, which the API reference shows as "Since" and editors show on hover. In Rust, every public item's docs end with "Added in 0.2.0." Items added later will name the release they first ship in. The types `ExtractFormat`, `ReadingOrder`, `SplitOn`, `TableMode`, `LayoutRegion`, `ModelInfo`, `ModelRequest`, `Models`, `OcrWord` and `RegionLabel` are now declared in `@laminapdf/sdk` with their own docs; they're the same types as before.
- e5fa57d: `builder.markdown` and `builder.flow` now lay a tab in a code block out as spaces to the next multiple of four columns, so the extracted text lines up as drawn. A tab anywhere else in the text becomes one space. Before, a font with CFF outlines (an `.otf` font) raised a `tounicode-conflict` warning for each code block line with a tab.
- 2e74d7d: Every error's and warning's `docsUrl` now opens its page on the docs site, `https://docs.laminapdf.com/reference/codes/<code>/`, instead of the Markdown file on GitHub. The code still ends the path.
- 8ae1bae: `FlowTemplate` is now an interface rather than a type alias, with the same keys and the same checks, so the API reference names it where `builder.flow()` takes it instead of spelling out every key of every block. Its reference page explains the placeholders, with an example. Every type declaration in the TypeScript reference is now a code block, one key or union case to a line.
- cafed9f: `0.2.0` is the first release that isn't an alpha. A plain `npm install @laminapdf/sdk` or `cargo add laminapdf` now installs it, and from here on releases follow SemVer as the [stability policy](https://docs.laminapdf.com/reference/stability/) says: a breaking change raises the minor version (`0.2` to `0.3`), so an upgrade within `0.2.x` needs no change to your code. The `alpha` npm tag stays on `0.2.0-alpha.6`.
- bd0148e: The packages' descriptions, keywords and README openings now say what each package does for you, and each package links to the issue tracker.
- 55fc5eb: In Rust, the save planner now checks page edits field by field. The save is now a conflict when a page edit deletes a signed signature's widget or changes how it is shown (`Restriction::SignedWidget`), or does the same to a field that a signature's lock covers (`Restriction::FieldMdp`); `Violation::field` names the field. Moving a page breaks neither. `laminapdf_ops::Applied::widgets` and `PendingChanges::widgets()` record the fields an edit touched, and `Op::page_changes()` lists the pages it will change, so `SavePlanner::would_violate` can warn before the edit. The page edits that report them come next.
- Updated dependencies [e5fa57d]
- Updated dependencies [2e74d7d]
- Updated dependencies [abd2520]
- Updated dependencies [8ae1bae]
- Updated dependencies [97cbf89]
- Updated dependencies [cafed9f]
- Updated dependencies [755d6ec]
- Updated dependencies [bd0148e]
- Updated dependencies [82d3c07]
- Updated dependencies [9f555a2]
  - @laminapdf/wasm@0.2.0

## 0.2.0-alpha.6

### Minor Changes

- 810d0d4: Rust callers can now describe flowing pages as JSON (TypeScript support follows), in the format `laminapdf.document/1`, and lay them out with `Builder::document`. A document names its page, its text styles and its blocks (paragraphs, headings, lists, tables, figures, a contents page and more), and its fonts and images by name, with their bytes given beside it. Its JSON Schema ships in `docs/schemas/document.schema.json`.
  
  A template is a document with placeholders that `flow::document::fill` replaces from data: `$data` for a value, `$each` to repeat a block or table row per item, `$item` for a value of the current item, and `$if` to keep a block only when a value is there. Templates have no expressions, so the data arrives formatted.
  
  A document with an unknown key, a missing one or a value of the wrong kind fails with `invalid-document`, which names the value's place in `data.pointer` and suggests the closest key in `didYouMean`. A template whose data lacks a value fails with `template-data-missing`, and data of the wrong kind with `template-data-type`.
- 64f7860: Turn Markdown into a PDF from TypeScript: `builder.markdown(text, options)` turns CommonMark (with tables, footnotes, strikethrough and task lists) into tagged pages that flow, and takes the title and language from the front matter. Fonts come by role (`regular`, `bold`, `italic`, `boldItalic`, `mono`, `fallbacks`) and images by the name the Markdown uses; nothing is fetched. `options.page` sets the page size, margins, header and footer, and `contents` adds a contents page. `Font.fromCollection(bytes, index)` loads one face of a font collection (`.ttc`).
- 70eb584: Undo and redo say which pages to draw again. `doc.undo()` and `doc.redo()` now return `{ outcome, touchedPages }` instead of the outcome alone, so a viewer can redraw only the pages the undone or redone edit changed. `doc.apply(ops)` also says whether the edit `changed` anything: an edit that only sets values the document already had leaves nothing to undo. `doc.isEncrypted` tells you whether the file is encrypted. The Rust `Document::undo` and `redo` return the same `Reverted` record, and `Edited` has `changed`.

### Patch Changes

- Updated dependencies [810d0d4]
- Updated dependencies [64f7860]
- Updated dependencies [70eb584]
  - @laminapdf/wasm@0.2.0-alpha.6

## 0.2.0-alpha.5

### Minor Changes

- 526aae3: Reader mode for every PDF. When a file has no tags, or its tags fail reader mode's checks, `doc.reflow()` reads the page layout instead of refusing: `source` is `"approximate"` and `reason` says why the tags weren't used. A scanned page, or one whose column order extraction doubts, comes back as a `page-image` block with `why` (`"scanned"` or `"uncertain"`) and the page's text for searching. Tables and pictures are crops, and their captions are kept with them. Only a file with nothing to show has `available: false`, with the reason `"nothing"`.
- 5fc1f96: `doc.properties()` reads what a properties view shows about a document in one call. It returns the Info entries, with the creation and modification dates as ISO 8601, and the XMP packet as text. It also gives the PDF version, page count, file size, whether the file is linearized and tagged, and the fonts the pages use (name, type, embedded, subset, encoding). Encryption comes with the author's permissions, and signatures are counted, never verified. `contains` says whether the file holds JavaScript, attachments or a form. It needs the extract or text module. For Rust callers, `Document::properties()` returns the same as `DocumentProperties`, and `laminapdf-model` gains `Pages::fonts()` and `Document::xmp_packet()`.
  
  The viewer's client gains `properties()`, `printPage(index, { dpi, annotations })` and `fileBytes()`. `printPage` draws a page in the document worker for printing, with annotations as their print flags say, and returns it as a PNG. `fileBytes` returns a linked file's bytes as opened.
- 95d2a70: Reader mode for tagged PDFs. `doc.reflow()` returns the document's blocks in its structure tree's order: headings, paragraphs, list items, quotes, captions and notes as text, and each figure, formula and table as a crop of its page with its alternate text, never as reflowed text. It checks the tree first: a file whose tree it can't trust comes back with `available: false` and the reason. It needs the extract module. The viewer's worker serves it as `client.reflow()`.
- 513aeb3: Links in PDFs now work in the viewer. `doc.pageLinks(index)` lists a page's links in page space, each with what following it does: a place in the document, a web address, another file, a named action, or an action such as JavaScript, which Lamina PDF never runs. The viewer draws an anchor over each link. Links inside the document go there, web links ask the app to confirm first, and anything else does nothing. `viewer.history` records each jump, so Back returns to where the reader was, and the web reader keeps those jumps in the browser's history.

### Patch Changes

- Updated dependencies [526aae3]
- Updated dependencies [771fafd]
- Updated dependencies [5fc1f96]
- Updated dependencies [95d2a70]
- Updated dependencies [513aeb3]
  - @laminapdf/wasm@0.2.0-alpha.5

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
- 703c2d8: Read a document's outline and named destinations, and move through it with thumbnails. `Document.outline()` gives the outline's items with their titles, whether each starts expanded, and where each goes: a page and a view in page space (points from the displayed page's top-left, y down), whichever coordinates the file used. `Document.namedDestination(name)` finds one named destination. Both need the extract or text module, like `page.text()`.
  
  In `@laminapdf/viewer`, `ViewerClient.outline()` and `namedDestination(name)` read them in the document Web Worker, and `Viewer.goToDestination(target)` shows an item's or a destination's target, with each kind of fit. `ThumbnailStrip` is a list of the pages as small pictures. It mounts only the thumbnails near its view, renders after the main view's pages and goes to a page on a click or a key.
- 60495bf: `doc.structureTree()` reads a tagged document's structure tree: its elements in reading order, down to the marked content each owns on a page. Each element gives its standard type (`H1`, `P`, `LI`, `Figure` …), alternate text, language and actual text; custom types resolve through the role map. A text run's `mcid` links it to its element, so a viewer can give assistive technology a document's headings, paragraphs, lists and tables.
  
  `doc.isTagged` says whether the document claims to be tagged. Both need the extract or text module, as `page.text()` does. For Rust callers, `Document::structure_tree()` returns `StructureNode`s.
- 2886263: Rust callers can now lay out a paragraph whose words change font, size or language: a bold word inside a sentence, a superscript, an Arabic name in an English line. `laminapdf_text::break_spans` takes the text and a list of `StyledSpan`s, each covering a stretch of it with its own fonts, size, baseline rise, language and OpenType features. Lines still break only where the text allows, so a style change inside a word never splits it. Spans of one style are shaped together, so kerning carries across them. Each line reports its ascent and descent from its tallest run, and `pdf_line_run` draws a superscript's rise. `break_lines` is unchanged. Spans that don't cover the text once, in order, that split a letter from its accent, or that name no font or a font past the list fail with the new error code `invalid-spans`.

### Patch Changes

- Updated dependencies [00bf63b]
- Updated dependencies [915c253]
- Updated dependencies [4fdbfd4]
- Updated dependencies [f69228f]
- Updated dependencies [2254184]
- Updated dependencies [c74d78f]
- Updated dependencies [cc7f9ce]
- Updated dependencies [703c2d8]
- Updated dependencies [60495bf]
- Updated dependencies [2886263]
  - @laminapdf/wasm@0.2.0-alpha.4

## 0.2.0-alpha.3

### Minor Changes

- 471ac52: The package now carries its docs as Markdown in `docs/`, matched to its version: every page of the docs site, the API reference, a page per error code, `renames.json` (what renamed items are called now) and `AGENTS.md`, a snippet for your project's agent instructions. The `laminapdf` crate ships the same folder without the API reference. The docs site serves a Markdown copy of every page, `llms.txt` and `llms-full.txt`.
- 860bf03: Every handle on WebAssembly memory frees with `free()` or `using`: `Document`, `PageRef`, `SaveJob`, `Builder`, `Page` and `Image`. A second `free()` does nothing, and any other call afterwards throws `disposed` instead of a glue error. A page or save job is freed with its document, and freeing a document now frees its memory at once, even while pages of it are still held. Pages of a freed builder throw `disposed` too (`builder-finished` once it finishes), and a freed `Image` throws `disposed` instead of decoding again.
  
  Each module's memory may grow to 2 GiB. Past that, the module traps and the call throws `out-of-memory`; any other trap throws `wasm-trap`. The next call runs in a fresh instance, and handles of the old one throw `instance-reset`. The new `reset()` gives grown memory back by starting every module afresh, and the new `onLeak(hook)` reports handles the garbage collector reclaims without a `free()`.
- 110d8b9: `doc.extract({ format })` gives a document's text for an LLM or a search index: Markdown or plain text in reading order, the `laminapdf.extract/1` JSON with a box for every block (the same bytes as Rust's `extract_json`), or chunks sized for retrieval. Tables drawn with lines come out as tables, and hidden text is left out. It needs the extract or text module.
- bdd0af4: Two new warning codes, `extract-table-ambiguous` and `extract-table-limit`, join the generated `WarningCode` union. In Rust, `laminapdf-extract` is now published, and the `laminapdf` crate exposes it. `Document::extract`, `extract_job` and `extract_json` return blocks in reading order, with tables and their cells. An `Extraction` renders as Markdown, plain text, retrieval chunks or `laminapdf.extract/1` JSON, whose JSON Schema is in `laminapdf-extract`'s `schemas/`.
- 48dcd44: Breaking (Rust, while alpha): `laminapdf::Error` no longer exposes the errors of the crates below it. The `Cos`, `Ops`, `Save`, `Text` and `Build` variants are one opaque `Error::Core`, and `CosError`, `OpsError`, `SaveError` and `SinkError` are no longer re-exported (`BuildError` and `TextError` stay: the builder's and `Font`'s own methods return them). Match on `error_code()` instead; a `data-pending` error's ranges come from the new `Error::pending_ranges()`, which replaces `laminapdf_source::pending_ranges`. The JavaScript API and every error's code, message and JSON are unchanged.
- ee80542: Breaking (Rust, while alpha): every public type now has a name of its own, and each item one public path.
  
  - An extracted run is `PageTextRun` (`laminapdf::extract::PageTextRun`, `laminapdf_text::PageTextRun`), the name `@laminapdf/sdk` already gives it. `TextRun` now means only drawn text.
  - `laminapdf-build`'s font handle is `FontHandle`, the name the façade already gave it, so the builder's signatures read `&FontHandle` beside `laminapdf::Font`.
  - `laminapdf-text`'s OpenType tag is `OpenTypeTag`, and the façade exports it for `Font::instance(&[(OpenTypeTag::WEIGHT, 700.0)])`.
  - `laminapdf-text`'s `extract` and `agl` modules are private. Their items are at the crate root (`laminapdf_text::TextExtractor`, `PageText` and the rest), and `agl::to_unicode` is now `glyph_name_to_unicode`, beside `zapf_dingbats_to_unicode`. `CMap` and `MAX_USECMAP_DEPTH` are now private to the crate.
  - `laminapdf::Document::ops` and `into_ops` are hidden from the docs. They stay for Lamina PDF's own crates, and may change without notice.
  
  In JavaScript, the extracted run's class is itself named `PageTextRun` now, so `run.constructor.name` reads `PageTextRun`. The rest of the JavaScript API and every output are unchanged.
- 9b579a4: New: the `laminapdf` command-line tool (`crates/laminapdf-cli`, not published to crates.io yet), with `info`, `check`, `pages rotate`, `extract` and `explain`, versioned JSON reports with JSON Schemas, and exit codes 0–5 (ADR 0031). Its own error codes (`invalid-command-line`, `invalid-page-range`, `binary-output-to-terminal`, `unknown-code`, `overwrite-refused`, `input-unreadable`, `output-unwritable`) join every code table, so the TypeScript `ErrorCode` union lists them too. The Rust façade gains `Document::version`, `is_encrypted`, `is_tagged` and `read_every_object`, and `laminapdf::did_you_mean`.
- f4a2d8e: Two new warning codes, `extract-limit-blocks` and `extract-reading-order-ambiguous`, join the generated `WarningCode` union. In Rust, `Budget`, `Step` and `Phase` now come from `laminapdf-syntax` (the façade re-exports them at the same paths), `laminapdf_model::Document::is_tagged` and `RectPt::union` are new, and `laminapdf_text::is_right_to_left` is public. A new crate, `laminapdf-extract`, extracts blocks in reading order (ADR 0030); it is published with L3b.
- 7b2c22f: New: `laminapdf mcp`, an MCP server on standard input and output that serves the CLI's commands as tools, in builds with the `laminapdf-cli` crate's `mcp` feature (ADR 0031 §9). It touches only files under its `--root` folders, is read-only unless started with `--allow-write`, and keeps document text out of everything but report fields. A path outside the roots is the new error code `path-outside-roots`, so the TypeScript `ErrorCode` union lists it too. The Rust façade exposes the error schema as `laminapdf::ERROR_SCHEMA`.
- 3536f81: Breaking (Rust, while alpha): `Matrix`, `FillRule` and `Dash` are now defined once, in `laminapdf-model`. `laminapdf`, `laminapdf-build` and `laminapdf-content` re-export them under their old paths. Their API changes as follows:
  
  - `Matrix::translate` takes two `f64`s instead of two `Pt`s.
  - `Dash` lengths are `f64`s in the drawing's own units, so the builder takes points: `Dash::new(vec![18.0, 6.0], 0.0)`.
  - The content reader's `Matrix::new` takes an array, `multiply` is `then` (the receiver applies first, with `after` for the other order), and `transform_point` is `apply`.
  - `PageFrame::to_user_space_matrix` and `TilePlan::page_to_tile` return a `Matrix` instead of a `[f64; 6]`. Read its fields, or call `to_array()` for the six numbers. `laminapdf-render` re-exports `Matrix`.
  - `Dash` is `#[non_exhaustive]`: build one with `Dash::new`. The content reader's `Matrix::from_object` is removed.
  - The content reader's current colour, `laminapdf_content::Color`, is now `CurrentColor`, so `Color` means only the builder's colour.
  
  The JavaScript API and every output are unchanged.

### Patch Changes

- 0a68798: `didYouMean` now suggests the same names as errors from the WASM module. Names equally close to a mistyped one come in name order, and a typo with characters outside ASCII counts one edit per character.
- 658d8b8: Text extraction now avoids a quadratic scan for right-to-left runs whose glyphs have zero advance, preventing long stalls on PDFs with missing font widths. The extracted text stays the same.
- 2440a9a: The `@laminapdf/sdk/worker` worker now frees the document of an `open()` or `openUrl()` you abort just as it finishes. Before, the call rejected with `cancelled`, but the document stayed in the worker's memory until the worker stopped. An `open()` aborted while it ran always leaked its document, since the worker reads the cancel only after the synchronous open has answered.
- Updated dependencies [110d8b9]
  - @laminapdf/wasm@0.2.0-alpha.3

## 0.2.0-alpha.2

### Minor Changes

- 2107764: `Document::save` no longer takes a `CancelToken`. To stop a save from another thread, call `save_with(&options, &cancel)` instead. In TypeScript, `save()`, `saveWithReport()` and `SaveJob.step()` now return `Uint8Array<ArrayBuffer>`, which `Response` and `Blob` accept without a cast. Smaller additions: `RectPt::from_points(x, y, width, height)`, `SetInfo::patch(patch)` for an edit that records no `/ModDate`, and `new()` on `BuildOptions`, `OpenOptions`, `PngOptions`, `RetryOptions` and `TextBoxOptions`. You can now ignore the run that `Surface::text` returns: the text is drawn either way.

### Patch Changes

- 32a4963: The WASM module's `beginTag` and `beginArtifact` now take a structure type, table scope and artifact kind by name instead of by index, which breaks code that calls the module directly; `@laminapdf/sdk` already calls them that way. `@laminapdf/sdk` also exports the `LineCap`, `LineJoin` and `TableScope` types that `StrokeStyle` and `TagSpec` use, and `laminapdf-build`'s `FillRule`, `LineCap`, `LineJoin`, `BlendMode`, `ArtifactKind` and `TableScope` gain an `ALL` list of their variants.
- a59a32a: Text copied from a standard 14 font no longer loses half of its last character when that text runs past 256 UTF-16 units and ends with a character outside the BMP, such as an emoji. Symbol and ZapfDingbats now write a `/CapHeight`, as every other font does, and `laminapdf-syntax` adds `Object::number` and `Object::rounded`.
- Updated dependencies [32a4963]
  - @laminapdf/wasm@0.2.0-alpha.2

## 0.2.0-alpha.1

### Minor Changes

- 5c2624e: The library is called Lamina PDF, and its public names follow. `@laminapdf/sdk`'s error class is `LaminaPdfError`, checked with `isLaminaPdfError`, and its warnings are `LaminaPdfWarning`. The error object's JSON Schema is `laminapdf.error/1`. `@laminapdf/wasm`'s modules are `lamina_pdf_base`, `lamina_pdf_text`, `lamina_pdf_extract` and `lamina_pdf_render_hayro` (`.js`, `.d.ts` and `.wasm`).
- 08e123d: `@laminapdf/viewer` exports only its documented API: `ViewerClient`, `createViewer` and `Viewer`, the types of their options, events and state, the protocol types `client.call` takes, and `MIN_ZOOM`/`MAX_ZOOM`. Its internal helpers (layout, canvas pool, bitmap cache, tiles, zoom maths), its byte-source functions and its protocol constants are no longer exported. Its worker no longer keeps a log of every render, which grew without bound, and no longer serves test-only methods. `@laminapdf/sdk/worker/server`'s `serve()` takes a `saveBudget` (work units per save step), and `workerStats()` reports the handles and calls a worker holds. `@laminapdf/sdk/worker` exports the `Emitter` its client uses.

### Patch Changes

- 1c6e773: A render `data-pending` thrown inside `@laminapdf/sdk`'s worker now reaches the page with its byte ranges: `LaminaPdfError.ranges` lists them. Before, they arrived empty. `RenderError` has a new `toDiagnostic()`, which returns the plain error object every Lamina PDF surface uses, and the worker posts that object for any error that has the method. `RenderError`'s `toJSON()`, `reviveRenderError` and `SerializedRenderError` are gone, and `ByteRange` and `RenderErrorKind` name its types. The viewer's worker now throws `RenderError`s for `cancelled` and `disposed` instead of plain objects, so they reach the page with a `docsUrl`.
- 2a9e364: The `rotatePage` op now reads a `/Rotate` stored as an integral real, such as `90.0`, as the rotation the page displays; before, it read 0. `PageBuilder.rotate` now refuses angles other than 0, 90, 180 and 270, as its type says. In the Rust crates, `Rotation` from `laminapdf-syntax` replaces `laminapdf-render`'s `ViewRotation` and the degrees in `RenderPage::rotate`, and `laminapdf_cos::Page` drops `media_box`, `crop_box` and `rotate`: read them, and their strict-mode errors, from `laminapdf-model`'s page view.
- Updated dependencies [5c2624e]
  - @laminapdf/wasm@0.2.0-alpha.1

## 0.2.0-alpha.0

### Minor Changes

- 2b4384a: Accessible and archival PDFs. A `profiles` option on the builder (`PDF/UA-1`, `PDF/A-2b`, `PDF/A-2u`, `PDF/A-3b`; `BuildOptions::with_profile` in Rust) checks each machine-checkable rule the document breaks when it finishes, with a coded error per rule: `missing-title`, `missing-lang`, `untagged-content`, `missing-alt-text`, `heading-level-skipped`, `unnumbered-heading-misused`, `invalid-structure-nesting`, `link-without-structure`, `link-without-contents`, `font-not-embedded`, `glyph-without-unicode`, `device-color-without-output-intent`, `embedded-file-forbidden`, `embedded-file-without-mime-type` and `profile-version-conflict`. The builder writes the XMP identification, `/DisplayDocTitle` for PDF/UA-1 and, for PDF/A, an sRGB output intent with a 480-byte embedded ICC profile. `attach(name, bytes, mime, { relationship })` adds associated files for PDF/A-3b, such as an e-invoice's XML. Every example that claims a profile is validated by veraPDF each night.
- 592d285: `@laminapdf/sdk/worker` can be extended: `@laminapdf/sdk/worker/server`'s `serve({ methods })` adds methods to the same worker, called with `client.request(method, params, { priority, key })`. The worker runs higher priorities first and coalesces calls with the same key. `emit()` sends events to `client.on("event")`, and `connect({ onDiscard })` sees late answers (by default it closes their ImageBitmaps). The worker protocol is now version 2.

### Patch Changes

- Updated dependencies [2b4384a]
  - @laminapdf/wasm@0.2.0-alpha.0

## 0.1.0-alpha.0

### Minor Changes

- First alpha, `0.1.0-alpha.0`, versioned and tagged but not published (ADR 0017). The `lamina` façade and its core crates at the same version: open, read, edit and save PDFs (incremental and full rewrites, the save planner, undo/redo), generate them (pages, vectors, colour, images, standard 14 and embedded fonts with shaping, bidi and line breaking, links, outlines, metadata, tagged PDF), and extract page text; `@laminapdf/sdk` over the base, text and extract WebAssembly modules in browsers, Node, Deno, Bun and Cloudflare Workers.

### Patch Changes

- Updated dependencies
  - @laminapdf/wasm@0.1.0-alpha.0
