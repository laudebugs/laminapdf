# B9c — `page.text()` in WASM and TypeScript; differential and quad tests; extraction fuzzing

| Field | Value |
| --- | --- |
| ID | `B9c` |
| Status | `ready` |
| Plan | [pdf-library §7 B9](../../plans/pdf-library.md#b9-text-extraction) |
| Round / milestone | Round B (library) — M2 Web reader v0.1 (prerequisite) |
| Depends on | [B6b](./B6b-lamina-pdf-package.md), [B6c](./B6c-runtime-matrix.md), [B9d](./B9d-text-runs-quads-and-page-text.md) |
| Blocks | [L3b](./L3b-extract-tables-and-serialisers.md), [L4a](./L4a-lamina-cli.md), [C5a](./C5a-text-layer-and-selection.md), [C5b](./C5b-find-in-document.md), [C6c](./C6c-print-download-properties.md), [E2a](./E2a-page-organisation-feature-plan.md), [E2e](./E2e-content-append-api.md), [E3i](./E3i-extraction.md), [E7a](./E7a-adr-0022-content-editing.md) |
| ADRs | Follows 0023, 0016 (module placement: extraction lives in the text module or base — decide by size), [0018](../../decisions/0018-test-strategy.md) tiers 6 and 9 |
| Branch | `feat/b9c-text-bindings` |
| PR title | `feat(pdf): page.text() in WASM and TypeScript with differential and quad tests` |
| Size | M–L (≈ 600–1,000 lines incl. test tooling) |
| PR | — |

## Goal

Expose text extraction everywhere the readers need it and prove it against independent tools: `page.text()` in `lamina-wasm` and `@lamina/pdf` returning glyph runs with quads in a compact transferable form, extracted text compared with pdf.js and pdftotext on the corpus, glyph quads compared with PDFium's character boxes, and an extraction fuzz target running 10 minutes clean. When this merges, web C5 can build its text layer.

## Context

- [B9b](../done/B9b-text-extraction-core.md) and [B9d](./B9d-text-runs-quads-and-page-text.md) API; ADR 0016 (from [B6a](../ongoing/B6a-lamina-wasm-and-adr-0016.md)) for module split and size budgets.
- Web plan [C5](../../plans/web-app.md#c5-text-selection-and-search) acceptance (copy in five scripts, search counts vs pdf.js) — design the TS shape for it.
- [Rendering notes §3](../../research/architecture/notes/rendering.md#3-text-layer-selection-search-accessibility-hit-testing).
- [ADR 0018 §1](../../decisions/0018-test-strategy.md#1-tiers) tier 6 (differential), tier 9 (fuzz).

## Scope

**In scope**

- WASM: `WasmPage::text(options)` returning a struct-of-arrays (`Float32Array` quads, `Uint32Array` offsets, one UTF-16 string per run, run metadata) to minimise copies; tsify types for metadata.
- TS: `page.text(options?) → PageText { runs: TextRun[], text(): string }` with lazy decoding of quads; worker-friendly (transferable buffers).
- Differential: `cargo xtask differential --text` comparing per-page text with pdf.js (`getTextContent`, normalised) and pdftotext (`-layout` off) on `fixtures/corpus` + `fixtures/text`; precomputed expectations in `fixtures/differential/text.json` for the PR lane, live tools in the full/nightly lanes; exceptions file with reasons.
- Quad test: PDFium (`pypdfium2` `get_charbox`) vs lamina quads per glyph, tolerance in points (e.g. ≤ 1 pt or ≤ 10% of font size, documented), on fixtures where both map glyphs 1:1.
- Fuzz target `extract_text` (open + extract all pages), added to the nightly workflow's fuzz matrix (`.github/workflows/nightly.yml`, 10 min) and to `FUZZ_TARGETS`.
- Nightly external-corpus text differential: a `--text` option (or a `text` fact) for `cargo xtask differential --live`, with pdf.js and pdftotext runners in `tools/differential/` and its classes in `exceptions.toml`, run by the nightly workflow's differential job ([X3a](../done/X3a-nightly-lane.md) built the lane; this todo adds the text part).
- Size: measure the extraction code's WASM cost and record it against the ADR 0016 budgets.

**Out of scope** (and which todo covers it)

- Text layer DOM, selection, search UI: web C5.

## Tasks

- [ ] Load `/rust-best-practices`; read ADR 0016 and web plan C5
- [ ] `lamina-wasm` text bindings; `@lamina/pdf` `page.text()`; tests in Node and Chromium, using the Vitest browser/Playwright config from [B6c](./B6c-runtime-matrix.md)
- [ ] `tools/differential/` additions (pdf.js script, pdftotext) and `cargo xtask differential --text [--update]`
- [ ] `tools/render/charbox.py` (pypdfium2) and `cargo xtask quads`
- [ ] `fuzz/fuzz_targets/extract_text.rs`
- [ ] Size measurement; ADR 0016 note if the budget changes

## Acceptance criteria

- [ ] Extracted text for fixtures in five scripts matches expected text and pdf.js/pdftotext output on the corpus (exceptions listed with reasons)
- [ ] Glyph quads lie within the documented tolerance of PDFium's character boxes on the quad fixtures
- [ ] Encrypted fixtures extract the same text as their originals through TS
- [ ] `extract_text` fuzz target runs 10 min locally (`cargo xtask fuzz extract_text --time 600`) without a panic; summary in the PR
- [ ] TS `page.text()` output equals the Rust output (shared JSON snapshot in `fixtures/text/`)
- [ ] WASM size still within ADR 0016 budgets (or a reviewed budget change)

## Post-merge checks

- [ ] `extract_text` is green in the first scheduled fuzz run on `main` (link it in a follow-up PR or the done todo)

## Tests

- `packages/pdf/test/text.test.ts`; `crates/lamina-wasm/tests/text.rs`; differential and quad xtask suites (fast lane: precomputed; full lane: live tools).

## Docs to update

- [ ] [pdf-library §10 B9](../../plans/pdf-library.md#10-to-dos): tick "expose `page.text()` …" and "Extraction tests … quad tests … fuzz target"
- [ ] Docs site guide "Extracting text" (if [B7a](./B7a-docs-site.md) merged)
- [ ] Web plan C5 can now reference the API (parent updates web-app.md)

## Risks and open questions

- pdf.js text normalisation (whitespace insertion heuristics) differs from ours; compare normalised strings and document the rules.
- Struct-of-arrays API is less ergonomic; provide a convenience `runs` getter that materialises objects lazily.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
