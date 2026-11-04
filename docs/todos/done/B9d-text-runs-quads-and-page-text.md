# B9d — Text extraction: glyph quads, runs with `ActualText` and MCIDs, `page.text()`

| Field | Value |
| --- | --- |
| ID | `B9d` |
| Status | `done` |
| Plan | [pdf-library §7 B9](../../plans/pdf-library.md#b9-text-extraction), requirement [§4 F8](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M2 Web reader v0.1 (prerequisite) |
| Depends on | [B9b](./B9b-text-extraction-core.md) |
| Blocks | [B9c](./B9c-text-extraction-bindings-and-differential.md), [B9e](./B9e-separators-at-backward-jumps.md), [L3a](./L3a-lamina-extract-reading-order.md), [U2b](./U2b-reading-order-ground-truth.md) |
| ADRs | Implements 0023 (from [B9a](../done/B9a-lamina-content-and-adr-0023.md)); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (tolerance, warnings), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (façade) |
| Branch | `feat/b9d-text-runs-and-quads` |
| PR title | `feat(text): glyph quads, text runs with ActualText and MCIDs, page.text()` |
| Size | M–L (≈ 800–1,200 lines) |
| PR | [#76](https://github.com/laudebugs/lamina/pull/76) |

## Goal

Finish what [B9b](../done/B9b-text-extraction-core.md) started: B9b decodes every shown glyph to Unicode with its origin and advance. This todo gives each glyph a quad in page space, groups glyphs into runs, applies `ActualText` and attaches MCIDs, flags invisible text, and exposes the result as `page.text()` in the `lamina` façade. B9b was split here because the whole of it passed the ~1,500-line limit its risks section set.

## Context

- B9b's `lamina_text::extract` (`TextExtractor`, `PageText`, `Glyph`, the CMap and font readers) and its tests (`crates/lamina-text/tests/extract_fonts.rs`): left-to-right text of every text golden already equals what pypdf, pdf.js and pdftotext extract; right-to-left lines come out in visual order.
- [Research 03 §2.6](../../research/pdf/03-text-and-fonts.md#26-matrices-and-glyph-displacement-math) (text rendering matrix, quads), [§5.3](../../research/pdf/03-text-and-fonts.md#53-ligatures-reordering-and-actualtext) (`ActualText`), [§8.1](../../research/pdf/03-text-and-fonts.md#81-how-extractors-work).
- [Rendering notes §3](../../research/architecture/notes/rendering.md#3-text-layer-selection-search-accessibility-hit-testing) (what the text layer needs); web plan [C5](../../plans/web-app.md#c5-text-selection-and-search).

## Scope

**In scope**

- Geometry: for each glyph, a quad from the font's ascent and descent (descriptor, else font program, else a default) across its advance, through the text rendering matrix, in page space (ADR 0027: top-left of the displayed CropBox, `UserUnit` applied), with a helper to map to rotated view space; vertical writing with `/W2` position vectors.
  *As built:* ADR 0027's page space already is the displayed (rotated) page, so quads are in it and `PageText::frame` (`PageFrame::to_user_space`) maps back to user space; the extent comes from the descriptor's `/Ascent`/`/Descent`, else its `/FontBBox`, else the standard 14's AFM values, else 800/−200 (the font program is not read for it).
- Runs: consecutive glyphs with the same font, size, direction, MCID and `ActualText` span; `TextRun { text, glyphs, font_name, font_size, direction, mcid, actual_text, invisible }` (as built: `vertical` for the direction, plus `font` and `bounds`). `ActualText` from `BDC` property lists replaces the run's text and keeps its quads; `Tr 3` sets `invisible` (OCR layers).
- Right-to-left text in logical order: `ActualText` where the producer wrote it, else a bidi pass per line (UAX #9 through `BidiParagraph`), documented with its limits.
  *As built:* in two steps. Within a run with right-to-left characters, glyphs are read by position (zero-width glyphs, marks and textless ones, go with the letter they sit on, in canonical combining-class order) and UAX #9 puts grapheme clusters in logical order; a run of only neutral characters after a right-to-left run on its line is reversed too. Then each line with right-to-left runs orders its runs by UAX #9 at run level (runs by position; strong classes R, L, neutrals by N1/N2; levels; L2), so a line split into runs (fonts, punctuation) comes out right whether its runs were drawn in visual or logical order. A run drawn in logical order glyph by glyph would come out reversed; no producer we test does that.
- `Document::page_text(index, &TextOptions) -> Result<PageText>` in the `lamina` façade (an addition: the existing façade items keep their shape), with the font cache kept per document until the revision moves.

**Out of scope** (and which todo covers it)

- WASM/TS exposure, differential tests vs pdf.js/pdftotext, quad tests vs PDFium, 10-minute extraction fuzzing: [B9c](../done/B9c-text-extraction-bindings-and-differential.md).
- Layout analysis and reading order: [L3a](../done/L3a-lamina-extract-reading-order.md).
- Bundled CJK character-collection CMaps: decide in B9c (size vs the WASM budget).

## Tasks

- [x] Load `/rust-best-practices`; read 03 §2.6, §5.3, §8.1 and ADR 0027
- [x] Quads in `extract/mod.rs` (the glyph loop; the extent in `extract/font.rs`), `extract/runs.rs` (grouping, `ActualText`, MCIDs, `Tr 3`, bidi for runs without `ActualText`, lines)
- [x] Façade `Document::page_text` (feature `text`, `lamina::extract`) with the per-document font cache; warnings flow through `WarningCursor`
- [x] Encrypted fixtures: two text goldens through qpdf (`tools/fixtures/text_encrypted.py` → `fixtures/text/encrypted/`, R6 AES-256 and R3 RC4-128), not B2b's generator, whose rerun would rewrite every encrypted fixture (its outputs are not reproducible)
- [x] Tests below

## Acceptance criteria

- [x] Extracted text for the five-script fixtures (Latin, Arabic, Devanagari, Hebrew, CJK: `text-shaped.pdf`, `build-text-multiscript.pdf`) equals the drawn text in `fixtures/text/extracted.json` and `fixtures/build/facts.json`, right-to-left lines included (normalised whitespace rules documented: lines trimmed, empty ones dropped, a space where the pen jumps forward over a quarter of the size). The text goldens are compared with their samples' drawn lines, the builder goldens with each line from the first extractor not listed as getting it wrong
- [x] Encrypted fixtures extract the same text and quads as their plaintext originals
- [x] `ActualText` replaces run text where present; MCIDs are attached and match the structure tree of tagged fixtures (from [B8a](./B8a-structure-tree.md) if merged, else hand-made fixtures): B8a has not merged, so hand-made content (nested spans, innermost MCID, outermost `ActualText`, a span over two runs)
- [x] Unit tests for the quad computation match hand-computed values; rotated pages and `UserUnit` covered (and the `CropBox` origin, vertical writing)

## Post-merge checks

None.

## Tests

- `crates/lamina-text/tests/extract_fonts.rs` (every golden line, right to left included), `extract_runs.rs` (quads, runs, `ActualText`, MCIDs, invisible text; tagged capability A.10), `extract_encrypted.rs`; unit tests in `extract/runs.rs` and `extract/font.rs`; `crates/lamina/tests/page_text.rs`.

## Docs to update

- [x] [pdf-library §10 B9](../../plans/pdf-library.md#10-to-dos): tick the glyph runs item (Rust part)
- [x] `CLAUDE.md` (façade and fonts API), the `lamina` and `lamina-text` READMEs

## Risks and open questions

- Quads: font bbox vs ascent/descent — pdf.js and PDFium differ; pick PDFium's character boxes as the reference (B9c tolerances) and document.
- Bidi without `ActualText` reorders by heuristics, as pdf.js does; a line mixing directions in one run can still come out wrong. Document what is guaranteed.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
