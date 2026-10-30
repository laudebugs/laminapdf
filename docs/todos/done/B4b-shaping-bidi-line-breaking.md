# B4b — Shaping (HarfRust), bidi and line breaking (ICU4X), with goldens in five scripts

| Field | Value |
| --- | --- |
| ID | `B4b` |
| Status | `done` |
| Plan | [pdf-library §7 B4](../../plans/pdf-library.md#b4-fonts-and-text), requirement [§4 F3](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M1 |
| Depends on | [B4a](./B4a-font-loading-and-subsetting.md), [X4](./X4-full-lane-gate.md) |
| Blocks | [B4d](../ongoing/B4d-tounicode-per-text-cids.md), [B5b](../ongoing/B5b-builder-text-and-document-features.md), [E4e](../ready/E4e-freetext-callouts.md), [E6c](../ready/E6c-add-text.md), [E13c](../ready/E13c-ocr-text-layer.md), [E16c](../ready/E16c-images-and-fonts.md) |
| ADRs | Follows [0005 §6](../../decisions/0005-document-model.md#6-generation-strategy--option-b-recommended-by-the-krilla-fit-spike-accepted-when-its-pr-merges) (HarfRust), [0001](../../decisions/0001-core-language-and-licence.md) (pure Rust, determinism), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (Unicode-3.0 data licences) |
| Branch | `feat/b4b-shaping` |
| PR title | `feat(text): HarfRust shaping, ICU4X bidi and line breaking, shaping goldens` |
| Size | L (≈ 800–1,200 lines + golden data) |
| PR | [#55](https://github.com/laudebugs/lamina/pull/55) |

## Goal

Turn a Unicode string into positioned glyph runs that render correctly in every script lamina claims: shaping with HarfRust behind a `Shaper` trait (with per-run OpenType features), bidi reordering (UAX #9) and line-break opportunities (UAX #14) from ICU4X, and greedy line breaking inside a box. Output keeps the cluster → text mapping that ToUnicode and `ActualText` need, so extraction from generated files returns the input string.

## Context

- [Research 03 §7](../../research/pdf/03-text-and-fonts.md#7-text-layout--shaping--everything-pdf-does-not-do-for-you): [§7.2 shaping](../../research/pdf/03-text-and-fonts.md#72-shaping), [§7.3 bidi](../../research/pdf/03-text-and-fonts.md#73-bidirectional-text-uax-9), [§7.4 line breaking](../../research/pdf/03-text-and-fonts.md#74-line-breaking-uax-14), [§7.6 justification](../../research/pdf/03-text-and-fonts.md#76-justification), [§7.7 fallback](../../research/pdf/03-text-and-fonts.md#77-font-fallback); [§5.3 ligatures, reordering and ActualText](../../research/pdf/03-text-and-fonts.md#53-ligatures-reordering-and-actualtext); [§2.6–2.7 displacement and TJ kerning](../../research/pdf/03-text-and-fonts.md#26-matrices-and-glyph-displacement-math).
- [pdf-library §8 risks](../../plans/pdf-library.md#8-risks): WASM size of shaping and ICU4X data (slice data).
- `lamina-text` from [B4a](../done/B4a-font-loading-and-subsetting.md) (`Font`, `FontUse`).

## Scope

**In scope**

- `Shaper` trait (`shape(&Font, &str, &ShapeOptions) -> ShapedRun`) with a HarfRust implementation; `ShapeOptions { script, language, direction, features: Vec<Feature> }` (e.g. `liga=0`, `smcp`, `tnum`), script/direction auto-detected per run.
- `ShapedRun { glyphs: [ShapedGlyph { gid, cluster, x_advance, y_advance, x_offset, y_offset }], text_range }`; conversion to PDF positioning (TJ adjustments in 1/1000 em, `Ts`/`Tm` for vertical offsets), registering (GID → cluster text) in `FontUse`, flagging clusters that need `ActualText` (reordered or many-to-many).
- Bidi via `icu_properties`/`unicode-bidi`-equivalent from ICU4X (`icu::properties::bidi`, or `unicode-bidi` if ICU4X's bidi isn't available on the pinned version — decide and record): paragraph level, runs in visual order.
- Line breaking via `icu_segmenter::LineSegmenter` (UAX #14) and grapheme clusters; `layout::break_lines(runs, width) -> Vec<Line>` greedy (no hyphenation, no justification beyond left/right/centre alignment; justification flag reserved).
- Itemisation: split text into runs by script, direction and font coverage; no fallback chain yet — a missing glyph is `missing-glyph` (B4a), with an optional explicit fallback list supplied by the caller.
- ICU4X data: compiled data with only the needed keys (`icu_segmenter` line + grapheme, bidi properties), sliced with `icu4x-datagen` or the `compiled_data` feature minimised; size measured.
- Shaping goldens for Latin (kerning, `fi` ligature), Arabic (joining, `lam-alef`), Devanagari (reph, conjuncts), Hebrew (marks, RTL) and CJK (vertical metrics not yet; horizontal only) compared with HarfBuzz `hb-shape` JSON output.

**Out of scope** (and which todo covers it)

- `surface.text()` / `textBox()` API: [B5b](../ongoing/B5b-builder-text-and-document-features.md).
- Hyphenation, justification, vertical writing, flowing layout across pages: later plan (pdf-library §2 non-goals).
- Font fallback chains and system fonts: never in core (caller supplies fonts); optional `@lamina/fonts` is an open question.

## Tasks

- [x] Load `/rust-best-practices`; read 03 §7 and §5.3
- [x] Add `harfrust`, `icu_segmenter`, `icu_properties` (or chosen bidi crate) to workspace deps; licence check (Unicode-3.0 allowed per `deny.toml`); wasm32 build. **Decided:** `harfrust` =0.8.0 (MIT; the last release on `read-fonts` 0.39, the one `skrifa` 0.42/`subsetter` 0.2.6 use, so the build keeps one copy; upgrade the three together), `icu_segmenter` and `icu_properties` =2.4.0 (Unicode-3.0, compiled data, no LSTM/dictionaries; `icu_segmenter`'s `unstable` feature for the Unicode 17 rules, hence the exact pins), and `unicode-bidi` 0.3.18 (MIT OR Apache-2.0) for the UAX #9 algorithm, fed ICU4X's `Bidi_Class` data (ICU4X has the data but no bidi algorithm). No new licence in `deny.toml`.
- [x] `shape/mod.rs` (`Shaper`, `ShapeOptions`, `Feature` parsing `"liga=0"`), `shape/harfrust.rs` (named `shape/engine.rs`: a module called `harfrust` shadows the crate)
- [x] `itemize.rs` (script runs, direction runs), `bidi.rs`, `linebreak.rs`, `layout.rs` (greedy box breaking, alignment). Deviation: `break_lines(&shaper, &fonts, text, &LayoutOptions)` instead of `break_lines(runs, width)`: lines are measured by shaping each candidate (so breaks re-shape, research 03 §7.4, and no line exceeds the box), which needs the shaper and fonts
- [x] `pdfpos.rs`: shaped run → `TJ` array + text map + `ActualText` flags; unit tests against hand-computed values (`tests/pdfpos.rs`). Changes B4a's `FontUse`/ToUnicode: a glyph used only with empty text maps to `<cid> <>` instead of being left out, so extractors don't read its CID as a character
- [x] `tools/shaping-goldens/`: script running `hb-shape --output-format=json` (pinned HarfBuzz) on each fixture string and font → `fixtures/shaping/<script>.json`; `cargo xtask golden --update` invokes it when HarfBuzz is present
- [x] Extraction tests: generate PDFs (test page writer from B4a) for the five scripts; check pdf.js, pypdf and pdftotext extraction equals the input (full lane; lamina-only checks in the fast lane). Deviations: `golden/text-shaped.pdf` with pypdf and pdf.js facts in `fixtures/text/extracted.json` (tests read committed facts, as in B4a, so they run in the fast lane; no lamina-only extraction yet: that is B9b); pdftotext not covered (not installed where the facts are written; add with the full-lane tools)
- [x] Measure the added WASM size (text module) and record it in the PR for [B6a](../ready/B6a-lamina-wasm-and-adr-0016.md)

## Acceptance criteria

- [x] Shaping output (GIDs, clusters, advances, offsets) for each of the five scripts matches HarfBuzz's `hb-shape` goldens exactly
- [x] Bidi: mixed English/Arabic/Hebrew paragraph produces runs in the visual order given by the UAX #9 reference test cases (subset of `BidiCharacterTest.txt`)
- [x] Line breaking passes a subset of `LineBreakTest.txt` and breaks a paragraph into lines no wider than the box
- [ ] Extracted text from generated files (pdf.js, pypdf, pdftotext) equals the input string for all five scripts, including ligatures (via ToUnicode or `ActualText`). **Not fully met:** Latin (ligatures included), CJK, the Arabic line and the mixed English/Hebrew/Arabic line extract exactly with pypdf; Latin (ligatures included) and CJK with pdf.js. The rest is listed with reasons in `fixtures/text/extraction-exceptions.json` (6 lines): neither pypdf nor pdf.js `getTextContent` honours marked-content `/ActualText` (reordered Devanagari clusters read in drawing order), both split right-to-left text at `TJ` offsets (pointed Hebrew), and pdf.js reads an empty ToUnicode entry as its code (Noto Sans Arabic's separate dot glyphs). Fixing these needs one CID per (glyph, text) (a `/CIDToGIDMap` stream instead of Identity) or extractor fixes: follow-up [B4d](../ongoing/B4d-tounicode-per-text-cids.md), which also adds pdftotext
- [x] OpenType features: `liga=0` disables the `fi` ligature (golden)
- [x] Results are byte-deterministic across runs and targets (wasm32 vs native in CI): integer arithmetic for positions, `BTreeMap`s, determinism tests; the crate builds for wasm32 in `cargo xtask wasm`/portability (a wasm32 test run is not part of CI yet)

Measured wasm32 size (`opt-level = "z"`, LTO, a cdylib calling the API; no `wasm-opt`): B4a's encode path alone 95 KB (40 KB gzip); with `HarfRust.shape` + `pdf_run` 604 KB (220 KB gzip); with `break_lines` (bidi, line breaking, graphemes) 728 KB (263 KB gzip). B4b adds about 632 KB (222 KB gzip), mostly HarfRust's shaping code and tables and ICU4X's compiled property and segmenter data.

## Post-merge checks

None.

## Tests

- Unit tests per module; UAX conformance subsets under `fixtures/unicode/` (Unicode licence noted in `fixtures/README.md`).
- `crates/lamina-text/tests/{shaping_goldens,bidi,linebreak,extraction}.rs`; goldens in `fixtures/shaping/`.
- External extraction checks in the full lane ([X4](../done/X4-full-lane-gate.md)).

## Docs to update

- [x] [pdf-library §10 B4](../../plans/pdf-library.md#10-to-dos): tick HarfRust shaping, ICU4X bidi and line breaking, shaping goldens + extraction tests
- [x] `fixtures/README.md`, crate README; record measured sizes in the PR for ADR 0016

## Risks and open questions

- HarfRust maturity vs rustybuzz: if a script's goldens differ from HarfBuzz, file upstream and mark the case in an exceptions list rather than switching shapers mid-PR.
- ICU4X data size in WASM can blow the budget; slice aggressively and measure; consider `unicode-linebreak` (smaller) if ICU4X can't fit, recording the trade-off.
- `hb-shape` must be pinned to a version matching HarfRust's target behaviour.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
