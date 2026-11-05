# L3e — Extraction in WASM and `@laminapdf/sdk`, and the extraction benchmark

| Field | Value |
| --- | --- |
| ID | `L3e` |
| Status | `ongoing` |
| Plan | [shareable-libraries §7 L3](../../plans/shareable-libraries.md#l3-ai-ready-extraction), principles [§4](../../plans/shareable-libraries.md#4-principles-recommendations-recorded-in-adrs-00260031) (5, 6) |
| Round / milestone | Round L3 (shareable libraries) — AI-ready extraction |
| Depends on | [L3b](../done/L3b-extract-tables-and-serialisers.md) |
| Blocks | None |
| ADRs | Implements 0030 for the web; follows 0016 (the extract module and its budget), [0002](../../decisions/0002-binding-strategy.md) (wasm-bindgen + tsify), 0026 (unknown option keys with "did you mean"), [0018](../../decisions/0018-test-strategy.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (benchmark data licences) |
| Branch | `feat/l3e-extract-bindings-and-benchmark` |
| PR title | `feat(sdk): doc.extract() in WASM and TypeScript, and the extraction benchmark` |
| Size | M (≈ 500–800 lines incl. bench tooling) |
| PR | — |

## Goal

Give TypeScript the extraction Rust has since [L3b](../done/L3b-extract-tables-and-serialisers.md), and measure what the rules achieve. L3b split this off at its size limit: L3b built the tables, the Markdown, JSON and chunk output, the JSON Schema and `laminapdf::Document::extract`; this todo exposes them in the extract module and `@laminapdf/sdk`, and runs a licence-clean public benchmark.

## Context

- L3b's todo has the full design of these parts (its WASM, TypeScript and benchmark bullets) and the benchmark licence risks.
- `laminapdf::Document::{extract, extract_job, extract_json}` and `laminapdf::extract::{Extraction, MarkdownOptions, ChunkOptions, …}` are the Rust surface to bind; `crates/laminapdf-extract/schemas/extract.schema.json` is the JSON's schema.
- [B9c](../done/B9c-text-extraction-bindings-and-differential.md): the extract module's bindings pattern and its Node/Chromium tests.

## Scope

**In scope**

- **WASM** (`laminapdf-wasm`, extract module per ADR 0016): `WasmDocument.extract(options)` returning the `laminapdf.extract/1` JSON as a string (one copy), Markdown and text as strings, chunks lazily; options as a tsify record (unknown keys `unknown-option`); the module's size against its budget.
- **TypeScript** (`@laminapdf/sdk`): `doc.extract({ format: "markdown" | "json" | "text" | "chunks", pages?, readingOrder?, drop?, tables?, chunk? })` with a return type per `format` (a string for Markdown and text, a typed `Extraction` for JSON, `Chunk[]` for chunks); types generated from Rust; an unknown option key rejected with a "did you mean" hint; Vitest in Node and Chromium.
- **Shared snapshot:** `doc.extract({ format: "json" })` equals the Rust JSON byte for byte on L3b's fixtures.
- **Benchmark:** `cargo xtask extract-bench` on a pinned public subset (`tools/bench/extract-datasets.toml`: URL, SHA-256, licence, subset IDs; data never committed unless its licence allows), reading-order score (OmniDocBench's normalised edit distance or the dataset's own metric) and table TEDS; scores recorded in `crates/laminapdf-extract/README.md` with the dataset, licence, subset size, version and date.
- The TypeScript half of the recipe "Extract text for an LLM" (a `.ts` twin of `extract_markdown.rs`).

**Out of scope** (and which todo covers it)

- Structure-tree order, headings, lists, tagged tables: [L3d](../ready/L3d-extract-structure-and-furniture.md).
- `laminapdf extract` output formats in the CLI: [L4b](../ready/L4b-cli-batch-render-schema.md).

## Tasks

- [ ] Extract module bindings and size measurement
- [ ] `doc.extract()` overloads, types, tests in Node and Chromium; README section
- [ ] Shared JSON snapshot between Rust and TypeScript
- [ ] Check each benchmark dataset's licence; the manifest; `extract-bench`; record the scores
- [ ] `extract-markdown.ts` and the recipe's TypeScript tab

## Acceptance criteria

- [ ] `doc.extract({ format: "json" })` equals the Rust JSON byte for byte on the fixtures; TS rejects an unknown `format` at compile time and an unknown option key at run time
- [ ] The extract `.wasm` is within its budget (or a reviewed budget change)
- [ ] `cargo xtask extract-bench` runs on the pinned subset; scores are recorded with dataset, licence, subset size, version and date

## Post-merge checks

- [ ] If `extract-bench` joins the nightly lane, its first run on `main` succeeds and matches the recorded scores

## Tests

- Vitest in Node and Chromium for `doc.extract()`; the shared JSON snapshot.
- `extract-bench` is not in the PR lanes.

## Docs to update

- [ ] [shareable-libraries §9 L3](../../plans/shareable-libraries.md#9-to-dos): tick the WASM, TypeScript and benchmark parts
- [ ] `packages/sdk/README.md` (extraction), `crates/laminapdf-extract/README.md` (scores), the recipe

## Risks and open questions

- **Benchmark licences and formats:** as L3b's todo lists them; use only PDF inputs under licences that allow use.
- **Size:** the extract module grows with the serialisers; measure before and after.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
