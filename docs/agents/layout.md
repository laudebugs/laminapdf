# Repository layout in detail

Every crate, package, app and tool, with what each file of note holds. The root `CLAUDE.md` has the one-line-per-folder version; update this file when a PR adds a crate, a package or a file of note.

```
.
├── CLAUDE.md            # what every session needs; this folder (docs/agents/) holds the detail
├── AGENTS.md            # a link to CLAUDE.md, for agents that read AGENTS.md
├── README.md            # project overview
├── Cargo.toml           # Rust workspace (virtual manifest): shared package fields, deps, lints, profiles
├── rust-toolchain.toml  # pinned Rust toolchain + targets (wasm32, iOS, Android)
├── package.json         # private pnpm root: dev tooling only; pnpm-workspace.yaml lists packages/*, apps/*
├── .changeset/          # Changesets (ADR 0017): config.json (fixed group: @laminapdf/wasm, pdf, render, render-hayro, render-pdfium, viewer), pre.json (alpha pre mode)
├── CHANGELOG.md         # generated: the fixed group's changelogs merged by `cargo xtask version sync`
├── biome.json  tsconfig.base.json  deny.toml  clippy.toml  rustfmt.toml
├── context7.json        # Context7 (W3f): how it parses the repository's listing (the docs pages, the examples, the error
│                        #   pages; rules for agents from packages/sdk/examples/AGENTS.md) and that listing's claim (url, public_key)
├── dist-workspace.toml  # cargo-dist for the `laminapdf` binary only (L4d): eight targets, `mcp` on, Homebrew formula
│                        #   (laudebugs/homebrew-tap), archives with licences, notices, man pages and completions
├── crates/              # Rust crates, flat: crates/laminapdf-<name>
│   ├── laminapdf-syntax/ #   core: object model + serialization, lexer, parser, filters (Flate, PNG predictors), Limits,
│   │                    #     the error contract's shared parts: ErrorKind, Location, the generated ErrorCode/WarningCode
│   │                    #     (src/generated/) and, with feature `json`, Diagnostic (the ADR 0026 object every boundary throws)
│   ├── laminapdf-cos/   #   core: reader and overlay: Document over a ByteSource (xref, object streams, repair, page tree,
│   │                    #     warnings), the overlay (CosMut, journal), rebase
│   ├── laminapdf-model/ #   core: typed views (pages, outline, Info, name/number trees, page labels, structure tree,
│                        #     the interactive form: `Document::form` → `Form` of `TerminalField`s with the reconciliation report, `tab_order` by `/Tabs` (forms/, E8c);
│                        #     form behaviour without JavaScript (forms/behaviour/, E8f, ADR 0021): tokens.rs and recognise.rs match a field's /AA K/F/V/C script against the helper-call list, number.rs, date.rs and special.rs do the helpers' work, scripts.rs gives `field_behaviour` → `Behaviour` and `form_scripts` → `FormScript` (handled, not handled, xfa);
│                        #     annotations: `Document::annotations` → `Annot`, E4b; comments as threads: `comment_threads` → `CommentThread`, E5b; an annotation's normal appearance and its §12.5.5 placement: `normal_appearance`, `appearance_placement`, and whether optional content is off, `optional_content_off`, E5e; `Annot::details` (`AnnotDetails`: a stamp's name, an attachment's file, a sound's format) and `Document::sound` → `Sound::to_wav` (raw, signed, μ-law and A-law samples as PCM WAV), sound.rs, E5c),
│                        #     LaminaPdfIds and the shared graphics values (Matrix, FillRule, Dash), ObjectCopier (pages and what
│                        #     they reach copied across documents); xml.rs: the guarded XML reader (no DTD, XML's five
│                        #     entities, bounded depth and declarations; E5d), which xmp/ and XFDF use; xmp/: the XMP
│                        #     packet (a tolerant reader on xml.rs, typed Info-mapped fields, the one Info ↔ XMP
│                        #     table, which the builder's literal tags are tested against, a writer that edits in place;
│                        #     any property set or removed (XmpValue), PDF/A extension schemas declared);
│                        #     writes trees: the page-tree editor (PageTreeEditor), name and number tree writers
│                        #     (TreeWriter), PageLabels::remap/write, and PageReferences (what points at a page); feature
│                        #     `navigation` (which `edit` implies): OutlineTree (E3b: the outline's lists and counts
│                        #     rewritten after inserts, moves and removals by OutlineItemId)
│   ├── laminapdf-ops/   #   core: typed serialisable Op (SetMetadata, RemoveMetadata, the XMP ops, the planner's
│                        #     SetFieldValue (forms/, E8c: a field's value with each widget's /AS, /I and /RV in step; E8f: committed through its keystroke and validate helpers, drawn through its format, then /CO's calculations in the same transaction, forms/calculate.rs, skipping totals `FieldLocks` reports locked), SetFieldHidden (forms/hidden.rs, E8f: a Hide action's flags, class Annotation), 
│                        #     UpdateSaveMetadata, AddAnnot (text markup, notes, carets; E4c; ink, squares and circles, lines, polygons with cloudy borders and the private /LMNA_Widths and /LMNA_Smoothing keys, annot/drawing.rs; E4d; FreeText text boxes, callouts and typewriter text with /DA, /DS, the /RC rich-text subset (RichText) and AnnotFont, a standard 14 name or a font's bytes by BlobId, annot/freetext*.rs, `rich-text-simplified` when a redraw drops styles; E4e; `read_text_box(doc, id)` → `TextBoxRead`, a box read back as its spec, the read `UpdateAnnot` makes, in annot/freetext.rs; E4l; Stamp: a standard name drawn as paths, an image by BlobId (`stamp-image-invalid`) or a dynamic stamp's resolved lines in Helvetica, annot/stamp.rs; FileAttachment: an icon holding a file written by E3f's writer, /AF on PDF/A-3, annot/file_attachment.rs; E5c), AddReply and SetReviewState (E5b, annot/reply.rs; delete_annotations), FlattenAnnots (E5e, annot/flatten.rs: appearances drawn into the page through the append API; flatten_candidates; flatten-widget, flatten-untagged), UpdateAnnot and DeleteAnnot (annot/: the
│                        #     Appearances hook laminapdf-build's AnnotationAppearances implements), AddLink, UpdateLink and
│                        #     DeleteLink (link/, E3c: tagged links as Link elements over the content they cover),
│                        #     RotatePage, the deprecated SetInfo; outline/: the bookmark and named-destination ops), metadata/ (one Info + XMP writer, read_metadata,
│                        #     conformance warnings), transactions, savepoints, undo/redo, source documents and the blob store
│   ├── laminapdf-save/  #   core: save planner (SaveConstraint, signatures, mode table), incremental and full writers
│   │                    #     over the overlay, WriteSink, /ID, rebase; the spike-era Writer (feature `test-support`)
│   ├── laminapdf-content/ #   core: content-stream interpreter (tokenizer, graphics/text state, Form XObjects,
│                        #     marked content) reporting to a Visitor (ADR 0023); write/: the append API (E2e:
│                        #     PageContentWriter, scan_balance); edit/: the editable IR (E7c, ADR 0022: ContentOps,
│                        #     objects with spans, Recorder, hit testing, the span-preserving encoder)
│   ├── laminapdf-text/  #   core: fonts for generation: loading (skrifa), glyph usage, subsetting (subsetter),
│                        #     Type 0 embedding with /W and ToUnicode; shaping (HarfRust), bidi and line
│                        #     breaking (ICU4X), greedy layout over styled spans, shaped runs as TJ arrays; the standard 14 (AFM
│                        #     metrics), simple-font encodings, the AGL; text extraction (fonts, CMaps,
│                        #     ToUnicode → Unicode per glyph)
│   ├── laminapdf-build/ #   core: the builder over the overlay: pages and boxes, paths, colour (device, ICC),
│                        #     opacity/blend ExtGStates, clipping, images (JPEG, PNG + SMask, RGBA), text runs and
│                        #     boxes, links, destinations, outlines, page labels, Info + XMP, tagged PDF, PDF/UA-1 and
│                        #     PDF/A-2b/2u/3b profiles (data/: the CC0 sRGB ICC), attachments; annotation appearances (annot/; stamps' lettering as paths from Lamina Sans Bold outlines, which tools/fonts/stamp_glyphs.py writes into annot/appearance/stamp/generated/glyphs.rs; FreeText's
│                        #     standard 14 tier under feature `freetext`, which the façade's `extract` turns on, embedded and
│                        #     fallback fonts under `text`; E4e); images_to_pdf with EXIF
│                        #     orientation; page space
│   ├── laminapdf/       #   core: the façade (ADR 0028): Document, one Error (ADR 0026 JSON), jobs, CancelToken,
│                        #     Builder; features `build`, `extract`, `text`, `markdown`, `saslprep`, `explain`; src/generated/ and schemas/ from
│                        #     `cargo xtask gen codes`; examples/ with byte goldens and render goldens
│   ├── laminapdf-extract/ # core: extraction for retrieval and LLMs (ADR 0030): runs → lines → blocks, XY-Cut++ reading
│                        #     order, invisible/off-page/tiny text dropped and counted, de-hyphenation, ruled and unruled
│                        #     tables; Markdown, text, chunks and laminapdf.extract/1 JSON (schemas/); ExtractJob;
│                        #     model plug-ins (plugins.rs, regions.rs: LayoutModel, OcrEngine, request/provide; L3c);
│                        #     reader mode (reflow/: reflow → blocks in the tree's order, crops, the tree's checks; U4b;
│                        #     approximate.rs: the page layout's order when the tags can't be used, page images; U4c)
│   ├── laminapdf-interchange/ # core: comments to and from other apps (E5d): XFDF (on laminapdf_model::xml) and FDF
│                        #     (on laminapdf-syntax's parser) read into an Exchange of annotation dictionaries and
│                        #     written back (map.rs: XFDF's attributes ↔ PDF keys, one table); export.rs (a document's
│                        #     comments); import.rs (preview, import: Merge, Replace, Add as one transaction of
│                        #     AddAnnot, AddReply, SetReviewState, UpdateAnnot, DeleteAnnot); markdown.rs; README.md
│                        #     lists how other apps' files differ
│   ├── laminapdf-layout-onnx/ # edge, unpublished: Docling's Heron layout model through ONNX Runtime (ort, load-dynamic)
│                        #     as a LayoutModel; weights checked by SHA-256; tests/heron.rs (ignored) vs a reference run
│   ├── laminapdf-wasm/  #   edge: wasm-bindgen bindings of the façade, built three times (base, `text`, `extract`) into @laminapdf/wasm
│                        #     (ADR 0016); tests/ run in Node with wasm-bindgen-test
│   ├── laminapdf-cli/   #   app: the `laminapdf` binary (ADR 0031): commands as functions (src/commands/: build, info, check,
│                        #     pages rotate, extract, render, explain, reveal) over an `Io`, the clap front end (main.rs, render.rs), page
│                        #     ranges, exit codes and `CliError`, schemas/ (generated from the reports); `cliErrors` in codes.json;
│                        #     src/mcp/ (feature `mcp`): `laminapdf mcp`, the commands as MCP tools (rmcp, stdio, roots)
│   ├── laminapdf-source/ #   edge: SparseSource (filled range by range), the data-pending retry loop (over `laminapdf::Error::pending_ranges`), FileSource
│   │                    #     (feature native), MmapSource (feature mmap) (L2c)
│   ├── laminapdf-render/ #   core: the RenderBackend / RenderDocument traits (ADR 0004): validated tile requests (TilePlan,
│   │                    #     RenderLimits), straight-RGBA tiles, RenderError; the hayro adapter (feature `hayro`);
│   │                    #     export_pages to PNG, JPEG and TIFF (feature `export`)
│   ├── laminapdf-render-pdfium/  # edge: the PDFium adapter (pdfium-render, bound at runtime to pdfium-binaries chromium/7881;
│   │                    #     tests and render goldens #[ignore]d: `cargo xtask render-golden` runs them)
│   ├── laminapdf-desktop-protocol/  # app, unpublished: the frames between the desktop main process and its engine
│   │                    #     (ToEngine/FromEngine, postcard header + raw payload, MAX_FRAME, PROTOCOL_VERSION) and
│   │                    #     `supervisor` (std only: spawn, Hello/Ready, replies by ReqId, ReadRange served, heartbeat
│   │                    #     watchdog, restart with backoff, 4th crash in 60 s = failed; `fake` with `test-support`) (D2b);
│   │                    #     SandboxLevel/Layer (an engine missing a required layer is refused), Fonts/FontRequest/FontData
│   │                    #     with FontServer (D2c); protocol 5: RenderPrintBand/PrintBand, a page for printing in bands of
│   │                    #     ≤ 16 MiB at a dpi with the print intent, TileFormat::Bgra8 (D4c); protocol 9: GetProperties/Properties, the Properties
│   │                    #     dialog's data as one JSON string (E1j)
│   ├── laminapdf-desktop-engine/  # app, unpublished: the desktop engine binary (ADR 0008 §2): stdin/stdout frames, a router
│   │                    #     thread that never parses, one worker (lamina model + PDFium), RemoteSource (256 KiB blocks
│   │                    #     read over the pipe); examples large_pdf (the 1 GB fixture) and measure (D2b);
│   │                    #     fonts.rs (PDFium's system fonts, asked of the main process); --sandbox-selftest[=<probe>]
│   │                    #     (feature `selftest`, selftest.rs), tests/sandbox_selftest.rs (D2c); print.rs (the print bands'
│   │                    #     maths: band height, the dpi a page fits, BGRA; D4c); properties.rs (E1j: the Properties dialog's facts, metadata
│   │                    #     and XMP packet in @laminapdf/ui's shapes, as JSON)
│   ├── laminapdf-desktop-sandbox/  # edge, unpublished: the engine's sandbox (ADR 0008 §5), both halves: spawn_sandboxed /
│   │                    #     SandboxSpawner (Windows: job, Low-IL restricted token + start-up impersonation, mitigation
│   │                    #     policies, CreateProcessAsUserW; Unix: only fds 0–2 inherited; macOS: memory watchdog; names
│   │                    #     sandbox exits (SIGSYS, memory limit)), SystemFonts/BackgroundFonts (the font faces the main
│   │                    #     process serves) and, feature `engine`, apply_engine_sandbox (macOS Seatbelt; Linux
│   │                    #     no-new-privs, Landlock, RLIMIT_AS, seccomp; Windows RevertToSelf + post-load policies);
│   │                    #     `probe` (the selftest's, test only); `seccomp-trap`; LAMINA_PDF_ENGINE_SANDBOX=off in debug
│   │                    #     builds only (D2c); the seccomp allow-list lets the crash handler use its one connected socket (D7d)
│   ├── laminapdf-desktop-crash/  # edge, unpublished: native crash capture (D7d, ADR 0014 §10). capture.rs (every OS; on Windows
│   │                    #     through the patched minidumper in third_party/): the monitor process (the app binary with
│   │                    #     --crash-monitor) hosts a minidumper DumpServer for the main process and one for its engines
│   │                    #     (on Windows the engines' is a debugger, capture/debugger.rs: the engine opens no socket);
│   │                    #     capture_crashes installs the crash-handler client
│   │                    #     each process connects before lock-down (both through capture_crashes_within, which waits for a monitor still starting). summary.rs (feature `summary`, only the engine): summarize_bytes
│   │                    #     reads a dump into the allow-listed DumpSummary, which the engine's `--summarize-dump` mode runs
│   │                    #     sandboxed so the main process never links the parser. `debug-crash` (fault), `test-support` (TestDumpServer)
│   ├── laminapdf-desktop-os/  # edge, unpublished: the desktop main process's calls into the OS's shell (D3b): the recent
│   │                    #     documents (macOS NSDocumentController, Windows SHAddToRecentDocs and the Jump List of the
│   │                    #     AppUserModelID set at start-up, Linux GTK RecentManager), on the main thread; open_shared and file_identity
│   │                    #     (D3c: the share mode documents open with, the file an open handle reads, volume and file ID or device and inode; example reopen-cost);
│   │                    #     print/ (D4c: PrintSource of bands, checked; macOS: AppKit's panel + a Core Printing session per job,
│   │                    #     a page format per page; Windows: PrintDlgExW + GDI's ResetDCW/StretchDIBits; Linux: GtkPrintOperation
│   │                    #     + cairo; choose on the main thread, spool anywhere (Linux: print, main thread); tests/print_to_file.rs,
│   │                    #     its own main(), prints four page sizes to the OS's PDF printer on macOS and Windows)
│   ├── laminapdf-render-wasm/  # edge: wasm-bindgen bindings of the hayro adapter (flat numbers and bytes), built by
│                        #     `cargo xtask wasm` as lamina_pdf_render_hayro into @laminapdf/wasm (C2c)
│   └── laminapdf-test-fixtures/  # dev, unpublished: the Rust test helpers that crates share (fixtures_dir, fixture, corpus,
│                        #     json, classic/classic_with_root/stream_body, Rng, read_at, the codes.json loader); no Lamina
│                        #     PDF dependencies, a path-only dev-dependency (Q2c); xtask uses its rules for which codes.json keys are groups
├── packages/wasm/       # @laminapdf/wasm: committed package.json + size-budgets.json, generated dist/
│                        #   (lamina_pdf_base / lamina_pdf_text / lamina_pdf_extract / lamina_pdf_render_hayro .js, .d.ts, .wasm),
│                        #   test/smoke.test.ts (Vitest, Node)
├── packages/render/     # @laminapdf/render: ADR 0004's web RenderBackend interface, planTile/pageToTile (the Rust
│                        #   planner's port), RenderError
├── packages/test-utils/ # @laminapdf/test-utils (private, workspace-only, outside Changesets): the test helpers the
│                        #   packages share, exported as TypeScript source: render-contract (the RenderBackend suite both
│                        #   adapters run, Node + browsers), browser-harness, browser-config (renderBrowserConfig),
│                        #   firefox (firefoxProvider, firefoxEnv), flaky (retryOnceAndReport: one retry and the flaky report, ADR 0018 §4),
│                        #   root (fromRoot), wrangler-dev (the sites' e2e server over HTTPS, getLocal)
├── packages/render-pdfium/  # @laminapdf/render-pdfium: @embedpdf/pdfium 2.15.1 (pdfium.wasm SHA-256 checked by
│                        #   scripts/verify-pdfium.mjs) behind RenderBackend; THIRD_PARTY_NOTICES.md
├── packages/render-hayro/   # @laminapdf/render-hayro: lamina_pdf_render_hayro behind RenderBackend
├── packages/viewer/     # @laminapdf/viewer: the viewer core (ADR 0009); C3a: ViewerClient (typed RPC over
│                        #   @laminapdf/sdk/worker's request(), timers, restart), src/worker/ (serve() with the viewer's
│                        #   methods: openDocument, render → ImageBitmap); C4c: src/source/http-range.ts (HttpRangeByteSource:
│                        #   a `{ url }` source fetched in the worker, block cache read by the backend, data-pending refills); U4a:
│                        #   src/network/watch.ts (watchNetwork: fetch, XHR, WebSocket, sendBeacon wrapped, Resource Timing for the rest),
│                        #   worker/network.ts (the worker's log, imported first; networkLog, `network`/`csp-violation` events); C5a: src/layers/ (the text
│                        #   layer: a transparent span per run from Lamina PDF's quads, selection across pages, keepMounted pins, copy; the
│                        #   structure tree as ARIA elements owning spans by MCID), worker/text.ts (pageText, structure); C6a: worker/navigation.ts
│                        #   (outline, namedDestination), zoom/destination.ts (a page-space target → zoom and scroll; Viewer.goToDestination),
│                        #   src/thumbnails/strip.ts (ThumbnailStrip: virtualised, own pool and 32 MB cache, background priority); C5b: src/find/
│                        #   (fold.ts: pdf.js-like folding; page.ts: page text mapped to glyphs, matches as quads; controller.ts:
│                        #   FindController, viewer.find), worker/find.ts (four pages per call, background), layers/highlight-layer.ts; C5c: find.results() with snippets (page.ts's snippetOf), goTo; C6b: layers/link-layer.ts (anchors over page links: internal ones as history jumps, web links
│                        #   through onExternal, the rest inert), layers/link-url.ts (externalLink: http(s) and mailto only, punycode),
│                        #   history/history.ts (NavigationHistory; viewer.history, jumpToPage, jumpToDestination), worker pageLinks; C6c: worker/print.ts
│                        #   (printPage: a page at print dpi with the print intent, ≤ 16 MP, as a PNG; fileBytes: a remote file's bytes as opened),
│                        #   worker properties; C7b: source/opfs.ts (OpfsByteSource: a local file over 200 MiB (`copyAbove`) copied once into
│                        #   OPFS `documents/` and read through a sync access handle, for Lamina PDF and the backend); T5b: worker/fingerprint.ts
│                        #   (documentFingerprint: "fp1:" + SHA-256 of the head through its first %%EOF within 64 KiB; OpenedDocument.fingerprint),
│                        #   Viewer.viewPosition()/restore(ViewPosition: a history place plus rotation, a history jump), a pending place
│                        #   for a view opened or restored while hidden; T5a: layers/link-preview.ts (previewRegion: a destination's
│                        #   crop in page space, its text column, zoom clamped 100–150 %, ≤ 480 × 180 CSS px; previewRender: the turned
│                        #   device-pixel clip), view/previews.ts (LinkPreviews: one visible-priority render, 8-picture LRU;
│                        #   Viewer.preview), the link layer's linkpreview/linkpreviewend events (300 ms hover or focus, 500 ms
│                        #   long-press), holdLinkPreview/dismissLinkPreview/followLinkPreview; E1g: worker/edit.ts (applyEdit: ops as one transaction, an incremental save
│                        #   of the appended revision into memory, the render document reopened over the file with every revision appended and
│                        #   swapped in once open; a failure undoes the transaction), viewer.invalidate(pages, sizes?) (bitmaps dropped by a per-page
│                        #   revision in the view key (viewKey), late answers of an older revision dropped, keptTop's layout, layers remounted,
│                        #   find.refresh, a restarted worker's sizes taken back, the strip laid out from viewer.pageSizes(id) on its `invalidate`
│                        #   event), viewer.applyEdit and client.applyEdit; E1f: client.metadata() (worker `metadata`: the edited document's, never the text copy's); E2h: client.wouldViolate(ops), EditResult's `references` and `fields`, a hidden view keeping its place (drawing nothing, back there with no history entry, a jump as it shows going where it says); E1d: worker/session.ts (the edit session's history: steps with
│                        #   label, pages, time; position, 500 deep, the saved position for the unsaved count), edit.ts's undoEdit/redoEdit/
│                        #   goToStep (one redraw, taken back on failure)/editHistory/restoreEdits, src/edit-log.ts (the page's copy of each
│                        #   step's ops, replayed into a restarted worker), edit/session.ts (EditSession, viewer.edits: calls in order, busy,
│                        #   blocked, the partial undo's notice), client `edit` event, OpenedDocument.encrypted, fit modes refit after a resize;
│                        #   E2l: edits that change the page list (the worker checks the order against `Edited.pageOrder`; `EditResult.order`,
│                        #   `EditStep.pageIds`), layout/page-moves.ts (pageMoves: old index → new, a deleted page's nearest survivor),
│                        #   the viewer's `#reorder` (bitmaps rekeyed by `viewKeyAt`, revisions, find.remap, history.remap, the top page kept)
│                        #   and its `reorder` event, which the thumbnail strip follows; E2m: worker/sources.ts (previewSource,
│                        #   renderSource, closeSource, addSource, into the text copy too), worker/parts.ts (writeParts, mergeFiles), worker/zip.ts
│                        #   (a stored zip, APPNOTE 6.3.10) and crc32.ts; files.ts (those calls as functions over a client), client.addSource,
│                        #   the edit log keeping each named source's bytes for a restarted worker's replay (none for encrypted ones);
│                        #   C3b: createViewer (src/layout/, view/, canvas/,
│                        #   render/, zoom/), demo/viewer-demo.html, examples/show-a-pdf.ts (the docs site's example); test/browser/ (Chromium, Firefox, WebKit);
│                        #   C3c: perf/ (the performance harness: seeded fixtures, page/ with globalThis.__laminaPerf,
│                        #   Playwright scenarios (T5a's link-previews: 20 citations hovered, reported), report.ts against
│                        #   web-app §4, serve.ts gzip server; `pnpm … perf`)
├── packages/cli/        # @laminapdf/cli (L4d; private in the workspace at 0.0.0, outside Changesets: its version is the
│                        #   CLI crate's): bin/laminapdf.js runs src/launcher.ts, which finds this platform's
│                        #   @laminapdf/cli-<os>-<cpu>[-<libc>] package (platforms.json, the one list; src/platform.ts) and
│                        #   runs its binary; no install script; test/ (platform resolution, generated packages vs dist's targets)
├── packages/layout-heron/  # @laminapdf/layout-heron (private, L3h): Heron for extractWithModels with the app's onnxruntime-web
│                        #   (an `OnnxRuntime` interface of the calls it makes; no runtime dependency): pre.ts (Pillow's bilinear
│                        #   resize over white), post.ts (labels, scores ≥ 0.3, duplicate boxes, pixels → points), HeronLayout.load
│                        #   (SHA-256 by WebCrypto) and detect; test/ (a fake runtime)
├── packages/sdk/        # @laminapdf/sdk: the TS API over @laminapdf/wasm (init(), Builder, Document, Font, LaminaPdfError);
│                        #   entries per export condition (src/index.ts, src/entry/), src/generated/ (codes, explain text),
│                        #   src/explain.ts (the ./explain subpath), src/worker/ (the ./worker subpath: the worker client), schemas/ (laminapdf.error/1),
│                        #   examples/ (byte-identical to the Rust goldens; report.ts; open-untrusted.ts; worker/: a Cloudflare Worker),
│                        #   test/ (Vitest: Node, Vite; suite/: the example list every runtime runs, browser/, deno/,
│                        #   bun/, workerd/; vitest.{node,browser,workers}.config.ts)
├── packages/tokens/     # @laminapdf/tokens (private, outside Changesets): the Paper design tokens (ADR 0024 §7): tokens.json
│                        #   (DTCG 2025.10; status, role and contrast pairs under $extensions["dev.lamina"]) →
│                        #   scripts/generate.ts → src/generated/tokens.css (--paper-*, :where() blocks per theme, accent,
│                        #   appearance), tokens.ts (THEMES[theme][accent], PAGE_APPEARANCES …), CONTRAST.md; test/ (pairs,
│                        #   coverage, design-language hexes; browser/: high contrast in Chromium, Firefox, WebKit)
├── packages/fonts/      # @laminapdf/fonts (private, outside Changesets): the UI typefaces, self-hosted (U6b): files/*.woff2 (Lamina
│                        #   Sans: Source Sans 3 renamed, its licence reserving 'Source'; Newsreader; Caveat; Latin and Latin Extended),
│                        #   pdf/*.ttf for PDFs (Lamina Sans Regular for built pages; Noto Sans Arabic, text boxes' fallback, E4j),
│                        #   fonts.css (generated, with a metric-adjusted Arial fallback face), fonts.lock.json (npm sources + SHA-256,
│                        #   outputs, coverage), LICENSES/, FONT_PRELOADS (src/index.ts); test/ (lock, CSS, budgets, coverage, font-CDN
│                        #   guard; browser/: what a page fetches, same-origin only); written by tools/fonts/subset.py
├── packages/icons/      # @laminapdf/icons (private, outside Changesets): the Paper stroke icons (design language §5): svg/*.svg
│                        #   (one per icon) + aliases.json → scripts/generate.ts (lints every icon: scripts/rules.ts, svg.ts) →
│                        #   src/generated/icons.ts (ICONS, IconName); iconSvg, iconButtonProps (root entry, no framework); <Icon>
│                        #   (./react, React 19 optional peer); gallery/ (`pnpm … gallery`); test/ (lint + fixtures, §5 tables,
│                        #   entry graph; browser/: <Icon>, gallery with axe)
├── packages/ui/         # @laminapdf/ui (private, outside Changesets): the Paper components for React 19 on React Aria Components:
│                        #   src/components/ (controls: Button, IconButton, ToggleButton, ToolToggle, ModeSwitcher, SegmentedTabs,
│                        #   TextInput, TextArea, Select, Checkbox, RadioGroup, Switch, SwatchGroup, PageAppearanceChips; display: Badge,
│                        #   Notice, Toast, ProgressBar, Skeleton, EmptyState, StatusBarItem, NetworkIndicator, RepairedNotice, PageLabel,
│                        #   Thumbnail, OutlineRow, ShortcutChip), src/overlays/ (the layer stack behind Escape, Dialog, Popover and
│                        #   PopoverTrigger, Menu and MenuTrigger, Tooltip, ToastRegion with ToastQueue, BottomSheet, SelectionToolbar,
│                        #   LinkPreview (T5a: a tooltip under its link, or a sheet on touch), CommandPalette; z-order from --paper-z-*), src/dialogs.ts (the ./dialogs entry: PrintDialog, PrintProgressDialog, InitialViewDialog and ThreadsPanel (E3d: reader/initial-view-form.ts, the change as one setInitialView patch; reader/threads.ts's stepThreadPart is in the main entry),
│                        #   PropertiesDialog, which the web reader loads once a document opens, issue #380; E1f: with `editing`, a Description tab whose form
│                        #   (reader/properties-editor.tsx, properties-form.ts: changeOf → one setMetadata patch and a removeMetadata list,
│                        #   fieldWarnings, PDF dates) Apply turns into one edit (reader/properties-ops.ts: propertiesOps, both apps' ops; E1j), and a File tab; E3e: an XMP tab (reader/xmp-editor.tsx: a
│                        #   tree of schemas, the property's type and value, Add property…, Raw XML checked as typed; xmp-model.ts: the
│                        #   packet read by DOMParser, XmpChange → setXmpProperty/removeXmpProperty/replaceXmpPacket)), src/capabilities.ts (the ./capabilities entry: CapabilityPage,
│                        #   help/: the O05 page over docs/capabilities/matrix.json, filters kept in platform storage), src/platform/ (ADR 0010: the Platform interface and capability
│                        #   flags, createWebPlatform (feature-detected pickers, canWriteDirectory's chooseFolder (folder.ts, loaded on use), downloads, IndexedDB and OPFS storage, recent files;
│                        #   T5b: Platform.positions, positions.ts: the `positions` store of `lamina-pdf` v2, 500 by `readAt`; idb.ts opens
│                        #   the database once; recent.reopen(id, { ask: false })),
│                        #   PlatformProvider, usePlatform, useCapability), src/state/ (ReaderController over the reader machine,
│                        #   useStoreState, useReaderState), src/reader/ (ReaderShell, ReaderTopBar (Back and Forward), PageInput, OutlineTree, FindBar, FindResults, ExternalLinkDialog, ZoomControl,
│                        #   ReaderView and ReaderSettingsPanel (reader-view.tsx: R14's column, crops, settings; U4b),
│                        #   PrintDialog and PrintProgressDialog (print-pages.ts: ranges of numbers and labels), PropertiesDialog (properties.ts:
│                        #   XMP via DOMParser, page sizes by paper, Copy all text); platform/print.ts: the web print frame (named @page rules
│                        #   in an adopted stylesheet, CSP-safe),
│                        #   PasswordDialog, useGatedEntries, OpenDropZone, RepairDetails, OpenFailure, WholeFileDownload, DebugMenu;
│                        #   codes.ts: a sentence per codes.json warning and error; the viewer as a shape), src/shell/ (U1c, ADR 0024 §1:
│                        #   ShellLayout, the rail's open state, width and tab kept in platform storage, focus mode per window, the
│                        #   inspector's content; ModeRegistry; LeftRail (nav, resizable by pointer and arrows, a drawer below 1024 px);
│                        #   Inspector (docked, a selection's is a layer); FocusModeItem; shellActions for view.toggle-sidebar,
│                        #   view.focus-mode and mode.<id>; useShell), src/organise/ (E2h, Paper O06: PageGrid, a React Aria ListBox in a virtualized GridLayout with multiple selection, drag with an insertion bar, ⇧⌘/Ctrl Shift + arrows to move, R/⇧R/Delete and the page menu, tiles drawn by page ID; moves.ts (gaps, moveToGap, stepGap, pageRanges); dialogs.tsx (Move to page…, Blank pages…, the signed-page question); OrganiseToolbar; commands.ts (pages.*); report.ts (toasts and step labels); E2m: from-file.tsx, merge.tsx, split.tsx (Extract pages too), parts.ts, on-open.ts; in the ./editing entry), src/edit/ (E1d: EditControls, the session as a shape; E3b: BookmarkEditor, NamedDestinationsDialog, bookmark-moves.ts; HistoryPanel,
│                        #   the virtualised History tab), commands/edit-commands.ts (edit.undo/redo with ⌘Z, ⇧⌘Z/Ctrl Y, `fields: false`;
│                        #   edit.history, edit.go-to-step, pages.rotate-*), src/tools/ (ToolRegistry: select, hand, textSelect keyed by
│                        #   InteractionModes, extended by declaration merging; attachTools), ReaderController.setMode and its close guard
│                        #   (setCloseGuard, requestClose, open(file, { discard })); src/editing.ts (the ./editing entry: EditArea, ReaderTopBar's
│                        #   `editArea` with the save state, Undo and Redo; HistoryPanel; the edit commands; the tools) and src/reader-mode.ts
│                        #   (the ./reader-mode entry: ReaderView, ReaderSettingsPanel), loaded once a document opens (#380),
│                        #   src/messages.ts (the catalogue) and src/later-messages.ts (the words of the lazily loaded properties form and capability page, #380),
│                        #   examples/ui-demo.{html,tsx} (`pnpm --filter @laminapdf/ui demo`: the chrome over a stand-in viewer, built with
│                        #   the React Compiler through Babel, vite.demo.config.ts),
│                        #   src/density.tsx (DensityProvider), src/styles/ (./styles.css: tokens
│                        #   only, @layer paper.*); test/ (states.tsx, fakes.ts (fakePlatform with any flags); Node: CSS tokens only,
│                        #   import boundary, §7 map, the reader machine and controller, the web platform over fake windows, the React
│                        #   Compiler over every component (compiler.test.ts); browser/: the demo page with axe,
│                        #   keyboard, ARIA values, reduced motion (a Playwright command), axe in 3 themes × 2 directions, target sizes)
├── packages/site-kit/   # @laminapdf/site-kit (private, outside Changesets; build time only): what both Astro sites share.
│                        #   src/integrations.mjs: the CSP's inline-script hashes into dist/_headers, X-Robots-Tag noindex unless
│                        #   LAMINA_PDF_ENV=production, src/robots.mjs (W3a: the one robots.txt of the three sites, every crawler
│                        #   let in, the sitemap named in production; robots, isProduction, robotsProblems), the website's checks of every built page (ADR 0011 §3: same-origin loads, no
│                        #   inline styles, cookies or beacons, title, description, canonical URL, internal links), pageFiles, pageExists;
│                        #   src/version.mjs (withVersion: the pages' `{{version}}`, P4); src/structured-data.mjs (W3d: the
│                        #   home pages' JSON-LD, Organization/WebSite/SoftwareSourceCode from Cargo.toml, no rating, review or
│                        #   offer; data blocks stay out of the CSP's hashes) and src/lastmod.mjs (the sitemaps' dates from git,
│                        #   a production build refusing a shallow checkout); src/code-kinds.mjs (Q4u: Shiki's TextMate scopes → the five
│                        #   --paper-code-* kinds, for the website's code panel and the docs' code blocks);
│                        #   the site header (W1d): src/nav.mjs (the one list of links, siteNav per site and page, APP_LAUNCHED),
│                        #   src/SiteHeader.astro, SiteNav.astro (the links alone) + header.css (sized per site by CSS variables); the select (W1g):
│                        #   src/MenuSelect.astro + menu-select.ts + menu-select.css (Q4w: a select-only combobox with an icon per choice,
│                        #   §7.19's field opening §7.26's listbox popup, 44 px on touch), icon.mjs
│                        #   (decorativeIcon: a Paper icon's ?raw SVG as hidden HTML); test/ (Node), test/browser/ (the header and the select
│                        #   rendered by Astro's container in a global setup, then Chromium, Firefox, WebKit: tab order, current underline,
│                        #   phone width, the select's name, keys and focus ring, axe; run by `cargo xtask pdf-runtimes`); README.md
├── apps/web/            # the web reader (C4b; ADRs 0009, 0010, 0011): a Vite 8 single-page app (React Compiler) of @laminapdf/ui's
│                        #   chrome around @laminapdf/viewer: src/session.ts (the engine: the document worker started at load, opens,
│                        #   warnings, the #debug backend switch, remote files as PlatformFiles naming their address), src/app.tsx,
│                        #   src/document.ts (window title, repair list; Node tests), src/open/ (C4c: url-fragment.ts parses and
│                        #   writes `#url=…&page=…` keeping raw values, navigation.ts, watch.ts on load/hashchange/popstate; C6b: history.ts,
│                        #   each viewer jump a browser history entry marked with its id), src/sw/service-worker.ts and plugins/pwa.ts (C7b: the
│                        #   service worker (the shell and WebAssembly precached, same-origin GETs only, headers kept, updates on Reload) and the
│                        #   manifest from the tokens with file_handlers), src/offline/ (updates, online, control, launch, install, document-cache;
│                        #   R15's notice, toasts and install dialog), src/recent.tsx (recent files and Settings › Storage and privacy),
│                        #   src/output.tsx (C6c: Print, Download a copy and Document
│                        #   properties with their dialogs and toasts; properties-editing.tsx, lazily loaded: the properties form's edits), src/download-name.ts (a safe file name),
│                        #   src/network/ (U4a: page-feed.ts, main.tsx's first import; classify.ts, the one classifier of app files,
│                        #   document traffic and unexpected sites; monitor.ts, the log and its summary; panel.tsx, R13's indicator and panel);
│                        #   src/reader-mode.tsx (U4b: the Reader toggle, the column over the reflow, crops, the place kept both ways, settings;
│                        #   U4c: reader-origin.ts outlines a block on its page for View original page, reader-crop.ts a crop's clip);
│                        #   src/initial-view-editing.tsx, threads-tab.tsx and thread-parts.ts (E3d: the Initial view dialog over the edit session, the Threads tab, the part followed last; the file's opening page and zoom in session.ts, after the first paint; its panel and the thread-part commands' handlers in editing-runtime.tsx);
│                        #   src/documents.tsx (E2m: DocumentsController: Insert from a file, Replace, Merge, Split, Extract over the worker's sources
│                        #   and parts; outputs to a chosen folder, one or two saves, or a zip), src/several.tsx (several files opened from the OS);
│                        #   src/page-setup.tsx (E2g: Organise's Page setup ▾: crop, resize, page boxes, orientation and page labels over `OrganiseController.edit`);
│                        #   src/editing.tsx (E1d: the edit session in the reader: rotate the current page, the History tab, "•" in the
│                        #   tab's title, beforeunload) and editing-runtime.tsx (the commands, toasts, the close confirmation, the tools),
│                        #   src/comment/ (E4f, E4j: Comment mode: controller.ts, tools.ts (one state machine per tool), marks.ts and ops.ts
│                        #   (marks and their edits, Node tests), changes.ts (the inspector's edits), mode.tsx (toolbar, inspector, commands);
│                        #   text boxes (C03): rich-text.ts (contenteditable ↔ the RichText subset), text-session.ts (a typing session's one
│                        #   edit), text-editor.ts (the box typed into, a page layer), text-boxes.ts (opening boxes, the text-capable copy,
│                        #   the bundled Arabic fallback font));
│                        #   src/organise.tsx (E2h: OrganiseController, the Organise mode registered while a document is open and its
│                        #   toolbar, grid, inspector and dialogs; each edit asked of wouldViolate first (signed page, certification) and
│                        #   toasted with Undo; the viewer hidden under the grid; ModeArea, the top bar's Read/Organise switcher; loaded with later.ts);
│                        #   src/bookmarks.tsx (E3b: the Outline tab's Edit bookmarks, the bookmark editor and the named
│                        #   destinations dialog, each action one op; loaded with later.ts),
│                        #   src/later.ts (the one chunk loaded once a document is open: dialogs, reader mode's column, the editor's parts, the capability page);
│                        #   src/capabilities.tsx (E1f: the /capabilities route over the reader, More › What Lamina PDF can do; capability-page.tsx, lazily loaded, bundles the matrix and owns the page's Back; capability-address.ts the address they share);
│                        #   src/reading/ (T5b: positions.ts parses, records and restores reading positions by fingerprint; resume.ts
│                        #   reopens the document open at the last close; recent.tsx's Continue reading and the session record);
│                        #   wrangler.jsonc (lamina-pdf-web at app.laminapdf.com, env.staging lamina-pdf-web-staging at
│                        #   staging-app.laminapdf.com; static assets only, SPA fallback), public/_headers (ADR 0011 §4 exactly);
│                        #   plugins/search.ts (W3a: robots.txt, and X-Robots-Tag noindex unless LAMINA_PDF_ENV=production, which only
│                        #   web-deploy.yml's production-checks sets; W3d: that build alone gets a canonical link to / and share tags);
│                        #   e2e/ (Vitest + Playwright against `wrangler dev`: headers, opening four ways, no CSP violation or other
│                        #   origin; remote-host.ts, an HTTPS fixture host with CORS and ranges per path, for remote, encrypted and
│                        #   privacy suites), run by `cargo xtask pdf-runtimes <browser>`; e2e/smoke.test.ts (the deployed reader at
│                        #   LAMINA_PDF_SMOKE_URL, Chromium, after e2e/live.ts, run by Node first, waits for the host and names who refused it); deployed by
│                        #   .github/workflows/web-deploy.yml (C4d: a preview per PR on the Worker lamina-pdf-web-preview, staging
│                        #   from main, production from web-v* tags on main, rollback on a failed smoke test); README.md, DEPLOY.md
├── apps/desktop/        # the desktop reader (D2a; ADR 0008): @laminapdf/ui's chrome in a Tauri 2 window (private, outside
│                        #   Changesets: its version is the crate's). src/main.tsx (the only isTauri() call, checked by `check ts`),
│                        #   src/platform/desktop.ts (the web platform plus canWriteInPlace, canWatchFiles, hasNativePrint,
│                        #   multiWindow; their calls reject with NotYetSupportedError naming the feature), src/app.tsx (top bar, home
│                        #   screen, version from the `app_info` command); src/print.tsx (File ▸ Print: print_document, the
│                        #   status-bar progress item with cancel_print; D4c); src/properties.tsx (E1j: File ▸ Document properties…
│                        #   over `document_properties`, edits through the session; useOutput groups it with Print); showing a document waits for D2d. src-tauri/: the
│                        #   workspace crate laminapdf-desktop (layer app, publish = false): lib.rs (plugins dialog/log/opener for
│                        #   Rust only, window `main` built in Rust, Engines managed state, window close/app exit stop engines),
│                        #   print.rs (D4c: print_document, cancel_print, e2e_print_to_file; a PrintSource over the window's
│                        #   engine at 300 dpi, engine/print.rs checking each band), commands.rs (`app_info`, `open_dialog`,
│                        #   `page_info`, `close_document`; `debug_kill_engine` with
│                        #   feature `e2e`), startup.rs (feature `e2e`: the cold-start marks, logged by the first
│                        #   `app_info`), engine/ (D2b: Engines = one supervised engine per window, engine://restarted and
│                        #   engine://failed to that window; documents.rs DocumentRegistry, a window reads only its own docs;
│                        #   bytesource.rs FileSource, positioned reads, no mmap, on Windows the path reopened for each read and checked (D3c); secret.rs SessionPassword, locked and wiped),
│                        #   open.rs (D3a: the OS's opens (argv, single-instance, macOS RunEvent::Opened, drops) through
│                        #   route_open, queued in PendingOpens until the webview's take_pending_opens; open://document,
│                        #   open://drag-enter|leave, open://refused), linux/laminapdf.desktop (Exec with %F),
│                        #   opened.rs (D3b: every open from a path: engine, recent lists, watcher; the fingerprint),
│                        #   recent.rs (recent.json, 20 entries by canonical path, ids for the webview), prefs.rs
│                        #   (preferences.json, Settings › Motion and Graphics among them), graphics.rs (D7b: Linux's
│                        #   WebKitGTK workarounds chosen before the builder, an in-place `exec` instead of `set_var`,
│                        #   --safe-graphics, the crash-loop guard in graphics-starts.json), motion.rs (U7f: GNOME's reduce-motion setting, which
│                        #   WebKitGTK misses under Wayland, read by gsettings for `system_motion`; src/motion.ts applies it), store.rs (atomic JSON), watch/ (notify on each open file's folder,
│                        #   stamp.rs's settle rule, app.rs's reload as a new document; document://reloaded|missing|
│                        #   unreadable); crash/ (D7a: opt-in crash reports: report.rs, the allow-list matching
│                        #   apps/updates/schemas/crash-report.schema.json; scrub.rs; consent.rs, crash.json; queue.rs, held/ and
│                        #   outbox/; upload.rs, POST /crash without the install ID; the panic hook; src/crashes.tsx, S08's question,
│                        #   its tab choice after an unexpected quit (D7e: session/marker.rs's `running` mark, the held-back
│                        #   session, session_recovery and session_reopen; @laminapdf/ui's CrashRecoveredDialog)
│                        #   and Settings › Privacy); src/recent.tsx, settings.tsx (Settings: Files, Appearance and accessibility's Motion, and the
│                        #   other sections' registry), graphics.tsx (Settings › Graphics, Linux only), about.tsx, motion.ts, window-state.ts
│                        #   (the webview's context menu off the chrome, dimmed bars in an inactive window), file-changes.tsx,
│                        #   error.rs (DesktopError as { kind, message }), build.rs (app manifest: each command needs an allow-*
│                        #   permission), tauri.conf.json (Lamina PDF, com.laminapdf.desktop, strict CSP, frozen prototype,
│                        #   capabilities ["main"]; no externalBin), tauri.bundle.conf.json (the engine sidecar and PDFium, merged
│                        #   by desktop:dev/desktop:build), tauri.windows.conf.json and windows/ (D6d: per-user NSIS, per-machine MSI with a fixed
│                        #   upgrade code; installer-hooks.nsh and registry.wxs register for PDFs through OpenWithProgids, never the default, and the
│                        #   MSI's InstallFlavour; artifact-signing.json), ../scripts/ (sign-windows.ps1, verify-windows-signatures.ps1,
│                        #   check-windows-installers.ps1), capabilities/main.json (the four commands' allow-*),
│                        #   tests/capabilities.rs (the capability guard; the e2e CSP is the base one plus 'unsafe-eval'),
│                        #   e2e/ (D2e: WebdriverIO through the app's embedded WebDriver server, feature `e2e` +
│                        #   tauri.e2e.conf.json: wdio.conf.ts, app.ts helpers, specs/ (eleven scenarios in one app, in
│                        #   order; cold-open in its own run, wdio.cold-open.conf.ts, with the recent list after a restart;
│                        #   the session and crash-recovered pairs in runs of their own, each clearing the `running` mark the
│                        #   service's kill leaves, but for crash-recover), MANUAL-OPEN.md, probe/ (imported only in Vite's e2e mode), cold-start.ts; src-tauri/src/e2e.rs's
│                        #   e2e_queue_pick and e2e_interactive), test/app-info.json
│                        #   (the app_info sample both languages' tests read), about/
│                        #   (cargo-about, MPL-2.0 accepted for Tauri's dependencies); Linux builds need WebKitGTK 4.1 and D-Bus
│                        #   (.github/actions/linux-webkit); README.md
├── apps/updates/        # the desktop update service (D5a; ADR 0014): a Cloudflare Worker (@laminapdf/updates, private) at
│                        #   updates.laminapdf.com (env.staging: staging-updates.) answering the Tauri updater's
│                        #   /{channel}/{target}/{arch}/{current_version} with 204 or 200 JSON from channels/{channel}.json in R2
│                        #   (lamina-pdf-releases: stable, beta; lamina-pdf-nightly; lamina-pdf-staging); src/route.ts (allow-lists,
│                        #   400/404), manifest.ts (types and validator; schemas/manifest.schema.json), rollout.ts (Lamina-Install-Id
│                        #   as a UUID v4, SHA-256 buckets of 10 000), select.ts (bridges, whatever the pause; release in its rollout;
│                        #   previous), cache.ts (one Cache API layer, 30 s, checked against the copy's read time), handler.ts,
│                        #   index.ts (the only file on workerd's types, src/generated/ from `pnpm run types`); crash.ts and
│                        #   crash-endpoint.ts (D7a: POST /crash checked against schemas/crash-report.schema.json, path-like free
│                        #   text refused, 64 KiB, rate-limited, into the private bucket lamina-pdf-crash-reports); scripts/manifest.ts
│                        #   (show, validate, put, pause, resume, set-rollout, promote; R2's S3 API, `--local` on Wrangler),
│                        #   scripts/r2.ts (the S3 client per-bucket R2 tokens need, SigV4 via aws4fetch; `pnpm run r2
│                        #   put|delete`), scripts/downloads.ts (`manifest downloads`: downloads/<channel>.json, the
│                        #   installers with their SHA-256 from SHA256SUMS, for the website's download page), smoke.sh;
│                        #   src/channels.ts (the channels and their buckets, the one map; a test holds wrangler.jsonc to it);
│                        #   unit tests beside their sources (src/*.test.ts, scripts/edits.test.ts, scripts/r2.test.ts), test/ (handle in Node, the
│                        #   bundled Worker in workerd through Miniflare, a log-leak test); invocation logs and traces off;
│                        #   deployed by .github/workflows/updates-deploy.yml (staging, then production in the
│                        #   `updates-production` environment with a rollback on a failed smoke test, from main); README.md has the updater
│                        #   public keys and the owner's setup
├── apps/docs/           # the docs site (Astro + Starlight, ADR 0015) in the Paper look (src/styles/paper.css over @laminapdf/tokens and
│                        #   @laminapdf/fonts): quick starts, guides, recipes, troubleshooting, TypeDoc
│                        #   reference for @laminapdf/sdk (generated, gitignored), rustdoc page; code from the tested example
│                        #   files (src/components/Snippet.astro; Example.astro adds a Result tab with pictures of each page, apps/docs/public/examples/preview/ from
│                        #   `cargo xtask docs-previews`, checked against the goldens by `check docs`, or an Output tab; B7e: "Edit and run" on six TS examples,
│                        #   runner/ (CodeMirror editor, a module worker at /run/worker.js running the edit with Sucrase, @laminapdf/sdk and
│                        #   @laminapdf/render-hayro, its own CSP with 'unsafe-eval' in public/_headers; Q4t: /run/types.js, TypeScript's
│                        #   language service over the SDK's .d.ts files for completion, types and errors, started on the first edit;
│                        #   Q4u: the code blocks in the editor's five Paper colours, plugins/code-kinds.mjs in ec.config.mjs, held to
│                        #   runner/kinds.ts by runner/colours.test.mjs), built by scripts/build-runner.mjs when
│                        #   the WebAssembly is built; e2e/ against `wrangler dev`, run by `cargo xtask pdf-runtimes <browser>`), the code table from fixtures/codes.json, the Benchmarks page
│                        #   from src/data/benchmarks.json (src/lib/benchmarks.ts checks it and makes the tables for the page's
│                        #   src/components/Benchmarks.astro and its Markdown twin), code pages from
│                        #   docs/errors/ (src/pages/, plugins/error-pages.mjs), post-build checks (plugins/), the shared site header
│                        #   (src/components/Header.astro, Starlight's Header override, with its search and theme switch; the theme switch is
│                        #   src/components/ThemeSelect.astro, Starlight's ThemeSelect override on site-kit's MenuSelect); public/_headers
│                        #   (ADR 0011 §4, inline-script hashes filled in after the build by @laminapdf/site-kit); wrangler.toml (Workers static
│                        #   assets: docs.laminapdf.com, no workers.dev address, and next.docs.laminapdf.com as [env.next]); robots.txt from the site kit
│                        #   (the sitemap only on the released docs, checked by plugins/release.mjs); deployed by .github/workflows/docs.yml; README.md has the owner's setup;
│                        #   for agents (L5a): a Markdown twin of every page, llms.txt and llms-full.txt (plugins/agent-docs.mjs,
│                        #   plugins/markdown.mjs), and scripts/package-docs.mjs, which writes the docs/ folder the packages ship;
│                        #   offline (Q4k): src/pages/one-page/[area].astro (each sidebar area on one page) and /pdf/lamina-pdf-guide.pdf
│                        #   (plugins/guide.mjs, scripts/guide-pdf.mjs: the twins through @laminapdf/sdk's Markdown builder, PDF/UA-1)
├── apps/site/           # the website at laminapdf.com (Astro, static, W1b; website plan §4) in the Paper look: src/layouts/Base.astro (head,
│                        #   font preload, the shared header from @laminapdf/site-kit), src/pages/ (the home page, for now the roadmap (W1c):
│                        #   src/components/home/, how-we-test.astro (T1b: src/components/how-we-test/, the latest full night's figures,
│                        #   the licence policy, where the code comes from), download.astro (D7b, S09: each channel's installers,
│                        #   src/scripts/download.ts guessing the visitor's system), 404 and the sitemap.xml endpoint; robots.txt from the site kit), src/data/ (check.ts:
│                        #   the readers' shared checks; downloads.json, the release lists' snapshot from `pnpm run downloads`
│                        #   (scripts/downloads.ts), with downloads.ts and downloads-snapshot.ts; progress.json from `cargo xtask progress` and its reader progress.ts; proof.json
│                        #   from `cargo xtask proof snapshot` and its reader proof.ts, how-we-test.ts its names and percentages;
│                        #   sources.ts: the code panel and packages from the tested examples, the CLI README and the docs install page,
│                        #   licencePolicy from Cargo.toml and deny.toml; copy.ts: the words, approved by the owner; rounds.ts: the areas' plain
│                        #   names; highlight.ts: the code panel's colours at build time, Shiki's tokens as classes coloured by the
│                        #   --paper-code-* tokens, since the CSP blocks inline styles), src/scripts/ (tabs.ts), scripts/share-image.mjs (public/og.png and the PNG favicons, run by hand) and story-image.mjs
│                        #   (~/git/lamina-stories/<story>/visuals/*.html → PNG at each file's lamina-size, styled by story-card.css), both over paper-render.mjs; public/_headers (ADR 0011 §4's website headers,
│                        #   test/headers.test.ts holds them to the ADR); @laminapdf/site-kit's checks after every build; wrangler.toml
│                        #   (lamina-pdf-site, the apex as a custom domain); deployed by .github/workflows/site.yml, which W1b's PR gives
│                        #   for the owner to commit; README.md has the owner's setup
├── apps/gallery/        # the Paper component gallery (U6f; private, dev-only, never deployed; Vite + React 19): one route per
│                        #   component group (src/routes.ts), states from @laminapdf/ui's `./testing` subpath (its test/states.tsx),
│                        #   overlays rendered one open state at a time (src/overlays.tsx); theme, page appearance, density, direction
│                        #   and motion in the URL; e2e/ (Playwright: visual.spec.ts screenshots in Chromium against
│                        #   e2e/__screenshots__/, a11y.spec.ts axe in three engines, fonts.spec.ts same-origin), test/ (Vitest:
│                        #   every exported component on a route, the Docker image matches @playwright/test); README.md
├── fixtures/            # shared test inputs: golden/ (byte-exact), corpus/ + expected.json + exceptions.toml
│                        #   + pdfjs-operators.json, content/ (operator fixture), differential/ (pypdf expectations),
│                        #   ops/v1/ (op JSON samples), fonts/ (OFL Noto subsets), images/ (JPEG, ICC), text/ (pypdf extraction facts),
│                        #   build/ (pypdf + pdf.js facts for laminapdf-build's goldens), signed/ (pyHanko), codes.json + codes.schema.json,
│                        #   errors/ (contract.json: shared error objects; marker-text.pdf),
│                        #   golden/examples/ and render-golden/ (the Lamina PDF examples), reveal/examples/ (the reveal example's report), reading-order/ (U2b's generated
│                        #   corpus, expected block order, ratchet threshold), reflow/ (U4b's tagged copies of those layouts, broken trees and
│                        #   mixed.pdf, with what reader mode should make of each); README.md has provenance
├── third_party/         # crates.io releases carried with a patch until upstream releases it, through [patch.crates-io]
│                        #   and outside the workspace's lints: minidumper 0.11.0, its Windows server checking the client's pid (D7f); README.md
├── fuzz/                # cargo-fuzz targets (OSS-Fuzz layout), its own nightly workspace outside the root one
├── tools/xtask/         # `cargo xtask …`: the single task entry point
├── tools/std14-gen/     # generates laminapdf-text's standard 14 / encoding / AGL tables from data/ (Adobe AFMs, AGL;
│                        #   provenance in data/README.md); `cargo run -p std14-gen [-- --check]`
├── tools/corpus-facts/  # pypdf facts for fixtures (Python, pinned in requirements.txt) and pdf.js operator counts
│                        #   (pdfjs-operators.mjs); never run by tests
├── tools/differential/  # pdf.js, PDFium, qpdf and pypdf runners for the nightly differential (pinned; own pnpm project);
│                        #   forms.mjs runs pdf.js's own scripting sandbox over a form and records what its fields show (E8f's fixtures/forms/*.pdfjs.json)
├── tools/fixtures/      # generators for fixtures written for Lamina PDF's tests (outline.py: no dependencies;
│                        #   fonts.py: fontTools, pinned in requirements.txt; text_facts.py: pypdf, qpdf;
│                        #   build_facts.py: pypdf, pdf.js, pdftotext, qpdf; images.py: no dependencies, macOS sips for the JPEGs;
│                        #   text_encrypted.py: qpdf)
├── tools/fonts/         # subset.py: @laminapdf/fonts' WOFF2 files, fonts.css and lock outputs from npm tarballs (fontTools, pinned
│                        #   in requirements.txt); run by hand when a font changes, never by tests
├── tools/cli-release/   # the CLI's release (L4d): generate.ts (the npm wrapper and platform packages from dist's archives,
│                        #   into target/cli-npm/), archive-test.sh and install-test.sh (an archive, the npm tarballs; run by
│                        #   cli-release.yml)
├── tools/roundtrip/     # pdf.js (Node) and PDFium (pypdfium2) checkers for `cargo xtask roundtrip`; pinned, never run by tests
├── tools/render/        # `render-golden` (hayro; its own workspace and Cargo.lock, outside the root one) and pdfium.py
│                        #   (pypdfium2) for `cargo xtask render-golden`; `render-diff` (laminapdf-render's hayro and PDFium
│                        #   adapters, the same parity comparison) for `cargo xtask render-diff`; `annots-parity` and
│                        #   pdfjs-render.mjs (pdf.js on @napi-rs/canvas, its own pnpm project) for `cargo xtask annots-parity`
├── tools/bench/         # the extraction benchmark (L3f): extract-datasets.toml (pinned datasets, licences, refusals), the
│                        #   lock of SHA-256 per file, olmocr_score.py (the benchmark's own tests), requirements.txt; README.md
├── tools/heron-web/     # check.mjs: Heron in onnxruntime-web on two pages (hayro) vs the native reference run; its own pnpm
│                        #   project outside the workspace (onnxruntime-web is 150 MB); run by hand (README.md)
├── tools/models/        # models.toml: the models plug-ins may use (pins, licences and their sources; `cargo xtask models`),
│                        #   heron_reference.py (the layout crate's reference run, by hand); README.md
├── tools/hn-research/   # the Hacker News research's scripts (docs/research/hacker-news/): collect stories and comments from the
│                        #   HN Search API, sort them into areas, count themes per area, check quotes; stdlib Python, run by hand,
│                        #   never by tests; the data stays out of git (README.md)
├── tools/desktop/        # cold_start.py: the desktop app's cold-start gates (ADR 0008 §4) from an e2e build's startup log,
│                        #   the app against floor/ (a page that sends the first command at once) launched in turn; by hand
├── tools/agent-eval/     # L5a: fixed tasks for a pinned coding agent, with and without the shipped docs, checked by
│                        #   @laminapdf/sdk and qpdf; run by hand with the owner's API key, never in CI (README.md)
├── tools/answer-check/   # W3i: twenty developer questions asked to Claude or OpenAI with web search, three times each; whether answers
│                        #   name, find and cite Lamina PDF (report.mjs); its own pnpm project; run by hand with the owner's API key,
│                        #   never in CI (README.md)
├── tools/ci-runner/linux/  # the Linux CI runner container on the Mac (Dockerfile, up.sh, down.sh, loop.sh)
├── .github/             # CI fast lane (workflows/ci.yml), full lane + required `gate` (workflows/full.yml),
│                        #   nightly lane (workflows/nightly.yml), release lane on v* tags (workflows/release.yml: every check, then the publish jobs),
│                        #   docs deploys (workflows/docs.yml), the CLI's release (workflows/cli-release.yml), Dependabot;
│                        #   shared setup in actions/,
│                        #   written `$/.github/actions/<name>`; every action locked by workflows/actions.lock (X11, ADR 0019:
│                        #   tags in `uses:`, `gh actions-lock` after a change, the fast lane's `lockfile` job runs --verify)
├── spikes/              # research spikes (throwaway code that informs decisions), outside both workspaces
│   ├── pdf-basics/      # minimal PDF core in Rust + TypeScript, its own frozen fixtures and spec (ported to crates/)
│   ├── krilla-fit/      # Round B1: can krilla generate into Lamina PDF's object graph? (answer + measurements in its README)
│   ├── hayro-parity/    # Round C1a: hayro vs PDFium render parity on the pdf.js and PDFBox corpora (baseline in its README)
│   ├── pdfium-wasm/     # Round C1b: PDFium-WASM beside laminapdf-wasm in one web worker (measurements in its README; ADRs 0004, 0009)
│   ├── lamina-ffi/      # Round L6a: the façade over UniFFI 0.32 for Swift and Kotlin (findings in its README; ADR 0029)
│   ├── desktop-print/   # D1b: webview, native raster and native vector printing per OS, scored against the fixtures (results in its README; D4c, ADR 0008)
│   ├── desktop-ipc/     # D1a: Tauri tile transports (channel, ipc::Response, lamina:// raw and compressed) and COOP/COEP per webview (results in its README; ADR 0008)
│   ├── auth/            # H1: Better Auth and Auth.js on a Worker with D1, Clerk's SDK on a Worker, a Rust loopback + PKCE desktop client (results in its README; ADR 0011)
│   ├── flow-layout/     # T4a: flowing layout by our own paginator over Surface, by Typst 0.15.1, and by Typst's frames through Surface (findings in its README; ADR 0032)
│   └── content-edit/    # E7a: an editable content IR and two encoders (copy unchanged spans, or write every operator again) on the external corpus (results in its README; ADR 0022)
├── .agents/skills/      # the repository's agent skills (rust-best-practices, lamina-story, session-name); .claude/skills links here
└── docs/                # all documentation (see docs/README.md)
    ├── research/        # deep dives and research notes, one subfolder per topic
    ├── plans/           # roadmaps and feature design docs (each links to its todos)
    ├── todos/           # one file per PR of work: ready/ → ongoing/ → done/ (the board is built by `cargo xtask todos`, never committed)
    ├── capabilities/    # the capability matrix: manifest.toml (one row per 06 §3 operation) + generated README.md
    ├── decisions/       # Architecture Decision Records (NNNN-title.md)
    ├── design/          # visual design: Paper design language, screen inventory, area docs, Design canvas sources
    ├── bugs/            # one folder per diagnosed bug: detail.md (report, measurements, cause) and resolution.md
    ├── guidelines/      # how we work (contributing, git workflow, code organization, Rust/TS)
    └── references/      # external specs, tools, corpora
```
