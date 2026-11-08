# @laminapdf/sdk

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
