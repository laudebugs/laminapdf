# @laminapdf/sdk

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
