# L3c — Model plug-ins: `LayoutModel` and `OcrEngine` traits in core, an optional ONNX layout edge crate

| Field | Value |
| --- | --- |
| ID | `L3c` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction), principle [§4](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) 7 (rules in core, models as plug-ins) |
| Round / milestone | Round L3 (shareable libraries) — AI-ready extraction |
| Depends on | [L3b](./L3b-extract-tables-and-serialisers.md) |
| Blocks | None |
| ADRs | Implements 0030's plug-in policy (from [L3a](./L3a-lamina-extract-reading-order.md)); follows 0028 (jobs; from [L1a](../ongoing/L1a-contract-adrs.md)), [0001](../../decisions/0001-core-language-and-licence.md) (no AGPL/GPL, bytes in / bytes out), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (licence checks), [0002](../../decisions/0002-binding-strategy.md) (bindings); relates to ADR 0013 (heavy jobs, from [E13a](./E13a-adr-0013-heavy-jobs.md)) |
| Branch | `feat/l3c-model-plugins` |
| PR title | `feat(extract): LayoutModel and OcrEngine plug-in traits and an optional ONNX layout crate` |
| Size | M–L (≈ 500–900 lines; model weights are downloaded, never committed) |
| PR | — |

## Goal

Let models improve extraction where rules fail (complex untagged layouts, borderless tables, scans) without putting weights or native runtimes in core. Core `lamina-extract` gets two small traits, `LayoutModel` and `OcrEngine`, and uses their results when the caller supplies them. An optional edge crate runs a permissively licensed layout model (recommendation: Docling's Heron via ONNX, licence to verify) on native targets. A licence gate makes sure no model with restricted weights can be wired in by accident.

## Context

- [Report: rule-based extraction in core, models as licence-clean plug-ins](../../research/libraries/shareable-libraries.md#rule-based-extraction-in-core-models-as-licence-clean-plug-ins): Heron is Apache-2.0 with an ONNX export; OCR behind a trait with Apple Vision and ML Kit at the mobile edges; avoid MinerU, Marker/Surya weights and PyMuPDF; Docling's and Marker's licences to verify.
- [AI extraction note §B](../../research/libraries/notes/llm-api-and-ai-extraction.md#b-ai-consumption-primitives-the-2026-landscape-algorithms-bboxprovenance-schema-tagged-pdf-and-rule-based-vs-model-based): Heron (RT-DETRv2), `ort` and onnxruntime-web, TableFormer (licence unverified), TATR (MIT per a third-party mirror), PaddleOCR and Tesseract (Apache-2.0, background knowledge), Kreuzberg's packaged layout models.
- [Report: cancellation and progress belong in lamina's API](../../research/libraries/shareable-libraries.md#cancellation-and-progress-belong-in-laminas-api-not-the-bindings): the core stays synchronous; async hosts (WebCrypto-style, platform OCR) need a request/provide shape.
- [Research 03 §8.3 scanned PDFs and OCR](../../research/pdf/03-text-and-fonts.md#83-scanned-pdfs-and-ocr), [§8.5 layout analysis](../../research/pdf/03-text-and-fonts.md#85-layout-analysis-and-ml-document-understanding-state-of-the-art-2026).
- [E13d](./E13d-local-ocr.md) (local OCR on web and desktop) and [full-editor E13](../../plans/full-editor.md#e13-ocr-and-conversion): the editor's OCR engines should implement this PR's `OcrEngine` rather than a second interface.
- [C2b](./C2b-lamina-render-crate.md): models need page images; rendering stays in `lamina-render`, never in `lamina-extract`.

## Scope

**In scope**

- **Traits in `lamina-extract` (core, no new dependencies):**
  - `LayoutModel: Send + Sync` — `fn detect(&self, page: &PageRaster, cancel: &CancelToken) -> Result<Vec<LayoutRegion>, ModelError>`; `LayoutRegion { label: RegionLabel, bbox: RectPt, score }` with labels mapped from the model (text, title, section-header, list-item, table, picture, caption, footnote, formula, page-header, page-footer); `fn info(&self) -> ModelInfo { name, version, sha256, licence }`.
  - `OcrEngine: Send + Sync` — `fn recognise(&self, page: &PageRaster, langs: &[String], cancel: &CancelToken) -> Result<Vec<OcrWord>, ModelError>`; `OcrWord { text, bbox: RectPt, confidence }`; `fn info(&self) -> ModelInfo`.
  - `PageRaster { width_px, height_px, dpi, pixels: Vec<u8> (format documented), page_box: RectPt }` supplied by the caller; core never renders.
- **Request/provide path for async hosts** (recommendation; decide in the PR and record in ADR 0030): the extraction job's `step` may return `Step::NeedsRaster { page, dpi }` and later `Step::NeedsModel(ModelRequest)`; the host renders and runs the model (Swift `async`, Kotlin `suspend`, JS promises, a worker) and calls `job.provide(result)`. The sync trait path is a thin adapter over the same steps.
- **Use of results in extraction:**
  - Layout regions group runs into blocks within regions, feed XY-Cut++ with regions instead of rule-built blocks, set block kinds from labels, and mark `page-header`/`page-footer` regions for the L3a furniture pass.
  - Table regions without ruling lines run L3b's stream fallback inside the region (cell structure models are out of scope).
  - OCR runs on pages without a usable text layer (no glyphs, or only `Tr 3` text whose quality is in doubt — policy documented) and produces spans flagged `ocr` with confidence; OCR text in an existing invisible layer is not duplicated.
  - Output records `generator.mode` (`rules`, `rules+layout`, `rules+ocr`, …) and every model's `ModelInfo` in `generator.models`.
- **Optional edge crate `crates/lamina-layout-onnx`** (layer `edge`, native targets only): implements `LayoutModel` with `ort` (ONNX Runtime bindings; licences of `ort` and ONNX Runtime to verify, both expected MIT/Apache) for the Heron ONNX export; pre- and post-processing (resize, normalise, NMS, label map); weights loaded from bytes or a path given by the app, never bundled.
- **Model manifest and licence gate:** `tools/models/models.toml` lists allowed models (name, version, URL, SHA-256, code licence, weights licence, source of the licence statement); `cargo xtask models fetch <name>` downloads and verifies by hash into a gitignored cache; `cargo xtask models check` fails if any entry's code or weights licence is outside the allow-list (Apache-2.0, MIT, BSD-2/3-Clause, CDLA-Permissive) or if a weights file appears in the repository.
- **Docs:** a "using models" section in the `lamina-extract` README that states what the rules do alone and what each plug-in adds, with the licence of each listed model.

**Out of scope** (and which todo covers it)

- OCR engines themselves on web and desktop: [E13d](./E13d-local-ocr.md) (implements `OcrEngine`); OCR text-layer writing into the PDF: [E13c](./E13c-ocr-text-layer.md).
- Apple Vision and ML Kit `OcrEngine` implementations: the mobile SDK rounds from [L6b](./L6b-mobile-sdk-plan.md).
- onnxruntime-web layout model in the browser: a follow-up todo (record it in the plan); this PR documents the shape only.
- Table-structure models (TableFormer, TATR), formula and figure-description models, VLM fallbacks: later todos, each after a licence check.
- Server-side model jobs: ADR 0013 / [E13e](./E13e-server-jobs-infrastructure.md).

## Tasks

- [ ] Load `/rust-best-practices` and follow [docs/guidelines/rust.md](../../guidelines/rust.md); `lamina-extract` stays `core` and `#![forbid(unsafe_code)]`; `lamina-layout-onnx` is `edge` (any `unsafe` only inside `ort`; none in our code, or `// SAFETY:` comments if unavoidable)
- [ ] **Verify licences first** and write the findings into `models.toml` with links: Heron weights and its ONNX export (Hugging Face model cards), Docling code (MIT vs Apache-2.0 conflict), RT-DETRv2 code, `ort`, ONNX Runtime; note Marker (code reportedly Apache-2.0 since v2.0.0, Surya weights under modified OpenRAIL-M: excluded) and MinerU (custom conditions: excluded)
- [ ] `lamina-extract/src/plugins/{mod,layout,ocr,raster}.rs`: traits, types, `ModelError` (`thiserror`), `ModelInfo`
- [ ] Extend `ExtractOptions` with `layout: Option<Arc<dyn LayoutModel>>`, `ocr: Option<Arc<dyn OcrEngine>>` (façade-friendly; no generics at the boundary per ADR 0028) and the request/provide steps in `ExtractJob`
- [ ] Integrate regions into block building, ordering, furniture and table passes; integrate OCR words as synthetic runs
- [ ] `crates/lamina-layout-onnx`: crate with `[package.metadata.lamina] layer = "edge"`; Heron pre/post-processing; tests behind `#[ignore]` that need fetched weights
- [ ] `tools/xtask`: `models fetch`, `models check`; add `models check` to `cargo xtask check` (fast, offline); `deny.toml` updates for `ort` and its dependencies
- [ ] Façade: re-export the traits behind the `extract` feature; `lamina-wasm` exposes the request/provide steps (no model runtime in wasm)
- [ ] README and ADR 0030 updates (request/provide decision, allow-list)

## Acceptance criteria

- [ ] With a fake `LayoutModel` returning fixed regions, a two-column fixture whose rules-only order is wrong comes out in the order the regions imply (golden), and `generator.mode` and `generator.models` record the fake model
- [ ] With a fake `OcrEngine`, an image-only page produces blocks flagged `ocr` with confidences; a page with a real text layer is not OCR'd
- [ ] The request/provide path produces the same output as the sync trait path for the same fake results (test)
- [ ] Cancelling during a model step returns `Cancelled` and the model receives the token
- [ ] `cargo xtask models check` passes on `main`, and fails on a test manifest entry with a non-allowed licence and on a weights file added to the tree
- [ ] `lamina-extract` and `lamina` still pass `cargo xtask portability` (no `ort`, no unsafe, wasm32 builds); `lamina-layout-onnx` builds on Linux, macOS and Windows
- [ ] With Heron weights fetched (full lane or locally), `lamina-layout-onnx` detects regions on the layout fixtures within a documented IoU threshold of a recorded reference run; scores before and after the plug-in recorded on L3b's benchmark subset

## Post-merge checks

- [ ] If the weights-dependent tests run in the full or nightly lane, their first scheduled run on `main` passes

## Tests

- Unit tests: region-to-block assignment, label mapping, OCR word to run conversion, request/provide state machine.
- Integration: `crates/lamina-extract/tests/plugins.rs` with deterministic fakes (no weights).
- `crates/lamina-layout-onnx/tests/heron.rs` (`#[ignore]`, run via `cargo xtask models test` after `models fetch`).
- xtask tests for `models check` with fixture manifests.

## Docs to update

- [ ] [shareable-libraries §9 L3](../../plans/shareable-libraries.md#9-to-dos): tick "`LayoutModel` and `OcrEngine` traits; optional model edge crate"; add a to-do for the onnxruntime-web follow-up
- [ ] ADR 0030 (plug-in section, allow-list, request/provide), `crates/lamina-extract/README.md`, new `crates/lamina-layout-onnx/README.md`
- [ ] E13d: note that its engines implement `OcrEngine` (edit that todo in this PR or file a follow-up)
- [ ] `CLAUDE.md` layout tree (new edge crate), `tools/` README (model manifest)

## Risks and open questions

- **Licences to verify:** Heron weights and ONNX export (Apache-2.0 per the Hugging Face card), Docling code (MIT per most sources, Apache-2.0 per one), Marker's code move to Apache-2.0 (single blog source; weights stay restricted either way). If Heron's licence does not hold up, ship the traits and fakes only and leave the edge crate for a follow-up.
- **ONNX determinism.** CPU inference may differ across platforms and ONNX Runtime versions; compare with IoU thresholds, never byte equality, and pin the runtime.
- **Binary weight.** ONNX Runtime adds tens of MB natively; the edge crate must stay optional and out of default features everywhere.
- **Trait shape for UniFFI.** Swift/Kotlin implementing `OcrEngine` needs foreign traits (`#[uniffi::export(foreign)]`, UniFFI 0.32) or callback interfaces; the request/provide path avoids depending on that. [L6a](./L6a-lamina-ffi-spike.md) checks which works.
- **OCR on pages with `Tr 3` layers** (existing OCR from scanners): trusting vs replacing them is a policy choice; default to trusting, flag low-quality layers, decide in the PR.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
