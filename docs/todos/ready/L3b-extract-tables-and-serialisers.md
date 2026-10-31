# L3b — Lattice tables, Markdown/JSON/chunk serialisers, extraction in the façade, WASM and `@lamina/pdf`

| Field | Value |
| --- | --- |
| ID | `L3b` |
| Status | `ready` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction), principles [§4](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) (5, 6) |
| Round / milestone | Round L3 (shareable libraries) — AI-ready extraction |
| Depends on | [B9c](../done/B9c-text-extraction-bindings-and-differential.md), [L3a](./L3a-lamina-extract-reading-order.md) |
| Blocks | [L3c](./L3c-model-plugins.md), [L4b](./L4b-cli-batch-render-schema.md) |
| ADRs | Implements 0030 (from [L3a](./L3a-lamina-extract-reading-order.md)); follows 0026, 0027, 0028 (from [L1a](../done/L1a-contract-adrs.md)), 0016 (from [B6a](../done/B6a-lamina-wasm-and-adr-0016.md): separate extract module and its budget), [0002](../../decisions/0002-binding-strategy.md) (wasm-bindgen + tsify), [0001](../../decisions/0001-core-language-and-licence.md), [0018](../../decisions/0018-test-strategy.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `feat/l3b-extract-tables-and-output` |
| PR title | `feat(extract): lattice tables, Markdown/JSON/chunk output and doc.extract() in Rust, WASM and TS` |
| Size | L (≈ 1,000–1,400 lines incl. bindings and bench tooling; split at the line in Risks if it grows) |
| PR | — |

## Goal

Finish the L3 deliverable. Detect tables from vector ruling lines (with a conservative stream fallback), serialise an `Extraction` to Markdown, `lamina.extract/1` JSON and retrieval-sized chunks with provenance, and expose it on every surface: `lamina::Document::extract` in Rust, the extract module in `lamina-wasm`, and `doc.extract({ format })` in `@lamina/pdf`. Measure reading-order and table scores on a licence-clean public benchmark subset and record them, so users know what rules achieve before models (L3c).

## Context

- [Report: rule-based extraction in core, models as licence-clean plug-ins](../../research/libraries/shareable-libraries.md#rule-based-extraction-in-core-models-as-licence-clean-plug-ins): lattice tables from vector paths (easier than Camelot's raster approach), the three-surface API sketch, the JSON example, borderless tables at TEDS 0.49 without models.
- [AI extraction note §B](../../research/libraries/notes/llm-api-and-ai-extraction.md#b-ai-consumption-primitives-the-2026-landscape-algorithms-bboxprovenance-schema-tagged-pdf-and-rule-based-vs-model-based): Camelot lattice/stream, pdfplumber's snap/join/intersection tolerances (MIT), Tabula; the chunk schema (`blockIds`, `headingPath`, `pages`, `regions`, `textRange`); benchmarks (olmOCR-Bench, OmniDocBench, OpenDataLoader's).
- [Report: the 350 KB budget fits parse, edit and save only](../../research/libraries/shareable-libraries.md#the-350-kb-budget-fits-parse-edit-and-save-only): extract is its own `.wasm` with its own budget.
- [Research 03 §8.4 table extraction](../../research/pdf/03-text-and-fonts.md#84-table-extraction), [Research 02 §5 paths](../../research/pdf/02-imaging-model.md#5-paths) (`re`, `m`/`l`, stroke vs fill, CTM).
- ISO 32000-2 §8.5 (path construction and painting), §14.8.4.8.3 table structure types (`Table`, `TR`, `TH`, `TD`, `/Scope`, `/RowSpan`, `/ColSpan`).
- [B9c](../done/B9c-text-extraction-bindings-and-differential.md): the WASM text bindings pattern (struct-of-arrays, transferable buffers) and Node/Chromium test setup to reuse.

## Scope

**In scope**

- **Lattice tables** (`lamina-extract::tables::lattice`): collect stroked line segments and thin filled rectangles from `lamina-content` path events in page space (top-left points); snap (default 3 pt), join collinear segments, find intersections, build the cell grid, infer row and column spans from missing inner edges, assign glyphs to cells by glyph centre, mark header rows (tagged `TH`, else bold or shaded first row). Tagged tables (`Table`/`TR`/`TH`/`TD` with spans) win when the structure tree is used. Output `Block { kind: Table { rows, cols, detector }, cells: [Cell { row, col, row_span, col_span, header, text, bbox }] }`.
- **Stream fallback** (`TableMode::LatticeAndStream`, off by default): whitespace column inference inside a region only when at least 3 rows and 2 columns align within tolerance; `detector: "stream"` and lower `confidence`. Conservative by design: no tables beat wrong tables.
- **Serialisers** (`lamina-extract::output`):
  - *Markdown*: `#`–`######` headings, paragraphs, nested lists, GFM pipe tables (escape `|`; tables with spans fall back to an HTML `<table>`), optional `<!-- page N -->` markers; hidden and dropped text never appears.
  - *JSON* `lamina.extract/1`: every key from ADR 0030; `textRange` offsets into the Markdown (or plain-text) output; deterministic key order and number formatting (fixed decimals for coordinates).
  - *Chunks*: `ChunkOptions { max_tokens, overlap, split_on: Headings | Blocks | Pages }`; token count approximated by a documented rule (no tokenizer dependency); each chunk carries `id`, `text`, `blockIds`, `headingPath`, `pages`, `regions` (page + bbox), `textRange`, `tokensApprox`; JSONL for streaming.
  - *Plain text*: reading-order text with blank lines between blocks.
- **JSON Schema** 2020-12 for `lamina.extract/1` generated from the Rust types (schemars, licence-checked, behind a build-time feature or xtask, not a runtime dependency of core), committed under the schema location ADR 0030 chose; a check fails if the committed schema is stale.
- **Façade** (`lamina` crate): `Document::extract(ExtractOptions) -> ExtractJob` and `Extraction::{to_markdown, to_json, to_text, chunks}`, behind a feature `extract` so structure-only builds stay small.
- **WASM** (`lamina-wasm`): extract bindings in the separate extract module per ADR 0016; JSON output crosses as a string (one copy) and chunk iteration is lazy; size measured against the module budget.
- **TypeScript** (`@lamina/pdf`): `doc.extract({ format: "markdown" | "json" | "text" | "chunks", pages?, readingOrder?, drop?, tables?, chunk? })` with discriminated return types (`string` for Markdown and text, typed `Extraction` for JSON, `Chunk[]` or an iterator for chunks); tsify types generated from Rust; unknown option keys rejected with a "did you mean" hint (ADR 0026); the subpath (e.g. `@lamina/pdf/extract`) follows B6a's module split.
- **Benchmark**: `cargo xtask extract-bench` runs extraction on a pinned public benchmark subset fetched by URL and SHA-256 (never committed unless the licence allows redistribution), computes a reading-order score (normalised edit distance over block order, as OmniDocBench defines it, or the benchmark's own metric) and table TEDS, and writes a results table. Results recorded in the crate README with the tool version, subset IDs and date.

**Out of scope** (and which todo covers it)

- Layout models, borderless-table models, OCR: [L3c](./L3c-model-plugins.md).
- `lamina extract` CLI commands and `lamina schema extract/1`: [L4b](./L4b-cli-batch-render-schema.md).
- `@lamina/pdf/worker` wrapping of `extract`: [L2a](../done/L2a-worker-client.md) (it wraps whatever the sync API exposes).
- Agent-facing docs for extraction: [L5a](./L5a-agent-docs.md).
- Editor UI for extraction (export dialog): full-editor [E3i](./E3i-extraction.md).

## Tasks

- [ ] Load `/rust-best-practices` and follow [docs/guidelines/rust.md](../../guidelines/rust.md); `lamina-extract` stays `core`, `#![forbid(unsafe_code)]`, bytes in / bytes out; read the Context links and ISO 32000-2 §8.5, §14.8.4.8.3
- [ ] `lamina-extract/src/tables/{mod,edges,lattice,stream,tagged}.rs`; reuse `lamina-content`'s `paint_path` events (add a path-segments view there if B9a only reports paint events)
- [ ] `lamina-extract/src/output/{markdown,json,text,chunks}.rs`; `textRange` bookkeeping shared between Markdown and JSON
- [ ] Schema generation: schemars derives behind a feature; `cargo xtask schemas [--check]` (or extend L1b's codegen command if it exists) writes the committed schema file
- [ ] Façade: `lamina` feature `extract`, re-exports, docs with a runnable example (`examples/extract_markdown.rs`)
- [ ] `lamina-wasm`: extract module bindings; size measurement recorded against ADR 0016's extract budget
- [ ] `@lamina/pdf`: `doc.extract()` overloads, types, tests in Node and Chromium (B6c config); README section
- [ ] `tools/xtask`: `extract-bench` with a pinned dataset manifest (`tools/bench/extract-datasets.toml`: URL, SHA-256, licence, subset IDs); run it and record scores
- [ ] Check every benchmark dataset's licence before use (see Risks); record licence and source in the manifest
- [ ] New warning codes (e.g. `extract-table-ambiguous`, `extract-table-limit`) in `fixtures/codes.json` with tests

## Acceptance criteria

- [ ] Lattice fixtures (simple grid, merged cells, header row, table across a page break as two tables) produce the expected cells, spans and header flags in the golden
- [ ] Tagged table fixture uses the structure tree's rows, cells and spans
- [ ] A ruled form that is not a table (boxes around fields, a page border) produces no table (or one with low confidence, per the golden)
- [ ] Markdown output for each L3a and L3b fixture equals its golden; hidden text from L3a's hidden-text fixture appears nowhere in Markdown, text or chunk output and is listed in JSON `dropped`
- [ ] JSON output validates against the committed JSON Schema in a test; the schema check fails when a Rust output type changes without regenerating
- [ ] Every chunk's `textRange` slices the Markdown to exactly the chunk's text; every chunk has non-empty `pages` and `regions`
- [ ] `doc.extract({ format: "json" })` in TS equals the Rust JSON byte for byte on the fixtures (shared snapshot); TS types reject an unknown `format` at compile time and an unknown option key at runtime
- [ ] Extract `.wasm` size is within ADR 0016's extract budget (or a reviewed budget change)
- [ ] `cargo xtask extract-bench` runs on the pinned subset; reading-order and table scores are recorded in the crate README with dataset, licence, subset size, lamina version and date

## Post-merge checks

- [ ] If `extract-bench` is added to the full or nightly lane, its first scheduled run on `main` succeeds and the scores match the recorded ones

## Tests

- Unit tests: edge snapping and joining, intersection grid, span inference, glyph-to-cell assignment, stream column inference, Markdown escaping, chunk splitting and overlap, `textRange` arithmetic.
- Integration: `crates/lamina-extract/tests/{tables,markdown,json_schema,chunks}.rs` against goldens in `fixtures/extract/` (generated tables via `lamina-build`; a few permissive third-party table PDFs with provenance).
- Façade doc tests and the example.
- TS: Vitest in Node and Chromium; shared JSON snapshot with Rust.
- Benchmark: `extract-bench` (not in the PR lane; data not committed unless licensed for it).

## Docs to update

- [ ] [shareable-libraries §9 L3](../../plans/shareable-libraries.md#9-to-dos): tick "Lattice tables; Markdown, JSON and chunk serialisers; expose in Rust, WASM and TS"
- [ ] `crates/lamina-extract/README.md` (tables, formats, benchmark results), `packages/pdf/README.md` (extract section), `lamina` crate docs
- [ ] `fixtures/README.md` (table fixtures), `tools/` README for the bench manifest
- [ ] ADR 0016 note if the extract budget changes; ADR 0030 if the schema gains keys during the work

## Risks and open questions

- **Benchmark licences and formats (to verify).** OmniDocBench, olmOCR-Bench and OpenDataLoader's benchmark have different data licences, and some inputs are page images rather than born-digital PDFs, which a rule-based extractor cannot read without OCR. Use only subsets whose licence allows use (and redistribution, if any file is committed), and only PDF inputs. Candidates with table ground truth (PubTables-1M, FinTabNet, PubTabNet) also need a licence check. Record each decision in the manifest. Vendor scores (OpenDataLoader) are not comparable to ours unless the same data and metric are used; say so next to the numbers.
- **TEDS implementation.** Implement TEDS in xtask or use a reference implementation only if its licence is permissive (verify).
- **Stream fallback false positives.** Keep it off by default; revisit after L3c adds model-detected table regions.
- **Token approximation.** Without a tokenizer, `max_tokens` is approximate; document the rule and the expected error.
- **Size.** If the PR passes ~1,400 lines, split: L3b (tables, serialisers, schema, façade) and L3b2 (WASM, TS, benchmark), chained.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
