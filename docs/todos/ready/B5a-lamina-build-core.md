# B5a — `lamina-build` core: builder, pages, vectors, colour and images

| Field | Value |
| --- | --- |
| ID | `B5a` |
| Status | `ready` |
| Plan | [pdf-library §7 B5](../../plans/pdf-library.md#b5-builder-and-the-lamina-crate), requirements [§4 F1, F4, F5](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B1b](../done/B1b-krilla-fit-spike.md), [B3c](./B3c-save-planner-and-roundtrip.md) |
| Blocks | [B5b](./B5b-builder-text-and-document-features.md) |
| ADRs | Implements [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (builder writes `New` overlay entries; reserve-then-fill; `pdf_writer::Content`); follows [0001](../../decisions/0001-core-language-and-licence.md) (determinism), [0018](../../decisions/0018-test-strategy.md) tier 3 |
| Branch | `feat/b5a-lamina-build` |
| PR title | `feat(build): lamina-build with pages, paths, colour, opacity and images` |
| Size | L (≈ 1,000–1,400 lines; the moved content builder counts once) |
| PR | — |

## Goal

Create the generation API everything else hangs off: `Builder → page → surface → finish() → Document`, writing directly into a `Document` whose base is empty (or, later, an existing document's overlay), with vector graphics, colour, opacity, clipping and images. Promote the spike's content builder and golden samples (`hello`, `imaging_demo`), today test support in `crates/lamina-cos/tests/common/`, into this crate. Text and document-level features follow in [B5b](./B5b-builder-text-and-document-features.md).

## Context

- [ADR 0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges): how the builder fits the overlay; [krilla-fit experiment C](../../../spikes/krilla-fit/src/exp_c.rs) (PNG + SMask, JPEG pass-through).
- [pdf-library §5 API sketch](../../plans/pdf-library.md#api-sketch-illustrative-not-final): shapes to aim for (names fixed in ADR 0016).
- [Research 02 imaging model](../../research/pdf/02-imaging-model.md): paths, colour spaces, ExtGState, images, SMask; [05 §5.1](../../research/pdf/05-creating-pdfs-from-code.md#51-writer-architecture-streaming-versus-in-memory-object-graph).
- Code: [`crates/lamina-cos/tests/common/content.rs`](../../../crates/lamina-cos/tests/common/content.rs), [`samples.rs`](../../../crates/lamina-cos/tests/common/samples.rs), goldens [`fixtures/golden/hello.pdf`](../../../fixtures/golden/hello.pdf), [`imaging-demo.pdf`](../../../fixtures/golden/imaging-demo.pdf).

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

- Text, links, outlines, destinations, page labels, metadata/XMP, viewer preferences: [B5b](./B5b-builder-text-and-document-features.md).
- The `lamina` façade and Rust examples: [B5c](./B5c-lamina-facade-and-examples.md).
- Tagging and conformance: [B8a](./B8a-structure-tree.md), [B8b](./B8b-conformance-profiles-and-verapdf.md).
- Shadings, patterns, Form XObjects as API: later (full-editor E6).

## Tasks

- [ ] Load `/rust-best-practices`; read ADR 0005 §6 and krilla-fit experiment C
- [ ] Create `crates/lamina-build` (core); module layout `builder.rs`, `page.rs`, `surface/{mod,path,style,color,image}.rs`, `resources.rs`, `finish.rs`
- [ ] Reserve/fill object helpers over `CosMut`; page tree writer
- [ ] Move `tests/common/content.rs` and `samples.rs`; update `lamina-cos`/`lamina-save` tests to depend on `lamina-build` as a dev-dependency (or a test-kit crate) without a dependency cycle
- [ ] JPEG SOF parser (fill bytes, standalone markers — krilla-fit fixes), PNG decode + SMask, raw RGBA
- [ ] ICC profile validation (header, `/N` from colour space)
- [ ] Goldens: `hello`, `imaging-demo`, plus new `vectors.pdf`, `images.pdf` via `cargo xtask golden --update`
- [ ] `Builder::on(&mut Document)` test: add a page to `fixtures/corpus/hello.pdf`, save incrementally, reopen

## Acceptance criteria

- [ ] `hello` and `imaging-demo` goldens are produced by `lamina-build` (byte-identical to today, or a reviewed diff with the reason in the PR)
- [ ] Every new golden is byte-identical across two runs and across native and wasm32 (CI)
- [ ] All generated files pass `qpdf --check`, open in pdf.js and PDFium (full lane, page count and no errors)
- [ ] PNG with alpha produces an `/SMask`; JPEG bytes are embedded unchanged; CMYK JPEG renders correctly in PDFium
- [ ] Adding a page to an existing document via `Builder::on` saves incrementally and keeps existing `PageId`s
- [ ] No `HashMap`, no clocks; `cargo xtask portability` passes for `lamina-build`

## Post-merge checks

None.

## Tests

- Unit tests per module (path ops, colour parsing, JPEG SOF, resource dedup).
- `crates/lamina-build/tests/{goldens,images,vectors,append}.rs`.
- Fixtures: small PNG/JPEG/ICC test assets under `fixtures/images/` with provenance (generated where possible).

## Docs to update

- [ ] [pdf-library §10 B5](../../plans/pdf-library.md#10-to-dos): tick "Create `crates/lamina-build`…" and the image/path/clip/opacity/colour part of "Surface ops"
- [ ] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick `lamina-build` and "Promote the spike's content builder…"
- [ ] `CLAUDE.md` (layout: `tests/common` no longer holds the content builder), `fixtures/README.md`, crate README

## Risks and open questions

- Dependency cycle: `lamina-cos` tests using the builder from a crate above it; use a dev-dependency (allowed by Cargo) or a `publish = false` test-kit crate.
- PNG decoder choice affects WASM size (krilla-fit counted `png` in its probe); consider decoding only what's needed and measure in [B6a](./B6a-lamina-wasm-and-adr-0016.md).
- API names are provisional until ADR 0016 ([B6a](./B6a-lamina-wasm-and-adr-0016.md)); keep the public surface small and `#[non_exhaustive]`.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
