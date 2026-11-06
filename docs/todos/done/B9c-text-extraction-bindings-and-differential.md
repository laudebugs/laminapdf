# B9c — `page.text()` in WASM and TypeScript; differential and quad tests; extraction fuzzing

| Field | Value |
| --- | --- |
| ID | `B9c` |
| Status | `done` |
| Plan | [pdf-library §7 B9](../../plans/pdf-library.md#b9-text-extraction) |
| Round / milestone | Round B (library) — M2 Web reader v0.1 (prerequisite) |
| Depends on | [B6b](./B6b-lamina-pdf-package.md), [B6c](./B6c-runtime-matrix.md), [B9d](./B9d-text-runs-quads-and-page-text.md) |
| ADRs | Follows 0023, 0016 (module placement: extraction lives in the text module or base — decide by size), [0018](../../decisions/0018-test-strategy.md) tiers 6 and 9 |
| Branch | `feat/b9c-text-bindings` |
| PR title | `feat(pdf): page.text() in WASM and TypeScript with differential and quad tests` |
| Size | M–L (≈ 600–1,000 lines incl. test tooling) |
| PR | [#97](https://github.com/laudebugs/lamina/pull/97) |

## Goal

Expose text extraction everywhere the readers need it and prove it against independent tools: `page.text()` in `lamina-wasm` and `@lamina/pdf` returning glyph runs with quads in a compact transferable form, extracted text compared with pdf.js and pdftotext on the corpus, glyph quads compared with PDFium's character boxes, and an extraction fuzz target running 10 minutes clean. When this merges, web C5 can build its text layer.

## Context

- [B9b](../done/B9b-text-extraction-core.md) and [B9d](../done/B9d-text-runs-quads-and-page-text.md) API; ADR 0016 (from [B6a](../done/B6a-lamina-wasm-and-adr-0016.md)) for module split and size budgets.
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
- **Split off to [B9f](./B9f-live-text-differential.md)** (the PR-lane differential below meets the acceptance criteria): nightly external-corpus text differential: a `--text` option (or a `text` fact) for `cargo xtask differential --live`, with pdf.js and pdftotext runners in `tools/differential/` and its classes in `exceptions.toml`, run by the nightly workflow's differential job ([X3a](../done/X3a-nightly-lane.md) built the lane; this todo adds the text part).
- Size: measure the extraction code's WASM cost and record it against the ADR 0016 budgets.

**Out of scope** (and which todo covers it)

- Text layer DOM, selection, search UI: web C5.

## Tasks

- [x] Load `/rust-best-practices`; read ADR 0016 and web plan C5
- [x] `lamina-wasm` text bindings; `@lamina/pdf` `page.text()`; tests in Node and Chromium, using the Vitest browser/Playwright config from [B6c](../done/B6c-runtime-matrix.md)
- [x] `tools/differential/` additions (pdf.js script, pdftotext) and `cargo xtask differential --text [--update]`
- [x] `tools/render/charbox.py` (pypdfium2) and `cargo xtask quads`
- [x] `fuzz/fuzz_targets/extract_text.rs`
- [x] Size measurement; ADR 0016 note if the budget changes (owner, 2026-10-30: a third `extract` module, 526 KB, budget 535 KB; text module budget 960 → 1,020 KB)

## Acceptance criteria

- [x] Extracted text for fixtures in five scripts matches expected text and pdf.js/pdftotext output on the corpus (exceptions listed with reasons)
- [x] Glyph quads lie within the documented tolerance of PDFium's character boxes on the quad fixtures
- [x] Encrypted fixtures extract the same text as their originals through TS
- [x] `extract_text` fuzz target runs 10 min locally (`cargo xtask fuzz extract_text --time 600`) without a panic; summary in the PR
- [x] TS `page.text()` output equals the Rust output (shared JSON snapshot in `fixtures/text/`)
- [x] WASM size still within ADR 0016 budgets (or a reviewed budget change)

## Post-merge checks

- [ ] `extract_text` is green in the first scheduled fuzz run on `main` (link it in a follow-up PR or the done todo)

## Tests

- `packages/sdk/test/text.test.ts`; `crates/lamina-wasm/tests/text.rs`; differential and quad xtask suites (fast lane: precomputed; full lane: live tools).

## Docs to update

- [x] [pdf-library §10 B9](../../plans/pdf-library.md#10-to-dos): tick "expose `page.text()` …" and "Extraction tests … quad tests … fuzz target"
- [x] Docs site guide "Extracting text" (if [B7a](./B7a-docs-site.md) merged; it isn't: the `@lamina/pdf` README has an "Extracting text" section)
- [x] Web plan C5 can now reference the API (parent updates web-app.md)

## Risks and open questions

- pdf.js text normalisation (whitespace insertion heuristics) differs from ours; compare normalised strings and document the rules.
- Struct-of-arrays API is less ergonomic; provide a convenience `runs` getter that materialises objects lazily.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- **Module (owner decision, 2026-10-30):** a third WASM module, `extract` (base + extraction, 526,184 B gzip; budget 535,000), chosen over extraction in base (+143 KB for everyone) or only in the text module (1 MB for a text layer). The text module has extraction too (1,008,844 B; budget raised to 1,020,000). ADR 0016 §4 and §6 record it.
- **API:** `WasmPage.text(options)` (struct of arrays); `PageRef.text()` → `PageText` in `@lamina/pdf`; `init({ extract })`, `loadExtract()`; `extract-module-not-loaded`.
- **Rust = TypeScript:** `fixtures/text/page-text.json`, checked by both.
- **Differential:** pdf.js and pdftotext agree with lamina on every page of the corpus and text goldens but five, each with its reason (`fixtures/differential/text-exceptions.json`).
- **Quads:** 2,140 one-to-one glyphs within the documented rule against PDFium's loose boxes (`crates/lamina/tests/quads.rs`).
- **Fuzz:** `cargo xtask fuzz extract_text --time 600`: 154,463 runs in 601 s, coverage 10,106, no crash (macOS ARM64, nightly).

