# B9d — Text extraction: glyph quads, runs with `ActualText` and MCIDs, `page.text()`

| Field | Value |
| --- | --- |
| ID | `B9d` |
| Status | `ongoing` |
| Plan | [pdf-library §7 B9](../../plans/pdf-library.md#b9-text-extraction), requirement [§4 F8](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M2 Web reader v0.1 (prerequisite) |
| Depends on | [B9b](../done/B9b-text-extraction-core.md) |
| Blocks | [B9c](../ready/B9c-text-extraction-bindings-and-differential.md), [L3a](../ready/L3a-lamina-extract-reading-order.md), [U2b](../ready/U2b-reading-order-ground-truth.md) |
| ADRs | Implements 0023 (from [B9a](../done/B9a-lamina-content-and-adr-0023.md)); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (tolerance, warnings), [0027](../../decisions/0027-coordinates-units-and-page-indexing.md) (page space), [0028](../../decisions/0028-facade-rules-jobs-and-cancellation.md) (façade) |
| Branch | `feat/b9d-text-runs-and-quads` |
| PR title | `feat(text): glyph quads, text runs with ActualText and MCIDs, page.text()` |
| Size | M–L (≈ 800–1,200 lines) |
| PR | — |

## Goal

Finish what [B9b](../done/B9b-text-extraction-core.md) started: B9b decodes every shown glyph to Unicode with its origin and advance. This todo gives each glyph a quad in page space, groups glyphs into runs, applies `ActualText` and attaches MCIDs, flags invisible text, and exposes the result as `page.text()` in the `lamina` façade. B9b was split here because the whole of it passed the ~1,500-line limit its risks section set.

## Context

- B9b's `lamina_text::extract` (`TextExtractor`, `PageText`, `Glyph`, the CMap and font readers) and its tests (`crates/lamina-text/tests/extract_fonts.rs`): left-to-right text of every text golden already equals what pypdf, pdf.js and pdftotext extract; right-to-left lines come out in visual order.
- [Research 03 §2.6](../../research/pdf/03-text-and-fonts.md#26-matrices-and-glyph-displacement-math) (text rendering matrix, quads), [§5.3](../../research/pdf/03-text-and-fonts.md#53-ligatures-reordering-and-actualtext) (`ActualText`), [§8.1](../../research/pdf/03-text-and-fonts.md#81-how-extractors-work).
- [Rendering notes §3](../../research/architecture/notes/rendering.md#3-text-layer-selection-search-accessibility-hit-testing) (what the text layer needs); web plan [C5](../../plans/web-app.md#c5-text-selection-and-search).

## Scope

**In scope**

- Geometry: for each glyph, a quad from the font's ascent and descent (descriptor, else font program, else a default) across its advance, through the text rendering matrix, in page space (ADR 0027: top-left of the displayed CropBox, `UserUnit` applied), with a helper to map to rotated view space; vertical writing with `/W2` position vectors.
- Runs: consecutive glyphs with the same font, size, direction, MCID and `ActualText` span; `TextRun { text, glyphs, font_name, font_size, direction, mcid, actual_text, invisible }`. `ActualText` from `BDC` property lists replaces the run's text and keeps its quads; `Tr 3` sets `invisible` (OCR layers).
- Right-to-left text in logical order: `ActualText` where the producer wrote it, else a bidi pass per line (UAX #9 through `BidiParagraph`), documented with its limits.
- `Document::page_text(index, &TextOptions) -> Result<PageText>` in the `lamina` façade (an addition: the existing façade items keep their shape), with the font cache kept per document until the revision moves.

**Out of scope** (and which todo covers it)

- WASM/TS exposure, differential tests vs pdf.js/pdftotext, quad tests vs PDFium, 10-minute extraction fuzzing: [B9c](../ready/B9c-text-extraction-bindings-and-differential.md).
- Layout analysis and reading order: [L3a](../ready/L3a-lamina-extract-reading-order.md).
- Bundled CJK character-collection CMaps: decide in B9c (size vs the WASM budget).

## Tasks

- [ ] Load `/rust-best-practices`; read 03 §2.6, §5.3, §8.1 and ADR 0027
- [ ] `extract/geometry.rs` (quads, page space, rotated view), `extract/runs.rs` (grouping, `ActualText`, MCIDs, `Tr 3`), bidi for lines without `ActualText`
- [ ] Façade `Document::page_text` with the per-document font cache; warnings flow through `WarningCursor`
- [ ] Encrypted fixtures: the text goldens through [B2b](../done/B2b-standard-security-decryption.md)'s generator
- [ ] Tests below

## Acceptance criteria

- [ ] Extracted text for the five-script fixtures (Latin, Arabic, Devanagari, Hebrew, CJK: `text-shaped.pdf`, `build-text-multiscript.pdf`) equals the drawn text in `fixtures/text/extracted.json` and `fixtures/build/facts.json`, right-to-left lines included (normalised whitespace rules documented)
- [ ] Encrypted fixtures extract the same text and quads as their plaintext originals
- [ ] `ActualText` replaces run text where present; MCIDs are attached and match the structure tree of tagged fixtures (from [B8a](../ready/B8a-structure-tree.md) if merged, else hand-made fixtures)
- [ ] Unit tests for the quad computation match hand-computed values; rotated pages and `UserUnit` covered

## Post-merge checks

None.

## Tests

- `crates/lamina-text/tests/{extract_scripts,extract_encrypted,extract_geometry}.rs`; façade tests in `crates/lamina/tests/`.

## Docs to update

- [ ] [pdf-library §10 B9](../../plans/pdf-library.md#10-to-dos): tick the glyph runs item (Rust part)
- [ ] `CLAUDE.md` (façade and fonts API), the `lamina` and `lamina-text` READMEs

## Risks and open questions

- Quads: font bbox vs ascent/descent — pdf.js and PDFium differ; pick PDFium's character boxes as the reference (B9c tolerances) and document.
- Bidi without `ActualText` reorders by heuristics, as pdf.js does; a line mixing directions in one run can still come out wrong. Document what is guaranteed.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
