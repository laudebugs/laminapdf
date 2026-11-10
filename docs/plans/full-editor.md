# Plan: full-featured PDF editor (web and desktop)

- **Status:** Draft
- **Date:** 2026-10-28
- **Builds on:** [architecture plan](architecture.md) ("Later rounds", ADRs 0012 and 0013, principles 2–3 on ops and the save planner) and the [architecture research report](../research/architecture/app-and-package-architecture.md) (editing and Cloudflare sections)
- **Depends on:** the readers ([web-app.md](web-app.md) C1–C7, [desktop-app.md](desktop-app.md) D1–D7) and the library's model, decryption, ops, save and extraction crates ([pdf-library.md](pdf-library.md) B2–B5 and B9)
- **Research:** [06 Feature inventory](../research/pdf/06-feature-inventory.md) (§2 groups, §3 operations, §7 difficulty, §9 phasing, §10 traps), [04 Interactive and security](../research/pdf/04-interactive-security.md), [03 Text and fonts](../research/pdf/03-text-and-fonts.md), [05 §3 processing](../research/pdf/05-creating-pdfs-from-code.md#3-processing-operations-under-the-hood)
- **UI/UX:** [ui-ux.md §5.7](ui-ux.md#57-trust-surfaces-r9r13) sets the editor's UI rules (modes, explicit save, two named signing actions, redaction mark → review → apply, organiser grid); each E round registers its mode and commands
- **Roadmap:** [roadmap.md](roadmap.md)

## 1. Goal

Grow the web and desktop readers into one **Acrobat-class editor** that covers all 13 feature groups (A–M, ~180 operations) in [06 §3](../research/pdf/06-feature-inventory.md#3-full-operation-inventory-with-mechanisms), in the order of [06 §9](../research/pdf/06-feature-inventory.md#9-suggested-phasing). Every operation is a typed `Op` in the core, so each one ships to the web app, the desktop app and the `@laminapdf/sdk` / `lamina` packages in the same PR ([architecture §2](architecture.md#2-the-architecture-in-one-picture)). Documents stay on the device; server features are optional and opt-in.

## 2. Scope and non-goals

**In scope:** every operation in 06 §3, each with a target status in the capability matrix (§3). Some reach only `read` (for example 3D, XFA) by design.

**Non-goals**

- XFA authoring or dynamic XFA rendering. XFA is detected and reported (`unsupported` with a warning); static XFA falls back to its AcroForm if present.
- Executing arbitrary PDF JavaScript. A limited, sandboxed subset for form formatting, validation and calculation is an open question (§9).
- A browser-grade HTML/CSS engine. HTML → PDF uses Cloudflare Browser Run or a headless browser as an external converter.
- Real-time co-editing of content streams. Collaboration covers annotations, form values and page order only ([report, editing section](../research/architecture/app-and-package-architecture.md)).

## 3. Tracking: the capability matrix

The capability matrix is the editor's roadmap and progress report ([06 §10](../research/pdf/06-feature-inventory.md#10-implications-for-lamina), [architecture §7 cross-cutting](architecture.md#7-to-dos)).

- **Rows:** one per operation in 06 §3, with a stable ID `<group>.<nn>` in the order the operations appear there (for example `C.04` = reorder pages).
- **Columns:** `core` (Rust), `pdf` (`@laminapdf/sdk`), `web`, `desktop`, `target round`.
- **Statuses:**
  - `unsupported`: detected and reported to the user, never silently dropped.
  - `read`: displayed or extracted faithfully.
  - `write`: can be created or changed; output passes qpdf `--check` and opens correctly in pdf.js and PDFium.
  - `round-trip`: open → change → save → reopen in lamina, pdf.js and PDFium preserves the feature and everything else in the file (golden and differential tests).
- **Mechanics** (built in [X2](../todos/done/X2-capability-matrix.md)): the manifest [`docs/capabilities/manifest.toml`](../capabilities/manifest.toml) (169 rows: ID, name as in 06 §3, target round, target status, targeted surfaces; IDs are never reused, removed ones are `retired`) and a doc-comment tag on each proving Rust test, `/// capability: C.08 core round-trip` (TS tests: `test("[cap C.04 pdf round-trip] …")`, collected once a package exists); `cargo xtask capabilities` runs the tagged tests and regenerates [`docs/capabilities/README.md`](../capabilities/README.md). The full lane's `cargo xtask capabilities --check` fails when the page differs from what the passing tests prove, and `cargo xtask check docs` (every PR, no test run) fails when the manifest and 06 §3 disagree or the page differs from what the manifest and the tags generate. Core `write` and `round-trip` can be claimed only by tests in the round-trip suite (`laminapdf-save/tests/roundtrip_suite.rs`), whose outputs `cargo xtask roundtrip` also checks with qpdf, pdf.js and PDFium.
- **Done means:** a round is done when every row it targets reaches its target status in every column it lists.

## 4. Users and use cases

| User | Typical jobs | Rounds that serve them |
| --- | --- | --- |
| Office workers | Merge, reorder, rotate, comment, fill and sign | E2, E4–E5, E8, E10 |
| Reviewers and editors | Comment threads, status, compare versions, export comments | E5, E14, E18 |
| Legal, HR, public sector | Redact, sanitise, encrypt, certify, accessible output | E10–E12, E15 |
| Designers and print shops | Preflight, colour, bleed, imposition, PDF/X | E16 |
| Developers | The same operations as an API in TS and Rust; batch processing | every round, E14 |

## 5. Requirements that hold for every round

1. **Every edit is an `Op`** with an inverse; undo/redo works across all of them, scoped to local changes.
2. **The save planner decides how to save** (ADR 0007). The UI shows the document's `SaveConstraint`:
   - `Free`: incremental by default, full rewrite on "Save as optimised".
   - `AppendOnly` (signed): only incremental saves; ops that DocMDP/FieldMDP would flag are blocked or require confirmation that signatures will be invalidated.
   - `RequiresRewrite` (applied redactions, decryption, sanitising): full rewrite, every old revision dropped. Order: redact → full rewrite → sign → append-only saves.
3. **Appearance streams are always generated** for anything lamina creates or changes (annotations, fields, stamps), so other viewers show the same thing ([04 §2.4](../research/pdf/04-interactive-security.md)).
4. **Correctness traps are tested**, not documented away: redaction that only covers content, crop that only hides content, incremental saves that keep deleted content, flattening without appearances, edits that break PDF/A or PDF/UA ([06 §10](../research/pdf/06-feature-inventory.md#10-implications-for-lamina)). The UI warns when an edit breaks a claimed conformance level.
5. **Same features on web and desktop** through `Platform` capability flags. Differences are listed per round.
6. **Privacy:** server features are off by default, opt-in per document, and say what leaves the device (ADR 0011).
7. **Library parity:** each op is exposed in `lamina` and `@laminapdf/sdk` in the same PR, with docs and an example.

## 6. Design summary

The editing architecture (lazy COS overlay, typed model, op log, save planner, overlays for annotations and fields, reload of touched pages after content edits) is in [architecture.md](architecture.md) and the [report](../research/architecture/app-and-package-architecture.md). This plan adds three things:

- **Interaction modes** (select, hand, text-select, annotate tools, form-fill, form-design, content-edit, redact, measure) as one state machine in `@laminapdf/ui`, with each tool a small module over `@laminapdf/viewer` layers.
- **A content-stream layer** in the crate `laminapdf-content` ([05 §8.5](../research/pdf/05-creating-pdfs-from-code.md#85-proposed-architecture-for-discussion)), grown in three steps:
  1. **Read** ([library B9](pdf-library.md#b9-text-extraction), ADR 0023): a read-only interpreter used for text extraction, created before the readers' text layer.
  2. **Append** (E2): a minimal write API that never parses existing operators. It wraps a page's existing content in `q … Q` by adding new streams to the page's `/Contents` array, appends new streams (for example `q <cm> /Fm1 Do Q`), and registers Form XObjects under unused resource names. A token scan (B9's lexer) checks that existing streams balance `q`/`Q` and adds closing `Q`s when they don't. Resize (E2), flattening (E5) and adding content (E6) use it.
  3. **Edit** (E7, ADR 0022): IR with object identity and an encoder, for editing and removing existing content (E7, E12).
- **New ADRs** (numbers proposed; next free after 0019): 0020 signatures and crypto backends, 0021 PDF JavaScript policy, 0022 content-stream editing model (editing existing content). The library plan proposes 0023 (content-stream reading). ADR 0012 (collaboration) and 0013 (heavy jobs) come from the architecture plan.

### Optional Cloudflare server features

| Feature | Products | ADR | Round |
| --- | --- | --- | --- |
| Shared review (annotations, field values, page order) | Durable Object per document, R2 for immutable base PDFs, D1 index | 0012 | E18 |
| OCR, Office ↔ PDF, PDF → Word/Excel | Workflows + Containers; blobs in R2 with lifecycle expiry | 0013 | E13 |
| HTML → PDF | Browser Run, then lamina post-processing (tags, PDF/A) | 0013 | E13 |
| Batch jobs (optimise, convert, stamp many files) | Workflows + Queues, Worker running `laminapdf-wasm` for small files, Containers for large | 0013 | E14, E16 |
| Timestamp authority proxy (TSAs rarely allow CORS) | Worker, no document bytes (hash only) | 0020 | E10 |
| Accounts and user login (hosted service, decided 2026-10-28) | Auth on D1 or a hosted vendor (`auth` spike) | 0011 | first round that needs it ([H1](../todos/ready/H1-auth-spike.md) spike, [H2](../todos/ready/H2-accounts-and-login.md)) |
| Cloud storage and sync of the user's documents (opt-in per document) | R2 for blobs, D1 index, per-user access | 0011 | first round that needs it ([H3](../todos/ready/H3-cloud-document-storage.md), [H4](../todos/ready/H4-document-sync.md)) |

## 7. Phases and rounds

Phase 1 (core viewer: open, render, zoom, search, text selection, thumbnails, download) is delivered by the reader rounds [C1–C7](web-app.md#6-rounds) and [D1–D7](desktop-app.md#6-rounds). Editor rounds start at E1 once the web reader has launched (C7c); they don't wait for the desktop beta (D7c). The desktop rounds go on alongside. An editor round's desktop part is its own todo that depends on the desktop todo it needs, such as D3b for file watching or D4b for native menus. The web and shared work therefore never waits on the desktop track ([roadmap: decisions made](roadmap.md#decisions-made), 2026-11-10).

Editor rounds are larger than reader rounds. Each starts with a short **feature-plan PR** (scope, op list, tests, any ADR) and then lands as a stack of implementation PRs, each mergeable on its own ([git workflow §5](../guidelines/git-workflow.md#5-pull-requests); [architecture §5](architecture.md#5-rounds): "each feature group gets its own plan and pull requests").

**Todos.** Each feature-plan PR and implementation PR is one todo file on the [todo board](../todos/README.md) (`ready/` → `ongoing/` → `done/`), with explicit dependencies. Rounds that introduce an ADR (E7, E8, E10, E13, E18) start with an ADR todo (`a`), then the feature plan (`b`); the others start with the feature plan (`a`). The hosted-service foundation (accounts, cloud storage, sync) is todos H1–H4. Links below point at `ready/` and are updated as todos move.

### Phase 2: page organisation, navigation and metadata

#### E1. Editing foundation

- **Ships:** edit session, dirty state, undo/redo UI and history panel, Save / Save as / Download copy, `SaveConstraint` banner, capability matrix page in the app. First user-visible write ops from group J: title, author, subject, keywords (J.01), creation/modification dates (J.02), producer and creator (J.03), remove selected metadata (J.10). Every metadata op writes the Info dictionary and the XMP packet together ([04 §5](../research/pdf/04-interactive-security.md#5-metadata): `dc:`, `pdf:` and `xmp:` properties, plus `xmp:MetadataDate` and `xmpMM:InstanceID` on save); a file without XMP gets a new packet. Files that claim PDF/A (`pdfaid:` schema) keep their identification schema and the edit is checked against the claim, with a warning before saving if it would break it (§5 requirement 4). For PDF 2.0 files XMP is authoritative and Info keeps only what 2.0 still allows. The general XMP editor (custom and extension schemas, J.04) stays in E3.
- **Crates:** `laminapdf-ops`, `laminapdf-save`, `laminapdf-model` (XMP packet parse and update for the Info-mapped properties, sharing the writer from library B5).
- **Save:** `Free` → incremental by default. Signed files are `AppendOnly`; metadata edits are allowed only as incremental updates.
- **Web vs desktop:** desktop saves in place and watches for external changes; web saves in place only where the File System Access API exists, otherwise downloads a copy.
- **Todos:** [E1a](../todos/ready/E1a-editing-foundation-feature-plan.md) feature plan · [E1b](../todos/ready/E1b-xmp-packet-model.md) XMP packet parse and update for Info-mapped properties · [E1c](../todos/ready/E1c-metadata-ops.md) Metadata ops: Info and XMP in sync (J.01–J.03, J.10) · [E1d](../todos/ready/E1d-edit-session-undo-ui.md) Edit session, interaction modes, undo/redo UI and history panel · [E1e](../todos/ready/E1e-save-and-constraint-banner.md) Save, Save as, Download copy and the SaveConstraint banner · [E1f](../todos/ready/E1f-metadata-ui-capability-page.md) Metadata editor UI and in-app capability-matrix page.

#### E2. Page organisation

- **Ships (group C):** insert blank pages, insert pages from other PDFs, delete and duplicate, reorder (drag in thumbnails), extract selected pages, merge documents, split by count/ranges/bookmarks, rotate, crop display area (with a warning that cropped content remains), resize page, margins and page boxes, portrait ↔ landscape, reverse order, page labels, replace pages. From H: export pages/ranges.
- **Crates:** `laminapdf-content` append API (introduced here, see §6): resizing that scales content wraps the existing streams in `q <scale cm> … Q` without parsing them; `laminapdf-ops` page ops (view-level mapping of `LaminaPdfId`s, no re-parse); merge remaps outlines, named destinations, AcroForm fields and the structure tree ([05 §3.1](../research/pdf/05-creating-pdfs-from-code.md#31-merge-and-split)).
- **Save:** `Free` → incremental. On signed files, page insertion/deletion is blocked by DocMDP P1/P2 and allowed with warning only when it invalidates signatures.
- **Web vs desktop:** identical; desktop allows multi-file merge from Finder/Explorer selection.
- **Todos:** [E2a](../todos/ready/E2a-page-organisation-feature-plan.md) feature plan · [E2b](../todos/ready/E2b-page-structure-ops.md) Page-structure ops: insert blank, delete, duplicate, reorder, rotate, reverse · [E2c](../todos/ready/E2c-cross-document-pages.md) Insert pages from other PDFs, replace pages, extract and export ranges · [E2d](../todos/ready/E2d-merge-and-split.md) Merge documents and split by count, ranges and bookmarks · [E2e](../todos/ready/E2e-content-append-api.md) `laminapdf-content` append API · [E2f](../todos/ready/E2f-page-geometry.md) Crop, resize, page boxes and orientation · [E2g](../todos/ready/E2g-page-labels.md) Page labels editor · [E2h](../todos/ready/E2h-page-organiser-ui.md) Page organiser UI: thumbnails drag-and-drop, multi-select and page dialogs · [E2i](../todos/ready/E2i-signed-gating-and-close.md) DocMDP gating tests on signed fixtures and E2 round close.

#### E3. Navigation, attachments, inspection and extraction

- **Ships:** group I: create/edit nested bookmarks, internal page links, external hyperlinks, named destinations, clickable table of contents (generated from bookmarks or headings), initial view and page display mode, article threads (read; edit later). Group J: general XMP editing (J.04: raw packet and custom/extension schemas; the Info-mapped properties are already kept in sync since E1), attach/extract/delete embedded files, portfolios (read), inspect version and conformance claims, identify fonts and colour profiles, object inspector (internal objects, streams, references). Group A: add hyperlinks to text. Group B: extract original embedded images. Group H: images → PDF, pages → PNG/JPEG/TIFF, render pages to images, extract text and images, extract hyperlinks and bookmarks, extract attachments. Group M: associated files (`/AF`, PDF 2.0 / PDF/A-3).
- **Crates:** `laminapdf-model` (outline, names, embedded files, XMP extension schemas), `laminapdf-build` (images → PDF), `laminapdf-render` (export), `laminapdf-text` (extraction from library B9).
- **Save:** `Free`; adding attachments to signed files only as incremental updates where permitted.
- **Web vs desktop:** object inspector ships on both; desktop exports many pages to a folder, web zips them.
- **Todos:** [E3a](../todos/ready/E3a-navigation-attachments-inspection-feature-plan.md) feature plan · [E3b](../todos/ready/E3b-bookmarks-and-destinations.md) Bookmark editor, named destinations, TOC generator · [E3c](../todos/ready/E3c-links.md) Internal links, external hyperlinks and links on text · [E3d](../todos/ready/E3d-initial-view-and-threads.md) Initial view, page display mode and article threads (read) · [E3e](../todos/ready/E3e-xmp-editor.md) General XMP editor (custom and extension schemas) · [E3f](../todos/ready/E3f-attachments-and-associated-files.md) Attachments panel, associated files (`/AF`) and portfolios (read) · [E3g](../todos/ready/E3g-inspectors.md) Version, conformance, fonts, colour profiles and object inspector · [E3h](../todos/ready/E3h-images-to-pdf-and-page-export.md) Images → PDF and page export to PNG/JPEG/TIFF · [E3i](../todos/ready/E3i-extraction.md) Extract text, images, links and bookmarks.

### Phase 3: annotations

#### E4. Markup and drawing annotations

- **Ships (group D):** highlight/underline/strike-through, sticky notes and text comments, text-correction (caret) annotations, freehand ink, shapes/lines/arrows, callouts and text boxes, appearance-stream generation (cross-cutting). Group B "arrows, paths, freehand drawing" as annotations.
- **Crates:** `laminapdf-model` annotations with `/NM` as `LaminaPdfId`; appearance generator in `laminapdf-build`; overlay rendering from lamina's model until save.
- **Save:** incremental. Signed files: annotations allowed only when DocMDP P3 or no certification signature; otherwise warn.
- **Web vs desktop:** identical; pen/stylus pressure for ink where the platform reports it.
- **Todos:** [E4a](../todos/ready/E4a-markup-and-drawing-annotations-feature-plan.md) feature plan · [E4b](../todos/ready/E4b-annotation-model-and-overlay.md) Annotation model with `/NM` IDs, annotation ops and overlay rendering · [E4c](../todos/ready/E4c-appearances-markup-notes-caret.md) Appearances and creation: text markup, notes and caret · [E4d](../todos/ready/E4d-appearances-ink-and-shapes.md) Appearances and creation: ink, shapes, lines and arrows · [E4e](../todos/ready/E4e-freetext-callouts.md) FreeText text boxes and callouts with `/DA` and rich text · [E4f](../todos/ready/E4f-annotation-tools-ui.md) Annotation tools and properties panel · [E4g](../todos/ready/E4g-appearance-parity-suite.md) Cross-viewer appearance parity suite and E4 round close.

#### E5. Review, stamps and interchange

- **Ships (group D):** custom stamps (image and dynamic text), attach files as annotations, replies, status and review state (`/IRT`, `/StateModel`), filter and search comments, import/export XFDF (and FDF), flatten annotations into content, delete or summarise annotations (comment summary as a new PDF). Audio annotations: `read` (listed, playable on desktop) only.
- **Crates:** as E4, plus XFDF reader/writer; flattening needs E4 appearances and uses the `laminapdf-content` append API from E2 (each appearance becomes a Form XObject drawn with an appended `q <cm> /FmN Do Q` stream; the annotation is then removed).
- **Save:** flattening is a content change → incremental on `Free`; on signed files it invalidates signatures and is gated.
- **Web vs desktop:** identical. Shared review across users is E18.
- **Todos:** [E5a](../todos/ready/E5a-review-stamps-interchange-feature-plan.md) feature plan · [E5b](../todos/ready/E5b-comment-threads-and-review.md) Comment panel: replies, review state, filter and search · [E5c](../todos/ready/E5c-stamps-and-attachments.md) Custom stamps, file-attachment annotations and audio annotations (read) · [E5d](../todos/ready/E5d-xfdf-fdf.md) XFDF and FDF import/export of annotations · [E5e](../todos/ready/E5e-flatten-and-summary.md) Flatten annotations into content and comment summary.

### Phase 4: content editing

#### E6. Adding content to pages

- **Ships:** group A: add text anywhere, insert special characters and symbols. Group B: insert images, resize/rotate/crop/reposition **new** images and shapes, opacity, draw lines/rectangles/circles/polygons as page content, add logos/icons/stamps as content, background images and watermarks. Group C: headers/footers and page numbers (Bates numbering as an option).
- **Crates:** `laminapdf-content` append API from E2 (new content as Form XObjects drawn by appended streams), `laminapdf-build` surfaces reused on existing pages, `laminapdf-text` for new text.
- **Save:** incremental on `Free`; on signed files these are content changes and invalidate signatures (gated).
- **Rendering:** touched pages reload in PDFium after an incremental update (the "two parsers" path in [architecture §6](architecture.md#6-risks)); measure and budget reload time.
- **Web vs desktop:** desktop can use system fonts (font discovery at the edge, passed into the core as bytes); web uses bundled or user-supplied fonts.
- **Todos:** [E6a](../todos/ready/E6a-adding-content-feature-plan.md) feature plan · [E6b](../todos/ready/E6b-place-images-and-shapes.md) Place images, shapes, logos and stamps as page content · [E6c](../todos/ready/E6c-add-text.md) Add text and special characters as page content · [E6d](../todos/ready/E6d-watermarks-headers-footers.md) Watermarks, backgrounds, headers/footers, page numbers and Bates numbering · [E6e](../todos/ready/E6e-content-tools-ui.md) Content insert tools and object handles · [E6f](../todos/ready/E6f-reload-budget.md) Touched-page reload time: measure and budget.

#### E7. Editing existing content

- **Ships:** group A: edit existing text, delete or replace text, change font family/size/colour/style, bold/italic/underline/super/subscript, alignment and line spacing, move/resize/rotate text blocks, search and replace across the document. Group B: delete/replace existing images, resize/rotate/crop/reposition existing objects, edit vector paths/fills/strokes, change stacking order, clipping paths and masks, transformations.
- **Crates:** ADR 0022 (content-stream editing model) and the `laminapdf-content` IR with object identity and encoder, built on B9's interpreter; text-run detection via MCIDs where tagged, heuristics otherwise; re-layout with the original subset when it has the glyphs, a new subset otherwise ([03 §11.2 "Editing"](../research/pdf/03-text-and-fonts.md#112-concrete-requirements)). This is the hardest round ([06 §7](../research/pdf/06-feature-inventory.md#7-difficulty-map)); "edit in place" is best-effort and the UI says so.
- **Save:** incremental on `Free`; structure tree and MCIDs updated so tagging survives; PDF/A/UA claims re-checked on save.
- **Web vs desktop:** identical; desktop font substitution can use system fonts.
- **Todos:** [E7a](../todos/ready/E7a-adr-0022-content-editing.md) ADR 0022: content-stream editing model for existing content · [E7b](../todos/ready/E7b-editing-existing-content-feature-plan.md) feature plan · [E7c](../todos/ready/E7c-content-ir.md) Content IR with object identity and encoder · [E7d](../todos/ready/E7d-select-transform-existing.md) Select, move, resize, rotate, delete and replace existing objects; stacking order · [E7e](../todos/ready/E7e-text-runs-and-inline-edit.md) Text-run detection and in-place text editing with the existing subset · [E7f](../todos/ready/E7f-font-handling-and-styling.md) Font handling: re-subset, substitution, font/size/colour/style, alignment and spacing · [E7g](../todos/ready/E7g-search-and-replace.md) Search and replace across the document · [E7h](../todos/ready/E7h-vector-clipping-masks.md) Vector path editing, fills, strokes, clipping paths and masks · [E7i](../todos/ready/E7i-structure-and-conformance-maintenance.md) Struct-tree and MCID maintenance; PDF/A and PDF/UA re-checks on save · [E7j](../todos/ready/E7j-edit-corpus.md) Edit-corpus tests and E7 round close.

### Phase 5: forms and signatures

#### E8. Form filling and data

- **Ships (group E):** fill text fields (single/multi-line), checkboxes and radio buttons, dropdowns and list boxes, push buttons (non-JS actions), date/formatted input, required and read-only, tab order (follow `/Tabs`), reset form, import/export form data (FDF, XFDF, JSON), populate fields programmatically (library), flatten forms. Input validation, automatic calculations and conditional behaviour: declarative cases (Hide actions, `/CO` order) first; JavaScript-driven ones depend on ADR 0021. Submit to external endpoints: off by default, confirm per submit. XFA: detect and report.
- **Crates:** `laminapdf-model` AcroForm field tree; appearance generation for field values ([04 §3.5](../research/pdf/04-interactive-security.md)); widgets as DOM overlays in `@laminapdf/viewer`.
- **Save:** incremental. Signed files: field filling allowed when DocMDP P2/P3 permit, blocked for locked fields.
- **Web vs desktop:** identical.
- **Todos:** [E8a](../todos/ready/E8a-adr-0021-javascript-policy.md) ADR 0021: PDF JavaScript policy · [E8b](../todos/ready/E8b-form-filling-feature-plan.md) feature plan · [E8c](../todos/ready/E8c-acroform-field-model.md) AcroForm field model, value ops, flags and XFA detection · [E8d](../todos/ready/E8d-field-appearances.md) Value appearance generation for every field type · [E8e](../todos/ready/E8e-field-widgets-ui.md) Field widgets as DOM overlays, tab order, required/read-only, push buttons and submit · [E8f](../todos/ready/E8f-declarative-behaviour.md) Formatting, validation, calculation and conditional behaviour (declarative and per ADR 0021) · [E8g](../todos/ready/E8g-form-data-and-flatten.md) Form data import/export (FDF, XFDF, JSON), reset, programmatic fill and flatten.

#### E9. Form authoring

- **Ships (group E):** create, move, resize and configure all field types, signature fields, validation and format settings, calculation order, tab order editor, accessible field labels (`/TU`, overlaps L), automatic field detection on flat forms (heuristic, optional).
- **Crates:** as E8, plus form-design mode in `@laminapdf/ui`.
- **Save:** incremental on `Free`; authoring on a signed file is blocked unless signatures are removed.
- **Web vs desktop:** identical.
- **Todos:** [E9a](../todos/ready/E9a-form-authoring-feature-plan.md) feature plan · [E9b](../todos/ready/E9b-field-creation-ops.md) Field creation and configuration ops, including signature fields · [E9c](../todos/ready/E9c-form-design-mode.md) Form-design mode: inspector, validation/format/calculation settings, tab-order editor, accessible labels · [E9d](../todos/ready/E9d-automatic-field-detection.md) Automatic field detection on flat forms (optional).

#### E10. Signatures

- **Ships (group F):** handwritten signature image, draw/type e-signature (visual, non-cryptographic, clearly labelled), certificate-based digital signatures (PAdES B-B, B-T), verify signatures (integrity, chain, revocation where reachable), certify (DocMDP), trusted timestamps, inspect signer identity and certificates, lock fields after signing, allowed changes after certification, signature status and modification warnings (revision diff analysis), signing via external services (two-phase: prepare → external sign → inject).
- **Crates:** ADR 0020; signature module over `laminapdf-save` (ByteRange placeholder, incremental append), RustCrypto + `cms`; validation diff between revisions.
- **Save:** signing makes the document `AppendOnly`; signing always runs after any pending rewrite (redact → rewrite → sign).
- **Web vs desktop:** web signs with WebCrypto keys or an imported PKCS#12 kept on the device; desktop adds OS keychains and PKCS#11 smart cards/tokens. Timestamping on web goes through the optional TSA proxy (sends a hash only).
- **Todos:** [E10a](../todos/ready/E10a-adr-0020-signatures.md) ADR 0020: signatures, crypto backends and key storage per platform · [E10b](../todos/ready/E10b-signatures-feature-plan.md) feature plan · [E10c](../todos/ready/E10c-visual-signatures.md) Visual signatures: image, drawn and typed (non-cryptographic) · [E10d](../todos/ready/E10d-pades-signing-core.md) PAdES B-B signing core with two-phase external signing · [E10e](../todos/ready/E10e-timestamps-and-tsa-proxy.md) Trusted timestamps (B-T, document timestamps) and the optional TSA proxy Worker · [E10f](../todos/ready/E10f-certification-and-locking.md) Certification (DocMDP), field locking (FieldMDP) and allowed changes · [E10g](../todos/ready/E10g-signature-validation.md) Signature validation: integrity, chain, revocation, signer inspection · [E10h](../todos/ready/E10h-revision-diff-and-attacks.md) Revision diff analysis, modification warnings and attack-catalogue tests · [E10i](../todos/ready/E10i-platform-signers.md) Platform key storage: OS keychains and PKCS#11 (desktop), WebCrypto and PKCS#12 (web) · [E10j](../todos/ready/E10j-signature-ui.md) Signing flow and signature panel UI.

### Phase 6: advanced workflows

#### E11. Security and sanitisation

- **Ships (group G):** document-open password, encrypt (AES-256 `/V 5 /R 6` for writing; reading every standard revision already works since [library B2](pdf-library.md#b2-document-model)), permission passwords, restrict print/copy/edit, remove security with authorisation, remove hidden metadata and comments, sanitise embedded content, remove attachments and scripts, inspect encryption, certificate-based (public-key) security (read first, write later), remove hidden layers and information.
- **Crates:** `laminapdf-save` encrypt on write (over the B2 crypt filters in `laminapdf-syntax`); public-key handler read; sanitiser as a set of ops with a report.
- **Save:** `RequiresRewrite`. On signed files, warn that signatures will be invalidated.
- **Web vs desktop:** identical; keys never leave the device.
- **Todos:** [E11a](../todos/ready/E11a-security-and-sanitisation-feature-plan.md) feature plan · [E11b](../todos/ready/E11b-encrypt-on-write.md) AES-256 encryption on write, passwords and permissions; inspect encryption · [E11c](../todos/ready/E11c-remove-security.md) Remove security with authorisation and change passwords · [E11d](../todos/ready/E11d-sanitiser.md) Sanitiser ops with a report · [E11e](../todos/ready/E11e-public-key-security.md) Public-key (certificate) security: read, write behind a flag.

#### E12. Redaction

- **Ships (group G):** permanent redaction (mark with Redact annotations, review, apply), search sensitive content to redact (text, regex, patterns such as emails and card numbers, plus OCR text from E13 when present).
- **Crates:** `laminapdf-content` removes glyphs, image pixels and vector paths under redaction areas; removes matching text from annotations, form values, metadata, bookmarks and the structure tree.
- **Save:** `RequiresRewrite`, no prior revisions kept. Tests prove that redacted text cannot be extracted with pdf.js, PDFium, pypdf and pdftotext, and that no previous revision survives.
- **Web vs desktop:** identical.
- **Todos:** [E12a](../todos/ready/E12a-redaction-feature-plan.md) feature plan · [E12b](../todos/ready/E12b-mark-and-review.md) Mark and review: Redact annotations and preview · [E12c](../todos/ready/E12c-apply-content-removal.md) Apply redactions: remove glyphs, paths and image pixels · [E12d](../todos/ready/E12d-scrub-and-rewrite.md) Scrub non-content layers, full rewrite and verification report · [E12e](../todos/ready/E12e-sensitive-search.md) Search sensitive content to redact · [E12f](../todos/ready/E12f-leak-tests.md) Redaction leak tests across pdf.js, PDFium, pypdf and pdftotext; round close.

#### E13. OCR and conversion

- **Ships:** group H: scanned → searchable PDF (invisible text layer), Word/Excel/PowerPoint → PDF, HTML/webpages → PDF, PDF → Word, PDF tables → Excel. Group A: convert outlined or scanned text via OCR.
- **Crates:** OCR text layer emission in `laminapdf-build` (text render mode 3, from hOCR/ALTO); converters are external engines.
- **Server:** ADR 0013. OCR and Office conversion in Containers via Workflows; HTML → PDF via Browser Run; lamina post-processes output (tags, PDF/A). Engine licences checked in the ADR (no AGPL in shipped artefacts).
- **Web vs desktop:** web offers client-side OCR for small documents (WASM OCR engine, if its licence and size fit) and opt-in server OCR; desktop runs OCR locally with an optional engine download. Office conversion is server-only on web and optional local on desktop.
- **Todos:** [E13a](../todos/ready/E13a-adr-0013-heavy-jobs.md) ADR 0013: heavy jobs on Workflows + Containers; OCR and conversion engines and licences · [E13b](../todos/ready/E13b-ocr-and-conversion-feature-plan.md) feature plan · [E13c](../todos/ready/E13c-ocr-text-layer.md) Invisible OCR text layer from hOCR/ALTO · [E13d](../todos/ready/E13d-local-ocr.md) Client-side OCR on web and local OCR on desktop · [E13e](../todos/ready/E13e-server-jobs-infrastructure.md) Opt-in server jobs: Workflows + Containers with R2 lifecycle (OCR and Office conversion) · [E13f](../todos/ready/E13f-office-and-html-to-pdf.md) Office → PDF and HTML → PDF (Browser Run) with lamina post-processing · [E13g](../todos/ready/E13g-pdf-to-word-excel.md) PDF → Word and PDF tables → Excel (server).

#### E14. Compare, revisions and batch

- **Ships (group M):** inspect incremental updates and revisions (open any prior revision read-only), compare versions visually (pixel diff) and structurally (text, objects, annotations). Batch processing (06 §9 phase 6): apply a saved sequence of ops to many files.
- **Crates:** `laminapdf-save` revision index, `laminapdf-render` for visual diff, `laminapdf-text` for text diff.
- **Server:** optional batch via Workflows (ADR 0013).
- **Web vs desktop:** desktop batches folders locally; web batches selected files locally and offers opt-in server batch.
- **Todos:** [E14a](../todos/ready/E14a-compare-revisions-batch-feature-plan.md) feature plan · [E14b](../todos/ready/E14b-revision-browser.md) Revision index and revision browser · [E14c](../todos/ready/E14c-compare.md) Visual and structural compare · [E14d](../todos/ready/E14d-batch-runner.md) Batch runner over saved op sequences (local) · [E14e](../todos/ready/E14e-server-batch.md) Optional server batch via Workflows.

#### E15. Accessibility

- **Ships (group L):** add structure tags (headings, paragraphs, lists, tables), logical reading order, alt text for images, document language, mark decorative artifacts, accessible form labels, accessible link descriptions, check missing fonts / Unicode mappings, run accessibility validation (Matterhorn machine checks), fix tagging and structure issues, PDF/UA-compliant output. Group I: define reading order and logical structure tags. Group H: convert to PDF/UA.
- **Crates:** structure-tree editing in `laminapdf-model`; checks shared with the library's B8 profiles. veraPDF stays a CI tool, or runs as an external process in an optional Container (GPL, never linked).
- **Save:** incremental on `Free`; tagging a signed file invalidates signatures unless done as allowed incremental changes.
- **Web vs desktop:** identical; tag tree panel and reading-order overlay on both.
- **Todos:** [E15a](../todos/ready/E15a-accessibility-feature-plan.md) feature plan · [E15b](../todos/ready/E15b-structure-tree-editing.md) Structure-tree editing model, tag tree panel and reading-order overlay · [E15c](../todos/ready/E15c-tagging-tools.md) Tagging tools: headings, lists, tables, artifacts, alt text, language, form and link labels · [E15d](../todos/ready/E15d-accessibility-checks.md) Matterhorn machine checks, missing fonts/Unicode checks and fix-ups · [E15e](../todos/ready/E15e-pdfua-output.md) PDF/UA output and veraPDF agreement in CI; round close.

#### E16. Optimisation and print production

- **Ships (group K):** compress, downsample and recompress images, embed or subset fonts, remove unused objects, deduplicate resources, linearise, flatten transparency, convert colour spaces, ICC profiles and output intents, bleed/trim/crop boxes, print-ready preflight, validate PDF/A or PDF/X, imposition and booklet, spot colours and overprint preview. Group C: booklet / N-up layouts. Group H: convert to PDF/A and PDF/X.
- **Crates:** `laminapdf-save` (object streams, linearisation, garbage collection), image codecs, colour management module; preflight rules as data.
- **Save:** `RequiresRewrite` for optimisation; signed files warned.
- **Server:** optional batch optimisation (ADR 0013).
- **Web vs desktop:** desktop recommended for large print files (memory); web enforces size limits from `Limits`.
- **Todos:** [E16a](../todos/ready/E16a-optimisation-print-feature-plan.md) feature plan · [E16b](../todos/ready/E16b-structural-optimiser.md) Optimiser: unused objects, dedupe, object streams, linearisation · [E16c](../todos/ready/E16c-images-and-fonts.md) Downsample and recompress images; embed or subset fonts · [E16d](../todos/ready/E16d-colour.md) Colour conversion, ICC and output intents, spot colours and overprint preview, page boxes for print · [E16e](../todos/ready/E16e-preflight-and-conversion.md) Preflight rules as data; PDF/A and PDF/X conversion and validation; transparency flattening · [E16f](../todos/ready/E16f-imposition.md) Imposition, booklet and N-up · [E16g](../todos/ready/E16g-server-batch-optimise.md) Optional server batch optimisation.

#### E17. Advanced interactions

- **Ships (group M):** add/edit JavaScript actions (authoring only; execution per ADR 0021), interactive buttons, event-triggered actions, manage/show/hide layers (OCGs; visibility toggles also in the reader), embed audio and video (Screen/Rendition and RichMedia, playback on desktop first), rich-media annotations, 3D models (`read`: detect and show a poster; interactive 3D out of scope for now), measurement tools and scales, geospatial coordinates (`read`, measure on maps), document open actions. Group D: audio annotations playback.
- **Crates:** `laminapdf-model` actions, OCGs, measurement dictionaries.
- **Save:** incremental on `Free`.
- **Web vs desktop:** media playback depends on codecs per platform; JavaScript never runs outside the ADR 0021 sandbox.
- **Todos:** [E17a](../todos/ready/E17a-advanced-interactions-feature-plan.md) feature plan · [E17b](../todos/ready/E17b-action-editor.md) Action editor: JavaScript authoring, buttons, event-triggered and open actions · [E17c](../todos/ready/E17c-layers.md) Layers panel: show/hide and edit optional content groups · [E17d](../todos/ready/E17d-media.md) Media: Screen/Rendition, RichMedia and audio annotation playback · [E17e](../todos/ready/E17e-measurement.md) Measurement tools and scales; geospatial read · [E17f](../todos/ready/E17f-3d-detection.md) 3D detection with poster fallback; round close.

#### E18. Collaboration (hosted service)

- **Ships:** shared review of annotations, form values and page order; presence; comment notifications (optional email). Content edits and redactions take a lease or run as a server job that mints a new base revision.
- **Server:** ADR 0012: one Durable Object per document (SQLite storage, WebSocket hibernation), LWW per (record, property), deletes win, fractional indices for page order; base PDFs immutable and content-addressed in R2; PDF bytes never cross the socket; accounts and sharing on D1 (`auth` spike).
- **Web vs desktop:** identical; desktop works offline and syncs on reconnect.
- **Todos:** [E18a](../todos/ready/E18a-adr-0012-collaboration.md) ADR 0012: collaboration — Durable Object per document, LWW records, leases · [E18b](../todos/ready/E18b-collaboration-feature-plan.md) feature plan · [E18c](../todos/ready/E18c-layer-document-schema.md) Layer-document schema and record ↔ op mapping · [E18d](../todos/ready/E18d-durable-object-server.md) Collaboration server: Durable Object per document, R2 base revisions, D1 index · [E18e](../todos/ready/E18e-client-sync-and-presence.md) Client sync, presence and offline sync on desktop · [E18f](../todos/ready/E18f-sharing-and-notifications.md) Sharing, roles and comment notifications · [E18g](../todos/ready/E18g-leases-and-base-revisions.md) Leases for content edits and redactions; server jobs minting new base revisions · [E18h](../todos/ready/E18h-load-test.md) Load test: 50 concurrent reviewers on one document; round close.

### Feature-group coverage

| Group | Rounds |
| --- | --- |
| A Text and typography | B9/C5 (extract, read), E3 (links), E6 (add), E7 (edit), E13 (OCR) |
| B Images and vector graphics | E3 (extract), E4 (as annotations), E6 (add), E7 (edit existing) |
| C Page organisation and geometry | E2, E6 (headers/footers), E16 (booklet/N-up) |
| D Comments and annotations | C6 (display), E4, E5, E17 (audio) |
| E Interactive forms | C6 (display), E8, E9 |
| F Signatures and approval | E10 |
| G Security and privacy | B2 (decrypt), C4 (open encrypted), E11, E12 |
| H Conversion and extraction | C5–C6 (render, text), E2, E3, E13, E15, E16 |
| I Navigation and structure | C6 (read), E2 (labels), E3, E15 (reading order) |
| J Metadata and attachments | C6 (properties), E1, E3 |
| K Printing and optimisation | C6/D4 (print), E16 |
| L Accessibility | C5/C7 (read tagged), E15 |
| M Advanced interactions | E3 (associated files), E14 (revisions, compare), E17 |

## 8. Risks

| Risk | Mitigation |
| --- | --- |
| Editing existing text is open-ended (06 §7 rank 7) | Ship E6 (add content) first; best-effort labelling; corpus of real edits; never break files we can't edit (fall back to read-only) |
| Re-render latency after content edits (two parsers) | Overlay rendering for annotations/fields; incremental reload of touched pages measured from E6; hayro long-term |
| Redaction leaks | Dedicated E12 with extraction tests across four tools and revision checks |
| Signature validation attacks (shadow, incremental) | Revision diff analysis in E10; tests from published attack catalogues; external review before release |
| Scope (~180 ops) | Capability matrix as the single source of truth; rounds deliver user-visible value in order |
| Engine licences for OCR/conversion | ADR 0013 checks each engine; external process boundary for copyleft tools |
| JavaScript in forms | ADR 0021 before E8; declarative cases first |

## 9. Open questions

1. JavaScript: never execute, or run a restricted sandbox (for example a WASM JS engine with only the Acrobat form-helper API) for formatting and calculation?
2. Which OCR engine (licence, size, quality) for client-side and Container use?
3. Is batch processing a desktop/CLI feature only, or also a hosted service?
4. Do we build a layout-aware "reflow edit" mode for paragraphs, or keep line-level editing only?
5. Qualified electronic signatures (eIDAS QES) via remote signing services: in scope, and which providers?
6. Does collaboration (E18) need accounts, or can it start with capability links?

## 10. To-dos

Each round's todos (one per PR, with dependencies) are linked from [§7](#7-phases-and-rounds) and on the [todo board](../todos/README.md); this checklist is the summary.

### Cross-cutting
Todos: [X2](../todos/done/X2-capability-matrix.md) (capability matrix; E1a and E1f depend on it)
- [x] Assign stable IDs `<group>.<nn>` to all operations in 06 §3 and create `docs/capabilities/manifest.toml`
- [x] Implement test tagging and `cargo xtask capabilities`; generate `docs/capabilities/README.md`
- [x] CI gate: no status claimed without a passing tagged test
- [ ] Show the capability matrix in the app (E1) and docs site (the generated page is [`docs/capabilities/`](../capabilities/README.md))

### E1. Editing foundation
- [ ] Feature-plan PR for E1
- [ ] Interaction-mode state machine in `@laminapdf/ui`
- [ ] Undo/redo commands, shortcuts and history panel
- [ ] Save, Save as and Download copy through the `Platform` adapter
- [ ] `SaveConstraint` banner and explanations
- [ ] Metadata editing ops (J.01–J.03, J.10) writing Info and XMP in sync, with round-trip tests in core, `@laminapdf/sdk`, web, desktop
- [ ] XMP packet parse/update for Info-mapped properties; create a packet when missing
- [ ] PDF/A-aware metadata edits: keep `pdfaid:`, warn before a save that breaks the claim; tests that veraPDF still passes on edited PDF/A fixtures

### E2. Page organisation
- [ ] Feature-plan PR for E2
- [ ] Page ops (insert, delete, duplicate, reorder, rotate, reverse, replace) as view-level `LaminaPdfId` mapping
- [ ] Merge with outline, destination, AcroForm and struct-tree remapping
- [ ] Split by count, ranges and bookmarks; extract and export pages
- [ ] `laminapdf-content` append API: wrap existing content in `q … Q`, append streams, register Form XObjects, balance `q`/`Q` by token scan
- [ ] Crop, resize (content scaled via the append API), page boxes, orientation; crop warning
- [ ] Page labels editor
- [ ] Thumbnail drag-and-drop and multi-select UI
- [ ] DocMDP gating tests on signed fixtures

### E3. Navigation, attachments, inspection and extraction
- [ ] Feature-plan PR for E3
- [ ] Bookmark editor (nested, drag), links and named destinations, TOC generator
- [ ] Initial view and page display settings
- [ ] General XMP editor (custom and extension schemas); attachments panel (add, extract, delete); `/AF` support
- [ ] Object inspector (objects, streams, references, revisions)
- [ ] Fonts and colour profiles report; version and conformance claims
- [ ] Images → PDF; page export to PNG/JPEG/TIFF; text, image, link and attachment extraction

### E4. Markup and drawing annotations
- [ ] Feature-plan PR for E4
- [ ] Annotation model with `/NM` IDs and overlay rendering
- [ ] Appearance-stream generator for every subtype shipped
- [ ] Tools: highlight, underline, strike-through, note, caret, ink, shapes, lines, arrows, callouts, text boxes
- [ ] Properties panel (colour, opacity, border, line endings)
- [ ] Round-trip tests: appearance identical in pdf.js and PDFium

### E5. Review, stamps and interchange
- [ ] Feature-plan PR for E5
- [ ] Comment panel with replies, status, filter and search
- [ ] Custom stamps (image, dynamic text) and file-attachment annotations
- [ ] XFDF and FDF import/export with tests against Acrobat-produced fixtures
- [ ] Flatten annotations via the E2 append API; comment summary export

### E6. Adding content to pages
- [ ] Feature-plan PR for E6
- [ ] Add text, images, shapes and stamps as page content via Form XObjects
- [ ] Watermarks and backgrounds; headers, footers, page numbers, Bates numbering
- [ ] Measure and budget touched-page reload time in web and desktop

### E7. Editing existing content
- [ ] Write ADR 0022 (content-stream editing model for existing content)
- [ ] Feature-plan PR for E7
- [ ] Content IR with object identity; select and transform existing objects
- [ ] Text-run detection (MCID first, heuristics second) and in-place text editing
- [ ] Font handling: reuse subset when possible, re-subset or substitute otherwise
- [ ] Search and replace across the document
- [ ] Stacking order, clipping, vector path editing
- [ ] Struct-tree and MCID maintenance; PDF/A/UA claim checks on save
- [ ] Edit-corpus tests: output renders identically except edited regions

### E8. Form filling and data
- [ ] Write ADR 0021 (PDF JavaScript policy)
- [ ] Feature-plan PR for E8
- [ ] Field widgets as DOM overlays; tab order; required/read-only
- [ ] Value appearance generation for every field type
- [ ] Declarative validation, calculation order and Hide actions
- [ ] FDF, XFDF and JSON import/export; reset; flatten
- [ ] XFA detection and user-facing report

### E9. Form authoring
- [ ] Feature-plan PR for E9
- [ ] Form-design mode: create, move, resize, configure every field type
- [ ] Signature fields; accessible labels; tab-order editor
- [ ] Optional automatic field detection on flat forms

### E10. Signatures
- [ ] Write ADR 0020 (signatures and crypto backends, key storage per platform)
- [ ] Feature-plan PR for E10
- [ ] Visual signatures (image, drawn, typed) clearly labelled as non-cryptographic
- [ ] PAdES B-B/B-T signing with two-phase external signing API
- [ ] Certification (DocMDP), field locking (FieldMDP)
- [ ] Validation: integrity, chain, revocation, revision diff analysis, status UI
- [ ] Desktop keychain and PKCS#11; web WebCrypto and PKCS#12
- [ ] Optional TSA proxy Worker (hash only)
- [ ] Attack-catalogue tests (incremental saving, shadow attacks)

### E11. Security and sanitisation
- [ ] Feature-plan PR for E11
- [ ] AES-256 encryption on write (decryption exists since library B2); passwords and permissions UI
- [ ] Remove security with authorisation
- [ ] Sanitiser ops with a report (metadata, comments, attachments, scripts, hidden layers, embedded content)
- [ ] Public-key security: read; write behind a flag

### E12. Redaction
- [ ] Feature-plan PR for E12
- [ ] Mark, review and apply Redact annotations
- [ ] Remove text, image pixels and paths under areas; scrub annotations, fields, metadata, bookmarks, struct tree
- [ ] Pattern search for sensitive content
- [ ] Leak tests with pdf.js, PDFium, pypdf, pdftotext; revision-drop tests

### E13. OCR and conversion
- [ ] Write ADR 0013 (Workflows + Containers, engines and licences)
- [ ] Feature-plan PR for E13
- [ ] Invisible OCR text layer from hOCR/ALTO in `laminapdf-build`
- [ ] Client-side OCR prototype (web) and local OCR (desktop)
- [ ] Container images for OCR and Office conversion; Workflow with R2 lifecycle expiry
- [ ] HTML → PDF via Browser Run with lamina post-processing
- [ ] PDF → Word and tables → Excel (server), with quality fixtures

### E14. Compare, revisions and batch
- [ ] Feature-plan PR for E14
- [ ] Revision browser (open any prior revision read-only)
- [ ] Visual diff and structural diff views
- [ ] Batch runner over saved op sequences (desktop folders, web file sets, optional Workflows)

### E15. Accessibility
- [ ] Feature-plan PR for E15
- [ ] Tag tree panel and reading-order overlay
- [ ] Tagging tools: headings, paragraphs, lists, tables, artifacts, alt text, language
- [ ] Machine-checkable Matterhorn checks with fix-ups
- [ ] PDF/UA export; veraPDF agreement tests in CI

### E16. Optimisation and print production
- [ ] Feature-plan PR for E16
- [ ] Optimiser: images, fonts, unused objects, dedupe, object streams, linearisation
- [ ] Colour conversion, output intents, overprint preview, spot colours
- [ ] Preflight rules as data; PDF/A and PDF/X conversion and validation
- [ ] Imposition, booklet and N-up

### E17. Advanced interactions
- [ ] Feature-plan PR for E17
- [ ] Action editor (links, buttons, events, open actions; JavaScript per ADR 0021)
- [ ] Layers panel (show/hide, edit OCGs)
- [ ] Media playback (Screen/Rendition, RichMedia) and audio annotations
- [ ] Measurement tools and scales; geospatial read
- [ ] 3D detection with poster fallback

### E18. Collaboration
- [ ] Write ADR 0012 (Durable Object per document, LWW records, leases)
- [ ] Run the `auth` spike ([H1](../todos/ready/H1-auth-spike.md)) and record the result in ADR 0011 (before the first round that needs accounts, which may be earlier than E18)
- [ ] Feature-plan PR for E18
- [ ] Layer-document schema for annotations, field values and page order
- [ ] Durable Object with SQLite storage and WebSocket hibernation; R2 base revisions; D1 index
- [ ] Presence, sharing links, offline sync on desktop
- [ ] Load test: 50 concurrent reviewers on one document

### Hosted service: accounts, cloud storage and sync
Todos: [H1](../todos/ready/H1-auth-spike.md), [H2](../todos/ready/H2-accounts-and-login.md), [H3](../todos/ready/H3-cloud-document-storage.md), [H4](../todos/ready/H4-document-sync.md)
- [ ] `auth` spike: login stack recorded in ADR 0011 ([H1](../todos/ready/H1-auth-spike.md))
- [ ] Opt-in accounts and sign-in on web and desktop ([H2](../todos/ready/H2-accounts-and-login.md))
- [ ] Opt-in cloud storage of documents: R2 blobs, D1 index, per-user access ([H3](../todos/ready/H3-cloud-document-storage.md))
- [ ] Sync of cloud documents across devices ([H4](../todos/ready/H4-document-sync.md))
