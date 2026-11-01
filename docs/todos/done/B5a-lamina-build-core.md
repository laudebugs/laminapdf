# B5a — `lamina-build` core: builder, pages, vectors, colour and images

| Field | Value |
| --- | --- |
| ID | `B5a` |
| Status | `done` |
| Plan | [pdf-library §7 B5](../../plans/pdf-library.md#b5-builder-and-the-laminapdf-crate), requirements [§4 F1, F4, F5](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B1b](./B1b-krilla-fit-spike.md), [B3c](./B3c-save-planner-and-roundtrip.md), [L1a](./L1a-contract-adrs.md) |
| Blocks | [B5b](./B5b-builder-text-and-document-features.md) |
| ADRs | Follows 0027 (coordinates and units, from [L1a](../done/L1a-contract-adrs.md)); implements [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (builder writes `New` overlay entries; reserve-then-fill; `pdf_writer::Content`); follows [0001](../../decisions/0001-core-language-and-licence.md) (determinism), [0018](../../decisions/0018-test-strategy.md) tier 3 |
| Branch | `feat/b5a-lamina-build` |
| PR title | `feat(build): lamina-build with pages, paths, colour, opacity and images` |
| Size | L (≈ 1,000–1,400 lines; the moved content builder counts once) |
| PR | [#59](https://github.com/laudebugs/lamina/pull/59) |

## Goal

Create the generation API everything else hangs off: `Builder → page → surface → finish() → Document`, writing directly into a `Document` whose base is empty (or, later, an existing document's overlay), with vector graphics, colour, opacity, clipping and images. Promote the spike's content builder and golden samples (`hello`, `imaging_demo`), today test support in `crates/lamina-cos/tests/common/`, into this crate. Text and document-level features follow in [B5b](../done/B5b-builder-text-and-document-features.md).

## Context

- [ADR 0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges): how the builder fits the overlay; [krilla-fit experiment C](../../../spikes/krilla-fit/src/exp_c.rs) (PNG + SMask, JPEG pass-through).
- [pdf-library §5 API sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final): shapes to aim for (names fixed in ADR 0016).
- [Research 02 imaging model](../../research/pdf/02-imaging-model.md): paths, colour spaces, ExtGState, images, SMask; [05 §5.1](../../research/pdf/05-creating-pdfs-from-code.md#51-writer-architecture-streaming-versus-in-memory-object-graph).
- Code (moved here by this todo): [`crates/lamina-build/tests/common/content.rs`](../../../crates/laminapdf-build/tests/common/content.rs), [`legacy.rs`](../../../crates/laminapdf-build/tests/common/legacy.rs) (was `crates/lamina-save/tests/common/samples.rs`), goldens [`fixtures/golden/hello.pdf`](../../../fixtures/golden/hello.pdf), [`imaging-demo.pdf`](../../../fixtures/golden/imaging-demo.pdf).

## Scope

**In scope**

- New core crate `crates/lamina-build` (deps: `lamina-cos`, `lamina-model`, `lamina-ops`, `lamina-save`, `pdf-writer` for `Content`; image decoding: `png` (or `zune-png`) for PNG, JPEG header parsing only).
- `Builder::new(BuildOptions { version, id, … })`, `Builder::on(&mut Document)` (append to an existing document's overlay; used by E2/E6 later), `page(PageSize | Rect)` with boxes (`media`, `crop`, `bleed`, `trim`, `art`), `rotate`, `user_unit`; `PageSize` presets (A0–A6, Letter, Legal, Tabloid).
- `Surface`: `save`/`restore`, `transform`, `path(|p| …)` (move/line/cubic/quad→cubic/close/rect/ellipse), `stroke(StrokeStyle { width, cap, join, miter, dash })`, `fill(FillRule)`, `fill_stroke`, `clip(FillRule)`, `opacity(fill, stroke)` via deduplicated ExtGState resources, blend mode (Normal/Multiply/…).
- Colour: `Color::{Gray, Rgb, Cmyk}`, `#rrggbb` parsing, `ColorSpace::IccBased(IccProfile)` with caller-supplied ICC bytes (`/N` checked), deduplicated resources.
- Images: `Image::jpeg(bytes)` (pass-through `DCTDecode`, dimensions/components from SOF; CMYK Adobe inversion flag noted), `Image::png(bytes)` (decode; RGB/Gray + alpha → `/SMask`; 16-bit downconvert option; palette expansion), `Image::rgba(width, height, bytes)`; `surface.image(&img, Rect)`; images deduplicated by content hash.
- `finish() -> Result<Document>`: fills reserved objects, builds the page tree, catalog, (minimal) Info if requested; the returned `Document` is `Dirty` until saved; `doc.save(sink, SaveOptions::default())` performs a full write.
- Determinism: fixed `/ID` and dates supplied via `BuildOptions`; resource names allocated deterministically (`F1`, `Im1`, `GS1`).
- Promote `tests/common/content.rs` + `samples.rs` into `lamina-build` (the content builder becomes an internal module or is replaced by `pdf_writer::Content` — decide; the samples become `examples/`-style test sources); `hello` and `imaging_demo` goldens regenerated **only if** bytes must change (prefer identical; if they change, review the diff and record why).

**Out of scope** (and which todo covers it)

- Text, links, outlines, destinations, page labels, metadata/XMP, viewer preferences: [B5b](../done/B5b-builder-text-and-document-features.md).
- The `lamina` façade and Rust examples: [B5c](./B5c-lamina-facade-and-examples.md).
- Tagging and conformance: [B8a](./B8a-structure-tree.md), [B8b](../done/B8b-conformance-profiles-and-verapdf.md).
- Shadings, patterns, Form XObjects as API: later (full-editor E6).

## Tasks

- [x] Load `/rust-best-practices`; read ADR 0005 §6 and krilla-fit experiment C
- [x] Create `crates/lamina-build` (core); module layout `builder.rs`, `page.rs`, `surface/{mod,path,style,color,image}.rs`, `resources.rs`, `finish.rs` (plus `error.rs`)
- [x] Reserve/fill object helpers over `CosMut`; page tree writer (catalog and root reserved by `Builder::new`, filled by `finish`; `lamina_cos::Document::empty` for the empty base)
- [x] Move `tests/common/content.rs` and `samples.rs`; update `lamina-cos`/`lamina-save` tests to depend on `lamina-build` as a dev-dependency (or a test-kit crate) without a dependency cycle. *Deviation:* no dependency at all: the content builder and `hello`/`imaging_demo` moved to `lamina-build/tests/common/{content,legacy}.rs`, which `lamina-save`'s tests include by `#[path]`; `lamina-save` keeps its two edited samples
- [x] JPEG SOF parser (fill bytes, standalone markers — krilla-fit fixes), PNG decode + SMask, raw RGBA
- [x] ICC profile validation (header, `/N` from colour space)
- [x] Goldens: `hello`, `imaging-demo`, plus new `vectors.pdf`, `images.pdf` via `cargo xtask golden --update`
- [x] `Builder::on(&mut Document)` test: add a page to `fixtures/corpus/hello.pdf`, save incrementally, reopen

## Acceptance criteria

- [x] Public builder coordinates and units follow ADR 0027 (from [L1a](../done/L1a-contract-adrs.md)): typed units, the origin named in the API, raw user space only in low-level calls ([shareable-libraries §8](../../plans/shareable-libraries.md#8-changes-to-existing-todos)). Concretely ([ADR 0027](../../decisions/0027-coordinates-units-and-page-indexing.md)): `Pt` (`from_mm`, `from_in`), `PointPt`, `RectPt` in page space (top-left of the displayed CropBox, y down, `/UserUnit` applied); `lamina_model::Rect` renamed to `UserSpaceRect` (plus `UserSpacePoint`); `PageFrame` with `to_page`/`to_user_space` and a test per `/Rotate` value against ADR 0027 §3's table; `PageFrame` computed **once per page and cached in `Pages` next to the boxes** (today `rotate()`/`user_unit()` re-read on every call), so its warnings are raised once; the US Letter fallback (`[0 0 612 792]`, `invalid-box` for `MediaBox`) when no box has an area, with a test; `draw_text` `y` is the baseline. *Deviations:* `draw_text` is B5b's (carried into its scope); `Pt` and friends have no serde derives yet (`lamina-model` has no serde; added with the bindings)
- [x] `hello` and `imaging-demo` goldens are produced by `lamina-build` (byte-identical to today, or a reviewed diff with the reason in the PR): byte-identical, written by `lamina-build`'s `examples/golden.rs` from `tests/common/legacy.rs` with the spike's content builder and `lamina-save`'s `Writer`. *Decision:* the builder itself writes `pdf_writer::Content` (ADR 0005 §6); the spike's content builder stays test support only, because these frozen Python outputs need its exact formatting (a hand-wrapped page dictionary, `%` comments, fixed decimals, a classic xref without `/ID`) that no builder layout reproduces
- [x] Every new golden is byte-identical across two runs and across native and wasm32 (CI). *Deviation:* two runs are checked (`goldens.rs`); there is no wasm32 runtime harness in CI yet, so native/wasm32 identity rests on the code (no floats from the platform: `pdf-writer`'s `f32` formatting, `format_real`, no hash maps or clocks)
- [x] All generated files pass `qpdf --check`, open in pdf.js and PDFium (full lane, page count and no errors): `vectors.pdf` and `images.pdf` joined the round-trip suite (`cargo xtask roundtrip`, run locally with all three tools: 460 outputs clean), and pdf.js's operator counts for them are in `fixtures/corpus/pdfjs-operators.json`
- [x] PNG with alpha produces an `/SMask`; JPEG bytes are embedded unchanged; CMYK JPEG renders correctly in PDFium (checked by rendering `images.pdf` with pypdfium2 5.14.0)
- [x] Adding a page to an existing document via `Builder::on` saves incrementally and keeps existing `PageId`s. *Deviation:* `Builder::on` takes a `lamina_model::Document` and returns an `Appender`; the page is saved with `IncrementalWriter`, since there is no op for drawn content to go through `lamina_ops` transactions yet (E2/E6)
- [x] No `HashMap`, no clocks; `cargo xtask portability` passes for `lamina-build`

## Post-merge checks

None.

## Tests

- Unit tests per module (path ops, colour parsing, JPEG SOF, resource dedup).
- `crates/lamina-build/tests/{goldens,images,vectors,append}.rs`.
- Fixtures: small PNG/JPEG/ICC test assets under `fixtures/images/` with provenance (generated where possible).

## Docs to update

- [x] [pdf-library §10 B5](../../plans/pdf-library.md#10-to-dos): tick "Create `crates/lamina-build`…" and the image/path/clip/opacity/colour part of "Surface ops"
- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick `lamina-build` and "Promote the spike's content builder…"
- [x] `CLAUDE.md` (layout: `tests/common` no longer holds the content builder), `fixtures/README.md`, crate README

## Risks and open questions

- Dependency cycle: `lamina-cos` tests using the builder from a crate above it; use a dev-dependency (allowed by Cargo) or a `publish = false` test-kit crate.
- PNG decoder choice affects WASM size (krilla-fit counted `png` in its probe); consider decoding only what's needed and measure in [B6a](../done/B6a-lamina-wasm-and-adr-0016.md).
- API names are provisional until ADR 0016 ([B6a](../done/B6a-lamina-wasm-and-adr-0016.md)); keep the public surface small and `#[non_exhaustive]`.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
