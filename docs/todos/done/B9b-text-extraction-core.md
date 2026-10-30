# B9b — Text extraction in `lamina-text`: fonts from files, encodings, ToUnicode, glyph runs and quads

| Field | Value |
| --- | --- |
| ID | `B9b` |
| Status | `done` |
| Plan | [pdf-library §7 B9](../../plans/pdf-library.md#b9-text-extraction), requirement [§4 F8](../../plans/pdf-library.md#functional) |
| Round / milestone | Round B (library) — M2 Web reader v0.1 (prerequisite) |
| Depends on | [B2b](./B2b-standard-security-decryption.md), [B4a](./B4a-font-loading-and-subsetting.md), [B4c](./B4c-standard-14-and-simple-fonts.md), [B5c](./B5c-lamina-facade-and-examples.md), [B9a](./B9a-lamina-content-and-adr-0023.md) |
| Blocks | [B9d](../ready/B9d-text-runs-quads-and-page-text.md) |
| ADRs | Implements 0023 (from [B9a](../done/B9a-lamina-content-and-adr-0023.md)); follows [0003](../../decisions/0003-reader-tolerance-and-limits.md) (tolerance, warnings) |
| Branch | `feat/b9b-text-extraction` |
| PR title | `feat(text): text extraction with encodings, ToUnicode, glyph quads, ActualText and MCIDs` |
| Size | L (≈ 1,100–1,500 lines) |
| PR | — |

> **Split.** As the risks below allowed, this todo was split when the whole passed ~1,500 lines: B9b (this one) reads fonts, CMaps and ToUnicode and decodes every shown glyph to Unicode with its origin and advance; [B9d](../ready/B9d-text-runs-quads-and-page-text.md) adds quads, runs, `ActualText`, MCIDs, invisible text and the façade's `page.text()`. The scope, tasks and criteria below say which part moved.

## Goal

Turn a page's text-showing operators into Unicode text with geometry: per page, glyph runs with their text, font size, writing direction and a quad per glyph in page space, plus `ActualText` and MCIDs where present. This is what the readers' text layer, search and copy (web C5) are built on, and what `page.text()` returns in Rust.

## Context

- [Research 03 §3](../../research/pdf/03-text-and-fonts.md#3-font-types-iso-32000-2-9597) (font types, descriptors, `/Widths`), [§4](../../research/pdf/03-text-and-fonts.md#4-encodings-965-in-pdf-20--966-in-iso-32000-1-975-annex-d) (simple encodings and `/Differences`, CMaps, CID vs GID vs Unicode, `CIDToGIDMap`), [§5](../../research/pdf/03-text-and-fonts.md#5-tounicode-cmaps-and-actualtext-910) (ToUnicode, ActualText), [§2.6](../../research/pdf/03-text-and-fonts.md#26-matrices-and-glyph-displacement-math) (glyph displacement, quads), [§8.1](../../research/pdf/03-text-and-fonts.md#81-how-extractors-work).
- ISO 32000-2 §9.4.4 (text space → device), §9.6–9.10, Annex D; Adobe TN #5014 (CMap format), predefined CMaps (Adobe-Japan1 etc., licence check before bundling).
- [Rendering notes §3](../../research/architecture/notes/rendering.md#3-text-layer-selection-search-accessibility-hit-testing) (what the text layer needs).
- Web plan [C5](../../plans/web-app.md#c5-text-selection-and-search).

## Scope

**In scope**

- `lamina-text::extract` module (read side), separate from generation, sharing encoding tables/AGL with B4c.
- Font dictionary reading: Type1/MMType1/TrueType simple fonts (`/FirstChar`, `/Widths`, `/MissingWidth`, `/Encoding` name or dict with `/Differences`, built-in encodings of embedded fonts via skrifa where available), standard 14 by name (AFM widths from B4c), Type 0 (`/Encoding` Identity-H/V or embedded CMap stream, `/DescendantFonts` CIDFontType0/2, `/W`, `/DW`, `/W2`/`/DW2` vertical, `CIDToGIDMap`), Type 3 (`/FontMatrix`, widths; Unicode via ToUnicode/encoding only).
- CMap parser (codespace ranges, `cidchar`/`cidrange`/`notdefrange`, `usecmap` with depth limit) and ToUnicode parser (`bfchar`/`bfrange` incl. array form, UTF-16BE surrogates, multi-char mappings), both tolerant with warnings and limits.
- Unicode resolution order: ToUnicode → encoding + AGL (incl. `uniXXXX`, `uXXXX[XX]`, `gNN`-style heuristics documented) → embedded font cmap via skrifa → U+FFFD with a `unmapped-glyph` warning count.
- Geometry: for each glyph, text rendering matrix from `Tm`, CTM, font size, `Tz`, `Ts`, `Tc`/`Tw` (word spacing only for single-byte code 32), displacement from widths; quad = font bbox ascent/descent (from descriptor or font program) in page space (respecting `/Rotate` and `UserUnit` — page space before rotation, with a helper to map to rotated view space); vertical writing mode.
- Runs: consecutive glyphs with the same font/size/direction/MCID grouped; `TextRun { text, glyphs: [Glyph { unicode, quad, code, gid? }], font_name, font_size, direction, mcid, actual_text }`; `ActualText` from `BDC` property lists replaces the run text while keeping quads; invisible text (`Tr 3`) flagged (OCR layers).
- Simple reading-order heuristic (content order; optional line grouping by baseline) — no layout analysis.
- `Page::text(&self, TextOptions) -> Result<PageText>` in `lamina` façade; per-page decode budget via `Limits`.

**Out of scope** (and which todo covers it)

- WASM/TS exposure, differential tests vs pdf.js/pdftotext, quad tests vs PDFium, 10-min extraction fuzz run: [B9c](../ready/B9c-text-extraction-bindings-and-differential.md).
- Text layer UI, selection, search: web C5.
- Layout analysis, tables, OCR: full-editor E3/E13.
- Bundling predefined CJK CMaps beyond Identity: decide in risks (may fetch from Adobe's cmap-resources, BSD-3-Clause).

## Tasks

- [x] Load `/rust-best-practices`; read 03 §3–5, §2.6, §8.1
- [x] `extract/font.rs` (simple, Type 0 and Type 3 fonts, descriptors, Unicode resolution), `extract/cmap.rs` (CMaps and ToUnicode CMaps, one parser), `extract/mod.rs` (the `lamina_content::Visitor`); geometry and runs moved to [B9d](../ready/B9d-text-runs-quads-and-page-text.md)
- [x] Font cache per document (keyed by font object id) to avoid re-parsing
- [x] Predefined CMaps: Identity-H/V built in, and the `Uni…-UCS2-…`/`Uni…-UTF16-…` ones read as UTF-16; bundling Adobe's CJK character-collection CMaps is left to [B9c](../ready/B9c-text-extraction-bindings-and-differential.md) (WASM size budget); until then they raise `cmap-invalid`
- [x] Warnings and codes (`font-invalid`, `cmap-invalid`, `unmapped-glyph`); the façade API moved to [B9d](../ready/B9d-text-runs-quads-and-page-text.md)
- [x] Fixtures: the existing text goldens in five scripts (`text-*.pdf` from `lamina-text`, `build-text-*.pdf` from `lamina-build`) with what pypdf, pdf.js and pdftotext extract (`fixtures/text/extracted.json`, `fixtures/build/facts.json`; not lamina); encrypted copies moved to [B9d](../ready/B9d-text-runs-quads-and-page-text.md)
- [x] Tests below

## Acceptance criteria

- [x] Extracted text for the five-script fixtures equals the drawn text that pypdf, pdf.js and pdftotext extract (`fixtures/text/extracted.json`, `fixtures/build/facts.json`), line by line, for every left-to-right line (Latin, Devanagari, CJK); right-to-left lines (Arabic, Hebrew) come out in drawing order and move to [B9d](../ready/B9d-text-runs-quads-and-page-text.md) with `ActualText` and bidi. Line rules (`PageText::lines`): a new line where the baseline moves by more than half the size, a space where the pen jumps forward by more than a quarter of it
- [x] Unit tests for the text-rendering matrix match hand-computed values (research 03 §2.6's `TJ` example, `Tc`/`Tw`/`Tz`, CTM and `Tm` scaling); quads, rotated pages, `UserUnit`, encrypted fixtures, `ActualText` and MCIDs moved to [B9d](../ready/B9d-text-runs-quads-and-page-text.md)
- [x] Malformed CMaps/ToUnicode degrade with warnings, never panic; limits paired tests (`/UseCMap` chains at and past `MAX_USECMAP_DEPTH`)

## Post-merge checks

None.

## Tests

- Unit tests (CMap/ToUnicode parsing, AGL heuristics, matrices).
- `crates/lamina-text/tests/extract_fonts.rs` (goldens, matrices, warnings, strict mode); unit tests in `extract/cmap.rs` and `extract/font.rs`. `extract_scripts`, `extract_encrypted` and `extract_geometry` moved to [B9d](../ready/B9d-text-runs-quads-and-page-text.md).

## Docs to update

- [x] [pdf-library §10 B9](../../plans/pdf-library.md#10-to-dos): tick font dictionary reading, encodings/ToUnicode (glyph runs: B9d)
- [x] `fixtures/README.md` (which tests read the text facts), crate README

## Risks and open questions

- Quads: font bbox vs ascent/descent — pdf.js and PDFium differ; pick PDFium's character boxes as the reference (B9c tolerances) and document.
- CJK CMap bundling size vs WASM budget: may need lazy loading via the shell (like pdf.js's `cMapUrl`) — ADR 0023 or a follow-up decides.
- If the PR exceeds ~1,500 lines, split font/CMap parsing (B9b) from geometry/runs (B9d) and chain.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
